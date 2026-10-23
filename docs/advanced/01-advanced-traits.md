# 1. Advanced traits & dispatch

## What & why

Traits are the center of Rust's design. This lesson covers the machinery behind them: how a call actually gets dispatched, what makes a trait usable as `dyn Trait`, how to extend foreign types safely, and the patterns (blanket impls, extension traits, sealed traits, generic associated types) that library authors use daily.

## Static vs dynamic dispatch, precisely

```rust
trait Shape {
    fn area(&self) -> f64;
}

struct Circle(f64);
struct Square(f64);
impl Shape for Circle { fn area(&self) -> f64 { 3.14159 * self.0 * self.0 } }
impl Shape for Square { fn area(&self) -> f64 { self.0 * self.0 } }

// Static: one specialised copy per concrete type. The call can be inlined.
fn total_static<T: Shape>(shapes: &[T]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

// Dynamic: one copy; each call goes through a vtable pointer.
fn total_dynamic(shapes: &[Box<dyn Shape>]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

fn main() {
    println!("{}", total_static(&[Circle(1.0), Circle(2.0)]));
    let mixed: Vec<Box<dyn Shape>> = vec![Box::new(Circle(1.0)), Box::new(Square(2.0))];
    println!("{}", total_dynamic(&mixed));
}
```

A `&dyn Shape` or `Box<dyn Shape>` is a **fat pointer**: two machine words, a data pointer and a vtable pointer. Check it:

```rust
use std::mem::size_of;

trait Shape {}

fn main() {
    println!("{}", size_of::<&u8>());              // 8  (on 64-bit)
    println!("{}", size_of::<&dyn Shape>());       // 16 — data pointer + vtable pointer
    println!("{}", size_of::<Box<dyn Shape>>());   // 16
    println!("{}", size_of::<&[u8]>());            // 16 — slices are fat too: pointer + length
}
```

The vtable holds the type's size, alignment, destructor and method pointers. Dynamic dispatch costs an indirect call and blocks inlining; it buys heterogeneous collections, smaller binaries and faster compiles.

## Dyn compatibility (object safety)

A trait can be used as `dyn Trait` only if the compiler can build a vtable for it. Methods are disqualifying if they:

- return `Self`, or take `Self` by value in arguments other than the receiver (`fn dup(&self) -> Self`),
- have generic type parameters (`fn map<T>(&self, ...)`),
- have no receiver (associated functions like `fn new() -> Self`),
- or the trait requires `Self: Sized`.

<!-- check: fail -->
```rust
trait Duplicate {
    fn duplicate(&self) -> Self;      // returns Self
}

impl Duplicate for i32 {
    fn duplicate(&self) -> i32 { *self }
}

fn main() {
    let x: Box<dyn Duplicate> = Box::new(5);   // error[E0038]: the trait `Duplicate` is not dyn compatible
}
```

You can opt individual methods out with `where Self: Sized`, keeping the rest dyn-compatible:

```rust
trait Animal {
    fn name(&self) -> String;

    fn boxed_clone(&self) -> Box<dyn Animal>;        // the standard "clone a trait object" trick

    fn named(name: &str) -> Self                     // excluded from the vtable
    where
        Self: Sized;
}

#[derive(Clone)]
struct Dog(String);

impl Animal for Dog {
    fn name(&self) -> String { self.0.clone() }
    fn boxed_clone(&self) -> Box<dyn Animal> { Box::new(self.clone()) }
    fn named(name: &str) -> Dog { Dog(name.into()) }
}

impl Clone for Box<dyn Animal> {
    fn clone(&self) -> Box<dyn Animal> { self.boxed_clone() }
}

fn main() {
    let a: Box<dyn Animal> = Box::new(Dog::named("Rex"));
    let b = a.clone();
    println!("{} {}", a.name(), b.name());
}
```

## Associated types vs generic parameters

```rust
// One output type per implementor: associated type.
trait Parser {
    type Output;
    fn parse(&self, input: &str) -> Option<Self::Output>;
}

// Many possible conversions per type: generic parameter.
trait ConvertTo<T> {
    fn convert(&self) -> T;
}

struct Digits;
impl Parser for Digits {
    type Output = u32;
    fn parse(&self, input: &str) -> Option<u32> { input.parse().ok() }
}

struct Celsius(f64);
impl ConvertTo<f64> for Celsius { fn convert(&self) -> f64 { self.0 } }
impl ConvertTo<String> for Celsius { fn convert(&self) -> String { format!("{}°C", self.0) } }

fn main() {
    println!("{:?}", Digits.parse("42"));
    let n: f64 = Celsius(21.5).convert();
    let s: String = Celsius(21.5).convert();
    println!("{n} {s}");
}
```

