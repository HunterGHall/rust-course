# 2. Advanced generics & type-level design

## What & why

Generics are not just for containers. Used well, they let you push rules into the type system so that wrong programs *don't compile* — invalid states, mixed-up units, API calls in the wrong order — all at zero runtime cost. This lesson covers the techniques: bounds on associated types, the typestate pattern, typed IDs, associated consts, `const fn`, and the way the compiler handles generic code.

## Bounds on associated types

You can constrain not just `T` but `T::Something`:

```rust
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;

fn join_all<I>(items: I, sep: &str) -> String
where
    I: IntoIterator,
    I::Item: Display,
{
    let mut out = String::new();
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        out.push_str(&item.to_string());
    }
    out
}

fn tally<I>(items: I) -> HashMap<I::Item, usize>
where
    I: IntoIterator,
    I::Item: Eq + Hash,
{
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(item).or_insert(0) += 1;
    }
    counts
}

fn main() {
    println!("{}", join_all([1, 2, 3], ", "));
    println!("{}", join_all(vec!["a", "b"], "-"));
    println!("{}", join_all("xyz".chars(), ""));
    let t = tally("mississippi".chars());
    println!("{}", t[&'s']);                    // 4
}
```

Prefer `IntoIterator` over `Iterator` or `&[T]` for parameters: it accepts vectors, arrays, ranges, iterator chains, and references to collections.

## Typed identifiers

Mixing up two `u64` IDs is a classic bug. A phantom-typed wrapper makes it impossible:

```rust
use std::fmt;
use std::marker::PhantomData;

struct Id<T> {
    raw: u64,
    _marker: PhantomData<fn() -> T>,       // fn() -> T: Id<T> is Send/Sync/Copy regardless of T
}

// Manual impls so we don't require `T: Clone`/`Copy` (derive would add that bound):
impl<T> Clone for Id<T> { fn clone(&self) -> Self { *self } }
impl<T> Copy for Id<T> {}
impl<T> PartialEq for Id<T> { fn eq(&self, other: &Self) -> bool { self.raw == other.raw } }
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Id({})", self.raw) }
}

impl<T> Id<T> {
    fn new(raw: u64) -> Self { Id { raw, _marker: PhantomData } }
}

struct User;
struct Order;

fn find_user(id: Id<User>) -> String { format!("user {:?}", id) }

fn main() {
    let user: Id<User> = Id::new(7);
    let order: Id<Order> = Id::new(7);
    println!("{}", find_user(user));
    // find_user(order);            // error[E0308]: expected `Id<User>`, found `Id<Order>`
    println!("{:?}", order);
    println!("{}", std::mem::size_of::<Id<User>>());   // 8: the marker costs nothing
}
```

`PhantomData<fn() -> T>` is the idiom for "I logically relate to `T` but own none of it", and it keeps auto-traits (`Send`, `Sync`) and variance sensible (see lesson 3).

## The typestate pattern

Encode an object's *state* in its type, and make each transition consume the old state and return the new one. Calling a method in the wrong state becomes a compile error:

```rust
use std::marker::PhantomData;

struct Closed;
struct Open;

struct Door<State> {
    name: String,
    _state: PhantomData<State>,
}

impl Door<Closed> {
    fn new(name: &str) -> Door<Closed> {
        Door { name: name.to_string(), _state: PhantomData }
    }

    fn open(self) -> Door<Open> {           // consumes the closed door
        println!("{} opens", self.name);
        Door { name: self.name, _state: PhantomData }
    }
}

impl Door<Open> {
    fn walk_through(&self) {
        println!("walking through {}", self.name);
    }

    fn close(self) -> Door<Closed> {
        println!("{} closes", self.name);
        Door { name: self.name, _state: PhantomData }
    }
}

fn main() {
    let door = Door::new("front door");
    // door.walk_through();               // error[E0599]: no method named `walk_through` found for `Door<Closed>`
    let door = door.open();
    door.walk_through();
    let _door = door.close();
}
```

Ownership does the heavy lifting: `open(self)` consumes the old value, so you can't keep using the closed door afterwards.

### A typestate builder

The same idea guarantees required fields are set before `build()` exists:

```rust
use std::marker::PhantomData;

struct Unset;
struct Set;

struct RequestBuilder<Url, Method> {
    url: Option<String>,
    method: Option<String>,
    headers: Vec<(String, String)>,
    _state: PhantomData<(Url, Method)>,
}

impl RequestBuilder<Unset, Unset> {
    fn new() -> Self {
        RequestBuilder { url: None, method: None, headers: vec![], _state: PhantomData }
    }
}

impl<M> RequestBuilder<Unset, M> {
    fn url(self, url: &str) -> RequestBuilder<Set, M> {
        RequestBuilder { url: Some(url.into()), method: self.method, headers: self.headers, _state: PhantomData }
    }
}

impl<U> RequestBuilder<U, Unset> {
    fn method(self, method: &str) -> RequestBuilder<U, Set> {
        RequestBuilder { url: self.url, method: Some(method.into()), headers: self.headers, _state: PhantomData }
    }
}

impl<U, M> RequestBuilder<U, M> {
    fn header(mut self, k: &str, v: &str) -> Self {      // optional: available in every state
        self.headers.push((k.into(), v.into()));
        self
    }
}

impl RequestBuilder<Set, Set> {
    fn build(self) -> String {                           // only exists once both are set
        format!("{} {} ({} headers)", self.method.unwrap(), self.url.unwrap(), self.headers.len())
    }
}

fn main() {
    let request = RequestBuilder::new()
        .method("GET")
        .header("Accept", "text/plain")
        .url("https://example.com")
        .build();
    println!("{request}");
    // RequestBuilder::new().url("x").build();   // error: no method `build` for RequestBuilder<Set, Unset>
}
```

It's more code than a runtime check, so use it where a mistake is costly or the API is public and widely used. Plain builders returning `Result` at the end are often enough.

## Associated consts and `const fn`

Types can carry compile-time constants, and `const fn` runs at compile time:

```rust
trait Unit {
    const SYMBOL: &'static str;
    const TO_METERS: f64;
}

struct Kilometer;
struct Mile;
impl Unit for Kilometer { const SYMBOL: &'static str = "km"; const TO_METERS: f64 = 1000.0; }
impl Unit for Mile { const SYMBOL: &'static str = "mi"; const TO_METERS: f64 = 1609.344; }

fn describe<U: Unit>(amount: f64) -> String {
    format!("{amount} {} = {:.1} m", U::SYMBOL, amount * U::TO_METERS)
}

const fn factorial(n: u64) -> u64 {
    if n == 0 { 1 } else { n * factorial(n - 1) }
}

const FACT_10: u64 = factorial(10);          // computed by the compiler
static LOOKUP: [u8; 4] = {                   // a const block can build tables
    let mut table = [0u8; 4];
    let mut i = 0;
    while i < table.len() {
        table[i] = (i * i) as u8;
        i += 1;
    }
    table
};

fn main() {
    println!("{}", describe::<Mile>(26.2));
    println!("{FACT_10} {LOOKUP:?}");
}
```

Const generics combine with this: a fixed-capacity buffer whose size is part of the type.

```rust
struct RingBuffer<T: Copy + Default, const N: usize> {
    data: [T; N],
    start: usize,
    len: usize,
}

impl<T: Copy + Default, const N: usize> RingBuffer<T, N> {
    fn new() -> Self {
        RingBuffer { data: [T::default(); N], start: 0, len: 0 }
    }

    fn push(&mut self, value: T) {
        let index = (self.start + self.len) % N;
        if self.len < N {
            self.len += 1;
        } else {
            self.start = (self.start + 1) % N;      // overwrite the oldest
        }
        self.data[index] = value;
    }

    fn iter(&self) -> impl Iterator<Item = T> + '_ {
        (0..self.len).map(move |i| self.data[(self.start + i) % N])
    }
}

fn main() {
    let mut ring = RingBuffer::<i32, 3>::new();
    for n in 1..=5 {
        ring.push(n);
    }
    println!("{:?}", ring.iter().collect::<Vec<_>>());   // [3, 4, 5]
    println!("{}", std::mem::size_of::<RingBuffer<i32, 3>>());   // no heap, size known statically
}
```

## `impl Trait` and which generics it captures

`-> impl Trait` hides the concrete return type. In edition 2024 an `impl Trait` return type automatically captures **all** in-scope generic parameters and lifetimes, which is usually what you want; use `use<..>` to narrow:

```rust
fn words<'a>(text: &'a str, min: usize) -> impl Iterator<Item = &'a str> {
    text.split_whitespace().filter(move |w| w.len() >= min)
}

fn counter(start: u32) -> impl FnMut() -> u32 {
    let mut n = start;
    move || {
        n += 1;
        n
    }
}

fn main() {
    println!("{:?}", words("a bb ccc dddd", 3).collect::<Vec<_>>());
    let mut next = counter(10);
    println!("{} {}", next(), next());
}
```

