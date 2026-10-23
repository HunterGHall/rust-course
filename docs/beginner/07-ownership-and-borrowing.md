# 7. Ownership, borrowing & slices

## What & why

This is the lesson that makes Rust *Rust*. Every language has to decide who is responsible for freeing memory:

- C and C++: the programmer, by hand — and mistakes become crashes and security holes.
- Java, Python, Go, Swift: a garbage collector or reference counting, at run time.
- **Rust: the compiler, using three rules, at compile time — with zero runtime cost.**

It feels strict for a week and then feels like a superpower. Take it slowly; type every example.

## The three rules

1. Every value has exactly one **owner** (a variable).
2. There is only one owner at a time.
3. When the owner goes out of scope, the value is **dropped** (its memory freed).

```rust
fn main() {
    {
        let s = String::from("hello");   // s owns the heap memory
        println!("{s}");
    }                                    // s goes out of scope: memory freed here, automatically
}
```

No `free()`, no garbage collector. The compiler inserts the cleanup exactly where the owner dies.

## Move

Assigning an owned value to another variable **moves** ownership:

<!-- check: fail -->
```rust
fn main() {
    let a = String::from("hello");
    let b = a;                 // ownership moves from a to b
    println!("{a}");           // error[E0382]: borrow of moved value: `a`
}
```

Why? If both `a` and `b` owned the string, both would try to free it at the end of the scope — a double free. Moving makes `a` invalid instead. The error message explains exactly this:

```
error[E0382]: borrow of moved value: `a`
 |     let a = String::from("hello");
 |         - move occurs because `a` has type `String`, which does not implement the `Copy` trait
 |     let b = a;
 |             - value moved here
 |     println!("{a}");
 |               ^^^ value borrowed here after move
```

If you really want two independent copies, ask for it explicitly with `clone`:

```rust
fn main() {
    let a = String::from("hello");
    let b = a.clone();         // a deep copy: a second heap allocation
    println!("{a} {b}");
}
```

## `Copy` types

Simple fixed-size values (integers, floats, `bool`, `char`, and tuples/arrays of those) are cheap to duplicate, so assigning them **copies** and the original stays valid:

```rust
fn main() {
    let x = 5;
    let y = x;
    println!("{x} {y}");       // both fine — i32 is Copy
}
```

Anything that owns heap memory (`String`, `Vec`, `Box`) is *not* `Copy`; it moves.

## Ownership and functions

Passing a value into a function moves it, too:

<!-- check: fail -->
```rust
fn consume(s: String) {
    println!("got {s}");
}                              // s dropped here

fn main() {
    let name = String::from("Ada");
    consume(name);
    println!("{name}");        // error[E0382]: borrow of moved value: `name`
}
```

You could return the value to hand it back, but that's clumsy. The solution is borrowing.

## Borrowing with references

A **reference** (`&T`) lets you use a value without taking ownership. Creating one is called *borrowing*.

```rust
fn length(s: &String) -> usize {
    s.len()
}                              // s is just a borrow: nothing is dropped

fn main() {
    let name = String::from("Ada");
    let n = length(&name);     // lend name to the function
    println!("{name} has {n} bytes");   // name is still ours
}
```

A shared reference is **read-only**. To change something through a reference you need a **mutable reference**, `&mut T`:

```rust
fn shout(s: &mut String) {
    s.push_str("!!!");
}

fn main() {
    let mut name = String::from("Ada");   // the variable must be `mut` too
    shout(&mut name);
    println!("{name}");                   // Ada!!!
}
```

## The borrowing rules

At any moment you may have **either**:

- any number of shared references (`&T`), **or**
- exactly one mutable reference (`&mut T`),

and references must never outlive the value they point to.

This is the "readers–writer lock" rule enforced at compile time. It is precisely what makes data races, iterator invalidation and use-after-free impossible.

<!-- check: fail -->
```rust
fn main() {
    let mut s = String::from("hello");
    let first = &s;            // shared borrow
    let second = &mut s;       // error[E0502]: cannot borrow `s` as mutable
    second.push('!');          //   because it is also borrowed as immutable
    println!("{first}");
}
```

The scope of a borrow lasts only until its **last use** (this is called non-lexical lifetimes), so the following is fine:

```rust
fn main() {
    let mut s = String::from("hello");

    let r1 = &s;
    let r2 = &s;
    println!("{r1} {r2}");     // last use of r1 and r2

    let r3 = &mut s;           // OK: the shared borrows are finished
    r3.push_str(" world");
    println!("{r3}");
}
```

### Dangling references

Rust will not let a reference outlive what it points to:

<!-- check: fail -->
```rust
fn dangle() -> &String {       // error[E0106]: missing lifetime specifier
    let s = String::from("hi");
    &s                         // s is dropped at the end of the function!
}

fn main() {
    let r = dangle();
    println!("{r}");
}
```

The fix is to return the owned value (`-> String`, `s`), which moves it out to the caller. In C this bug compiles and crashes; in Rust it doesn't compile.

## Slices

A **slice** is a reference to a contiguous part of a collection. It borrows; it doesn't own.

```rust
fn first_word(text: &str) -> &str {
    match text.find(' ') {
        Some(end) => &text[..end],
        None => text,
    }
}

fn main() {
    let sentence = String::from("hello brave new world");
    let word = first_word(&sentence);
    println!("{word}");        // hello

    let numbers = [10, 20, 30, 40, 50];
    let middle: &[i32] = &numbers[1..4];
    println!("{middle:?}");    // [20, 30, 40]
}
```

`&str` is a string slice; `&[T]` is a slice of anything. The borrow checker keeps `word` honest — if you try to modify `sentence` while `word` is alive, it won't compile:

<!-- check: fail -->
```rust
fn first_word(text: &str) -> &str {
    text.split(' ').next().unwrap_or("")
}

fn main() {
    let mut sentence = String::from("hello world");
    let word = first_word(&sentence);
    sentence.clear();              // error[E0502]: cannot borrow `sentence` as mutable
    println!("{word}");            //   because it is also borrowed as immutable
}
```

**Parameter rule of thumb:** accept `&str` instead of `&String`, and `&[T]` instead of `&Vec<T>`. Callers can pass either, and your function works with more types.

## Choosing: owned, `&`, or `&mut`?

| Your function needs to... | Take | Example |
| --- | --- | --- |
| just read the data | `&T` | `fn len(s: &str)` |
| modify the caller's data | `&mut T` | `fn push_twice(v: &mut Vec<i32>)` |
| keep it or consume it | `T` (owned) | `fn into_title(s: String)` |

Start with `&`. Go to `&mut` or owned only when you must.

## Common mistakes

- **Using a variable after moving it.** Borrow it instead, or `.clone()` if you truly need a second copy.
- **Reaching for `.clone()` everywhere** to silence the compiler. It works, but it's usually a sign you should borrow.
- **Forgetting `mut` on both** the variable *and* the reference (`let mut x`, `&mut x`).
- **Holding a reference across a mutation** of the same value (the iterator-invalidation trap).
- **Returning a reference to a local variable.** Return the owned value.
- **Fighting the compiler.** When the borrow checker complains, it has found a real aliasing problem. Restructure: shorten the borrow, split the data, or pass the value instead.

## Exercises

1. Make `let a = String::from("x"); let b = a; println!("{a}");` compile three different ways (clone, borrow, reorder).
2. Write `fn count_vowels(s: &str) -> usize`, call it twice on the same `String`, and explain why that works.
3. Write `fn append_exclaim(s: &mut String)` and call it on a `String` variable.
4. Write `fn longest_word(text: &str) -> &str` (just return the first of equal length). Why does returning a `&str` from a `&str` parameter compile?
5. Create two shared references and one mutable reference to the same `Vec` in different orders. Find an order that compiles and one that doesn't.
6. Write a function that takes ownership of a `Vec<i32>` and returns its sum, then try to use the vector afterwards. Fix it by borrowing.
7. Take `&numbers[2..]` of an array and print its length.

## Recap

- One owner per value; when the owner leaves scope the value is dropped.
- Assignment and function calls **move** non-`Copy` values; `Copy` types (numbers, `bool`, `char`) are duplicated.
- `&T` borrows read-only; `&mut T` borrows for writing. Many readers **or** one writer, never both.
- References can't outlive their owner — dangling references are compile errors.
- Slices (`&str`, `&[T]`) borrow a part of a collection; prefer them in parameters.

Next: [Strings →](08-strings.md)