Use an associated type when the choice is determined by the implementor (an iterator's `Item`), and a generic parameter when a type legitimately implements the trait several ways (`From<A>`, `From<B>`). Associated types also keep bounds short: `fn f<I: Iterator<Item = u32>>(i: I)`.

Default type parameters exist too — that's how `Add` lets you write `a + b` for the same type by default:

```rust
use std::ops::Add;

#[derive(Debug, Clone, Copy)]
struct Meters(f64);
#[derive(Debug, Clone, Copy)]
struct Millimeters(f64);

// trait Add<Rhs = Self> { type Output; fn add(self, rhs: Rhs) -> Self::Output; }
impl Add<Millimeters> for Meters {
    type Output = Meters;
    fn add(self, rhs: Millimeters) -> Meters { Meters(self.0 + rhs.0 / 1000.0) }
}

fn main() {
    println!("{:?}", Meters(1.0) + Millimeters(250.0));    // Meters(1.25)
}
```

## Blanket implementations

You can implement a trait for *every* type satisfying a bound. The standard library does this constantly (`impl<T: Display> ToString for T`):

```rust
use std::fmt::Display;

trait Describe {
    fn describe(&self) -> String;
}

impl<T: Display> Describe for T {
    fn describe(&self) -> String {
        format!("<<{self}>>")
    }
}

fn main() {
    println!("{}", 42.describe());
    println!("{}", "hello".describe());
    println!("{}", 2.5.describe());
}
```

Consequences: you can't also write `impl Describe for i32` (it would overlap), and adding a blanket impl to a published trait is a breaking change.

## Extension traits

The orphan rule stops you adding methods to foreign types directly, but a trait *you* define can be implemented for them — the "extension trait" pattern (it's how `itertools` adds methods to every iterator):

```rust
trait StrExt {
    fn is_blank(&self) -> bool;
    fn truncate_to(&self, max: usize) -> &str;
}

impl StrExt for str {
    fn is_blank(&self) -> bool {
        self.trim().is_empty()
    }

    fn truncate_to(&self, max: usize) -> &str {
        match self.char_indices().nth(max) {
            Some((byte, _)) => &self[..byte],
            None => self,
        }
    }
}

trait IteratorExt: Iterator + Sized {
    fn every_other(self) -> std::iter::StepBy<Self> {
        self.step_by(2)
    }
}
impl<I: Iterator> IteratorExt for I {}

fn main() {
    println!("{} {}", "   ".is_blank(), "héllo wörld".truncate_to(4));   // true héll
    println!("{:?}", (1..=7).every_other().collect::<Vec<_>>());         // [1, 3, 5, 7]
}
```

## Fully qualified syntax

When two traits (or a trait and an inherent method) share a method name, say which you mean:

```rust
trait Pilot { fn fly(&self) -> &'static str; }
trait Wizard { fn fly(&self) -> &'static str; }

struct Human;
impl Pilot for Human { fn fly(&self) -> &'static str { "captain speaking" } }
impl Wizard for Human { fn fly(&self) -> &'static str { "up!" } }
impl Human { fn fly(&self) -> &'static str { "*waves arms*" } }

trait Animal { fn baby_name() -> String; }
struct Dog;
impl Dog { fn baby_name() -> String { "Spot".into() } }
impl Animal for Dog { fn baby_name() -> String { "puppy".into() } }

fn main() {
    let h = Human;
    println!("{}", h.fly());                    // inherent method wins
    println!("{}", Pilot::fly(&h));
    println!("{}", <Human as Wizard>::fly(&h));

    println!("{}", Dog::baby_name());           // inherent
    println!("{}", <Dog as Animal>::baby_name());   // no receiver, so the full form is required
}
```

## Generic associated types (GATs)

An associated type can itself have parameters, including lifetimes. The classic use is a "lending iterator", whose items borrow from the iterator itself — something plain `Iterator` can't express:

```rust
trait LendingIterator {
    type Item<'a> where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

struct Windows<'s> {
    data: &'s mut [i32],
    pos: usize,
}

impl<'s> LendingIterator for Windows<'s> {
    type Item<'a> = &'a mut [i32] where Self: 'a;

    fn next<'a>(&'a mut self) -> Option<&'a mut [i32]> {
        let end = self.pos + 2;
        if end > self.data.len() {
            return None;
        }
        let window = &mut self.data[self.pos..end];
        self.pos += 1;
        Some(window)                    // overlapping mutable windows: each borrows `self`
    }
}

fn main() {
    let mut numbers = [1, 2, 3, 4];
    let mut it = Windows { data: &mut numbers, pos: 0 };
    while let Some(w) = it.next() {
        w[1] += w[0];                   // running sum
    }
    println!("{numbers:?}");            // [1, 3, 6, 10]
}
```

`Iterator::next` can't do this because every `Item` must outlive the call to `next`; with a GAT the item's lifetime is tied to the borrow of `&mut self`.

## Traits in return position and `async fn`

Since Rust 1.75, traits may use `impl Trait` in return position and `async fn` directly:

```rust
trait Source {
    fn items(&self) -> impl Iterator<Item = u32>;      // return-position impl Trait in a trait (RPITIT)
}

struct Evens(u32);
impl Source for Evens {
    fn items(&self) -> impl Iterator<Item = u32> {
        (0..self.0).filter(|n| n % 2 == 0)
    }
}

fn total(s: &impl Source) -> u32 {
    s.items().sum()
}

fn main() {
    println!("{}", total(&Evens(10)));    // 20
}
```

These traits are not dyn-compatible; for async traits behind `dyn` you still need boxing (the `async-trait` crate automates it).

## `dyn Any`: runtime types

```rust
use std::any::Any;

fn describe(value: &dyn Any) -> String {
    if let Some(i) = value.downcast_ref::<i32>() {
        format!("an i32: {i}")
    } else if let Some(s) = value.downcast_ref::<String>() {
        format!("a String: {s}")
    } else {
        "something else".to_string()
    }
}

fn main() {
    println!("{}", describe(&42_i32));
    println!("{}", describe(&String::from("hi")));
    println!("{}", describe(&1.5_f32));
}
```

`Any` only works for `'static` types, and downcasting is a sign you may want an enum instead. It's the mechanism behind `Box<dyn Error>::downcast_ref`.

## Sealed traits

To publish a trait that downstream crates may *use* but not *implement* (so you can add methods later without breaking them), put a private supertrait in a private module:

```rust
mod sealed {
    pub trait Sealed {}
}

pub trait Codec: sealed::Sealed {
    fn name(&self) -> &'static str;
}

pub struct Json;
pub struct Toml;

impl sealed::Sealed for Json {}
impl sealed::Sealed for Toml {}
impl Codec for Json { fn name(&self) -> &'static str { "json" } }
impl Codec for Toml { fn name(&self) -> &'static str { "toml" } }

fn main() {
    println!("{} {}", Json.name(), Toml.name());
}
```

Outside the crate nobody can name `sealed::Sealed`, so nobody can implement `Codec` for their own type.

## `Sized` and `?Sized`

Every generic parameter is implicitly `T: Sized` (known size at compile time). `str`, `[T]` and `dyn Trait` are *unsized*; relax the bound to accept them behind a pointer:

```rust
use std::fmt::Display;

fn show<T: Display + ?Sized>(value: &T) -> String {
    format!("[{value}]")
}

fn main() {
    println!("{}", show("a str"));       // T = str, unsized
    println!("{}", show(&42));           // T = i32
    let d: &dyn Display = &3.5;
    println!("{}", show(d));             // T = dyn Display
}
```

## `AsRef`, `Borrow`, `Into` for flexible APIs

```rust
use std::path::Path;

fn file_name_len<P: AsRef<Path>>(path: P) -> usize {
    path.as_ref().file_name().map_or(0, |n| n.len())
}

fn greet(name: impl Into<String>) -> String {
    let owned: String = name.into();      // accepts &str, String, char -> String...
    format!("hello, {owned}")
}

fn main() {
    println!("{}", file_name_len("src/main.rs"));                 // &str
    println!("{}", file_name_len(String::from("a/b/c.txt")));     // String
    println!("{}", file_name_len(std::path::PathBuf::from("x.md")));
    println!("{} {}", greet("Ada"), greet(String::from("Grace")));
}
```

Accept `impl AsRef<Path>`, `impl AsRef<str>` or `impl Into<String>` in public APIs so callers don't have to convert by hand.

## Common mistakes

- **Reaching for `dyn` first.** Start with generics/enums; use `dyn` when you need open-ended heterogeneity.
- **Putting generic methods on a trait you want as `dyn`.** Add `where Self: Sized` to exclude them.
- **Blanket impls that conflict** with later concrete impls ("conflicting implementations").
- **Forgetting that trait methods need the trait in scope**, especially extension traits.
- **Using `Any` as a general-purpose escape hatch.**
- **Making a trait public and implementable** when you may want to extend it later — seal it.
- **Associated type where you wanted a generic** (or vice-versa) — ask: "can one type implement this twice?"

## Exercises

1. Implement a `trait Plugin { fn run(&self) -> String; }` for three structs and keep them in `Vec<Box<dyn Plugin>>`. Then add `fn clone_box(&self) -> Box<dyn Plugin>` and make the vector `Clone`.
2. Write an extension trait `VecExt` that adds `second()` to `Vec<T>` (returning `Option<&T>`).
3. Write a blanket impl `impl<T: Iterator<Item = u32>> Stats for T` providing `mean()` and `max_gap()`.
4. Print `size_of::<&dyn Trait>()` and `size_of::<Box<dyn Trait>>()`; explain the numbers.
5. Create a trait with both a generic method and `&self` methods, then make it dyn-compatible with `where Self: Sized`.
6. Write `fn total_len(items: &[impl AsRef<str>]) -> usize`.
7. Implement the lending iterator for overlapping `&mut` windows of a slice (as above) but with a configurable width.
8. Seal a `trait Backend` so only your crate's two backends implement it.

## Recap

- Generics monomorphize (static dispatch, inlining); `dyn Trait` is a fat pointer + vtable (dynamic dispatch).
- Only dyn-compatible traits can be objects: no `Self` returns, no generic methods, unless marked `where Self: Sized`.
- Associated types for "one answer per implementor"; generic parameters for "many".
- Blanket impls, extension traits and sealed traits are the library-design toolbox.
- GATs and RPITIT/async-in-traits extend what traits can express; `?Sized`, `AsRef`, `Into` make APIs flexible.

Next: [Advanced generics & type-level design →](02-advanced-generics.md)