Use `impl Trait` in argument position for simple cases, but prefer an explicit `<T: Trait>` when callers might need turbofish (`f::<u8>(...)`): a parameter declared as `impl Trait` has no name, so it can't be specified explicitly.

## Specialization doesn't exist (yet)

You cannot write one blanket implementation and a more specific override for `String`. Rust requires implementations not to overlap. The usual workarounds:

- an extra trait method that types can override,
- wrapper (newtype) types that select the behaviour,
- enums for closed sets,
- or a dedicated trait with separate implementations.

```rust
trait Summary {
    fn summarize(&self) -> String;
}

impl Summary for String {
    fn summarize(&self) -> String { format!("String of {} bytes", self.len()) }
}

impl<T: std::fmt::Debug> Summary for Vec<T> {
    fn summarize(&self) -> String { format!("Vec: {:?}", self) }
}

// `impl<T: Display> Summary for T` would now conflict with the two above — rejected at compile time.

fn main() {
    println!("{}", String::from("abc").summarize());
    println!("{}", vec![1, 2].summarize());
}
```

## Generic code and compile times

Every distinct instantiation (`Vec<i32>`, `Vec<String>`, ...) is compiled separately. That's the source of both Rust's speed and its long builds. Keep generic functions small, and move non-generic work into an inner function:

```rust
use std::path::Path;

pub fn read_config<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    fn inner(path: &Path) -> std::io::Result<String> {     // compiled once, not once per P
        std::fs::read_to_string(path)
    }
    inner(path.as_ref())
}

fn main() {
    println!("{}", read_config("Cargo.toml").is_ok());
}
```

(`std` itself uses this trick throughout.)

## Higher-order generic code: functions generic over closures

```rust
fn retry<T, E, F>(mut attempts: u32, mut op: F) -> Result<T, E>
where
    F: FnMut(u32) -> Result<T, E>,
{
    let mut n = 0;
    loop {
        n += 1;
        match op(n) {
            Ok(value) => return Ok(value),
            Err(e) if attempts <= 1 => return Err(e),
            Err(_) => attempts -= 1,
        }
    }
}

fn main() {
    let result = retry(5, |attempt| {
        if attempt < 3 { Err(format!("attempt {attempt} failed")) } else { Ok(attempt * 10) }
    });
    println!("{result:?}");           // Ok(30)

    let hopeless: Result<(), String> = retry(2, |n| Err(format!("nope {n}")));
    println!("{hopeless:?}");         // Err("nope 2")
}
```

## Common mistakes

- **Using `#[derive(Clone, Copy)]` on `Id<T>`/`Door<State>`** wrappers — the derive adds `T: Clone`, which your marker types don't satisfy. Implement manually, or use `PhantomData<fn() -> T>` with manual impls.
- **Over-engineering with typestate.** If the state is cheap to check at run time and the API is private, a `Result` is simpler.
- **Forgetting that marker types need no data** — use empty `struct Open;`, not an enum.
- **Hitting "conflicting implementations"** and trying to force specialization; restructure instead.
- **Bloating compile times** with deeply generic code in hot-to-compile crates.
- **Putting bounds on struct definitions** (`struct Foo<T: Display>`); put them on `impl` blocks instead so the type stays flexible.

## Exercises

1. Make `Id<T>` hashable (`Hash`) and use it as a `HashMap` key without requiring `T: Hash`.
2. Add a `lock` / `unlock` typestate to a `Safe` type where `read()` only exists when unlocked.
3. Extend the request builder so `body()` is only available when the method is `Post` (a third type parameter).
4. Write `fn largest_by<I, K, F>(items: I, key: F) -> Option<I::Item> where I: IntoIterator, K: PartialOrd, F: Fn(&I::Item) -> K`.
5. Write a `Matrix<const R: usize, const C: usize>` with `fn mul<const K: usize>(&self, other: &Matrix<C, K>) -> Matrix<R, K>`; check that a 2×3 times 2×3 does not compile.
6. Compute a table of squares for 0..16 at compile time with `const fn` and a `static`.
7. Rewrite a generic function taking `P: AsRef<Path>` so that the heavy part is a private non-generic inner function.

## Recap

- Bound associated types (`I::Item: Display`) and take `IntoIterator` for flexible APIs.
- `PhantomData` gives a type a compile-time tag at zero cost: typed IDs, units, typestate.
- Typestate + ownership (consuming `self`) turns misuse into compile errors; use it where mistakes are costly.
- Associated consts, `const fn` and const generics move work and sizes to compile time.
- No specialization: avoid overlap; keep generic functions small to protect compile times.

Next: [Advanced lifetimes, variance & Pin →](03-advanced-lifetimes.md)
