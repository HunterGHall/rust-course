# 3. Advanced lifetimes, variance & Pin

## What & why

Intermediate lesson 5 taught lifetimes as a tool for functions and structs. This lesson goes underneath: how lifetimes relate to one another (**subtyping and variance**), how to say "for *every* lifetime" (**higher-ranked bounds**), what `'static` really means in a bound, how trait objects carry lifetimes, and why some structures — self-referential ones, futures — need **`Pin`**.

You won't use all of this daily, but it explains compiler errors that otherwise seem arbitrary, and it is essential for writing generic library code and async code.

## Lifetime bounds and outlives

`'a: 'b` reads "`'a` outlives `'b`" — the region `'a` completely contains `'b`:

```rust
fn pick_first<'long, 'short>(a: &'long str, _b: &'short str) -> &'long str {
    a                       // output tied only to 'long
}

fn longer_of<'a, 'b: 'a>(a: &'a str, b: &'b str) -> &'a str {
    // 'b: 'a means b lives at least as long as a, so a &'b str can be used as a &'a str
    if a.len() > b.len() { a } else { b }
}

fn main() {
    let outer = String::from("outer");
    {
        let inner = String::from("in");
        println!("{}", longer_of(&inner, &outer));
        println!("{}", pick_first(&outer, &inner));
    }
}
```

The same notation bounds types: `T: 'a` means "all references inside `T` live at least as long as `'a`".

```rust
struct Ref<'a, T: 'a> {       // (the `T: 'a` is inferred nowadays; shown for clarity)
    value: &'a T,
}

fn main() {
    let n = 5;
    let r = Ref { value: &n };
    println!("{}", r.value);
}
```

## Subtyping: longer lifetimes are subtypes of shorter ones

A reference valid for a long time can safely be used where a shorter-lived one is expected. The compiler applies this automatically ("lifetime shortening"):

```rust
fn take_short<'a>(_x: &'a str) {}

fn main() {
    let long_lived: &'static str = "forever";
    take_short(long_lived);       // &'static str coerces to &'a str for any 'a
}
```

## Variance

Variance describes how subtyping of a *parameter* lifts to subtyping of the *whole type*. For a type `F<'a>`:

- **Covariant**: `F<'long>` can be used where `F<'short>` is expected. Shared references `&'a T`, `Box<T>`, `Vec<T>`, `Option<T>`... are covariant. This is why lifetimes shorten silently.
- **Invariant**: no substitution at all. `&'a mut T` is invariant in `T`; `Cell<T>`, `RefCell<T>` and `*mut T` are invariant in `T`.
- **Contravariant**: the reverse. Only appears with function arguments: `fn(&'a T)`.

Why is `&mut T` invariant in `T`? Otherwise this unsound program would be accepted:

<!-- check: fail -->
```rust
fn overwrite<'a>(slot: &mut &'a str, value: &'a str) {
    *slot = value;
}

fn main() {
    let mut forever: &'static str = "static";
    {
        let temporary = String::from("short-lived");
        overwrite(&mut forever, &temporary);   // error[E0597]: `temporary` does not live long enough
    }
    println!("{forever}");                     // would be a dangling pointer
}
```

If `&mut &'static str` were allowed to shrink to `&mut &'short str`, a short-lived string could be written into a slot the rest of the program believes is `'static`. Invariance forbids it.

You choose a struct's variance through its fields; `PhantomData<T>` acts as if the struct owned a `T`, while `PhantomData<fn() -> T>` is covariant and `PhantomData<fn(T)>` contravariant. The `Id<T>` in lesson 2 used `PhantomData<fn() -> T>` for that reason.

## Higher-ranked trait bounds (`for<'a>`)

Sometimes a bound must hold for *every* lifetime, not one chosen by the caller:

```rust
// F must accept a &str of ANY lifetime and return one of the same lifetime.
fn apply_to_parts<F>(text: &str, f: F) -> Vec<String>
where
    F: for<'a> Fn(&'a str) -> &'a str,
{
    text.split(',').map(|part| f(part).to_string()).collect()
}

fn main() {
    println!("{:?}", apply_to_parts("  a , b ,c ", |s| s.trim()));
}
```

Why isn't `fn apply<'a, F: Fn(&'a str) -> &'a str>` right? There, the caller picks one `'a`, and `f` is only usable with that particular lifetime, but `apply_to_parts` calls `f` with many different short borrows. `for<'a>` ("for all `'a`") is called a **higher-ranked trait bound** (HRTB).

You rarely write it: for `Fn(&str) -> &str` the compiler inserts `for<'a>` by elision. You meet it in error messages, in trait-object types like `Box<dyn for<'a> Fn(&'a str) -> &'a str>`, and in parsers/visitors that hand out borrows:

```rust
struct Parser {
    rules: Vec<Box<dyn Fn(&str) -> Option<&str>>>,    // elided: for<'a> Fn(&'a str) -> Option<&'a str>
}

fn main() {
    let parser = Parser {
        rules: vec![
            Box::new(|s| s.strip_prefix("let ")),
            Box::new(|s| s.strip_prefix("fn ")),
        ],
    };
    for line in ["let x", "fn main", "mod y"] {
        let hit = parser.rules.iter().find_map(|rule| rule(line));
        println!("{line:?} -> {hit:?}");
    }
}
```

## `'static` bounds, correctly understood

There are two different things named `'static`:

- `&'static T` — a reference that is valid for the whole program (literals, leaked data).
- `T: 'static` — the *type* contains no non-static borrows. **Owned data satisfies it.**

```rust
use std::fmt::Debug;

fn needs_static<T: Debug + 'static>(value: T) -> Box<dyn Debug> {
    Box::new(value)             // the Box<dyn Debug> carries a 'static bound by default
}

fn main() {
    let owned = String::from("owned");
    println!("{:?}", needs_static(owned));           // fine: String contains no borrows
    println!("{:?}", needs_static(42));
    println!("{:?}", needs_static("literal"));       // &'static str is 'static

    let local = String::from("local");
    let borrowed: &str = &local;
    // needs_static(borrowed);    // error: `local` does not live long enough — the &str isn't 'static
    println!("{borrowed}");
}
```

`std::thread::spawn` requires `F: 'static` for exactly this reason: the new thread may outlive the function that created it, so the closure can't borrow locals — it must `move` owned data in (scoped threads, Advanced 5, relax this).

## Lifetimes in trait objects

`Box<dyn Trait>` has a hidden lifetime bound. In a type like `Box<dyn Trait>` appearing in a struct field or as a return type, it defaults to `'static`; behind a reference, `&'a dyn Trait` defaults to `'a`. To store a trait object that borrows, say so:

```rust
trait Greeter {
    fn greet(&self) -> String;
}

struct Named<'a>(&'a str);
impl Greeter for Named<'_> {
    fn greet(&self) -> String { format!("hello {}", self.0) }
}

struct Registry<'a> {
    greeters: Vec<Box<dyn Greeter + 'a>>,      // may borrow data that lives for 'a
}

fn main() {
    let name = String::from("Ada");
    let mut registry = Registry { greeters: Vec::new() };
    registry.greeters.push(Box::new(Named(&name)));
    for g in &registry.greeters {
        println!("{}", g.greet());
    }
}
```

Without `+ 'a`, the field would require `'static` and `Named(&name)` would be rejected.

## The anonymous lifetime and `impl` headers

```rust
use std::fmt;

struct Wrapper<'a>(&'a str);

impl fmt::Display for Wrapper<'_> {                  // '_ : "some lifetime, I don't need to name it"
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.0)
    }
}

fn first_word(s: &str) -> &'_ str {                  // '_ makes elided lifetimes visible
    s.split(' ').next().unwrap_or("")
}

fn main() {
    println!("{} {}", Wrapper("x"), first_word("hello world"));
}
```

## Self-referential structs and why they're hard

A struct that holds a reference into *its own* data can't be expressed with ordinary lifetimes, because moving the struct would invalidate the pointer:

<!-- check: fail -->
```rust
struct SelfRef<'a> {
    text: String,
    first: &'a str,            // wants to point into `text`
}

fn main() {
    let text = String::from("hello world");
    let s = SelfRef { text, first: &text[..5] };   // error[E0382]: borrow of moved value: `text`
    println!("{}", s.first);
}
```

Workarounds, in order of preference:

1. Store **indexes/ranges** instead of references: `first: Range<usize>`.
2. Store the owner and the borrower in separate variables.
3. Use `Rc`/`Arc` to share ownership.
4. Use a crate like `ouroboros`, or `Pin` plus `unsafe`, for the rare cases where you genuinely need it.

```rust
use std::ops::Range;

struct Document {
    text: String,
    title: Range<usize>,         // an index "reference" survives moves
}

impl Document {
    fn title(&self) -> &str {
        &self.text[self.title.clone()]
    }
}

fn main() {
    let doc = Document { text: "Rust Guide: chapter 1".into(), title: 0..10 };
    let moved = doc;                      // moving is fine
    println!("{}", moved.title());
}
```

## `Pin`: preventing moves

In Rust every value can be moved by memcpy-ing its bytes to a new address. Most types don't care. Types that contain pointers into themselves — most notably **`async` blocks compiled into state machines** — would break if moved after they've started running.

`Pin<P>` wraps a pointer type `P` and promises that the value it points to **will not move** until it is dropped. Most types implement the auto-trait **`Unpin`** ("moving me is always fine"), for which `Pin` is a no-op. Self-referential futures are `!Unpin`.

```rust
use std::marker::PhantomPinned;
use std::pin::Pin;

struct Immovable {
    data: String,
    self_ptr: *const String,         // will point at `data`
    _pin: PhantomPinned,             // makes this type !Unpin
}

impl Immovable {
    fn new(data: &str) -> Pin<Box<Immovable>> {
        let boxed = Box::new(Immovable {
            data: data.to_string(),
            self_ptr: std::ptr::null(),
            _pin: PhantomPinned,
        });
        let mut pinned = Box::into_pin(boxed);          // now it can never move
        let ptr: *const String = &pinned.data;
        // SAFETY: we only set a field, and never move the value out of the pin.
        unsafe { pinned.as_mut().get_unchecked_mut().self_ptr = ptr };
        pinned
    }

    fn data_via_ptr(self: Pin<&Self>) -> &str {
        // SAFETY: self_ptr was set to point at self.data and the value is pinned (never moves).
        unsafe { &*self.self_ptr }
    }
}

fn main() {
    let value = Immovable::new("pinned");
    println!("{}", value.as_ref().data_via_ptr());
    println!("{}", value.data);
}
```

Day to day you meet `Pin` through async code — `Pin<Box<dyn Future<Output = T>>>` is the type of a boxed future, and `std::pin::pin!` pins a value on the stack:

```rust
use std::future::Future;
use std::pin::{pin, Pin};

fn boxed_future() -> Pin<Box<dyn Future<Output = u32>>> {
    Box::pin(async { 40 + 2 })
}

fn main() {
    let _a = boxed_future();
    let b = pin!(async { "on the stack" });
    let _ = b;                    // a Pin<&mut impl Future>
}
```

Advanced lesson 6 builds a tiny executor that polls pinned futures. For now the rule is: *you will use `Pin` as a type in signatures far more often than you will implement anything with it.*

## Common mistakes

- **Confusing `T: 'static` with `&'static T`.** The former includes any owned type.
- **Adding `'static` bounds** to make the compiler quiet, then being unable to pass borrowed data.
- **Expecting `&mut T` to behave covariantly** — it's invariant in `T`, which is why mutable borrows of containers of references are rigid.
- **Writing HRTBs by hand** when elision would do.
- **Forgetting `+ 'a`** on `Box<dyn Trait>` fields that borrow.
- **Trying self-referential structs** instead of indices.
- **Moving a pinned value** — the API prevents safe code from doing it; unsafe code that does is unsound.

## Exercises

1. Write `fn longest_with_note<'a, 'b>(x: &'a str, y: &'a str, note: &'b str) -> &'a str`; explain why `note` needs a separate lifetime.
2. Change `Registry<'a>` above so `greet_all(&self) -> Vec<String>` is a method, and verify it compiles with a short-lived name but not when `Registry` outlives `name`.
3. Write `fn map_all<F>(items: &[String], f: F) -> Vec<&str> where F: for<'a> Fn(&'a str) -> &'a str`.
4. Demonstrate covariance: pass a `&'static str` where a shorter `&'a str` is expected, and show that `&mut &'static str` can't be shortened.
5. Replace a self-referential design (a parser that stores the input and a `&str` slice into it) with an index-based version.
6. Create an `Unpin` struct and a `!Unpin` struct (with `PhantomPinned`); try to call `Pin::new(&mut x)` for each and observe the error for the second.
7. Write a function returning `Pin<Box<dyn Future<Output = String>>>` that formats its argument.

## Recap

- `'a: 'b` means "outlives". Longer lifetimes are subtypes of shorter ones, so shared references shrink freely.
- Variance: `&T` covariant; `&mut T`, `Cell<T>` invariant in `T`; function arguments contravariant. Invariance is what keeps mutation sound.
- `for<'a>` (HRTB) says "for every lifetime"; elision usually writes it for you.
- `T: 'static` means "no short-lived borrows inside" — owned data qualifies. Trait objects have default lifetime bounds you can override with `+ 'a`.
- Self-referential data is awkward; use indices. `Pin` guarantees a value stays put and is the foundation of async.

Next: [Macros →](04-macros.md)
