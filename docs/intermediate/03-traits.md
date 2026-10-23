# 3. Traits

## What & why

A **trait** describes behaviour a type can have: "things that can be printed", "things that can be compared", "things that can make a sound". It's Rust's version of interfaces / protocols, and it's the foundation of nearly everything in the standard library: `Display`, `Clone`, `Iterator`, `Add`, `From`, `Default`...

Traits let you write one function that works for many types, and let you add behaviour to types you didn't write.

## Defining and implementing a trait

```rust
trait Speak {
    fn name(&self) -> String;

    fn speak(&self) -> String;                    // required: implementors must provide it

    fn introduce(&self) -> String {               // provided: a default implementation
        format!("{} says {}", self.name(), self.speak())
    }
}

struct Dog;
struct Cat {
    lives: u8,
}

impl Speak for Dog {
    fn name(&self) -> String { "Dog".to_string() }
    fn speak(&self) -> String { "woof".to_string() }
}

impl Speak for Cat {
    fn name(&self) -> String { format!("Cat({} lives)", self.lives) }
    fn speak(&self) -> String { "meow".to_string() }

    fn introduce(&self) -> String {               // override the default
        format!("The cat purrs: {}", self.speak())
    }
}

fn main() {
    println!("{}", Dog.introduce());
    println!("{}", Cat { lives: 9 }.introduce());
}
```

## Traits as parameters: static dispatch

```rust
trait Speak {
    fn speak(&self) -> String;
}
struct Dog;
struct Cat;
impl Speak for Dog { fn speak(&self) -> String { "woof".into() } }
impl Speak for Cat { fn speak(&self) -> String { "meow".into() } }

// "Any type that implements Speak" — the compiler generates a copy for each concrete type.
fn announce(animal: &impl Speak) {
    println!("{}!", animal.speak());
}

// The same thing written with an explicit generic parameter and a trait bound:
fn announce_twice<T: Speak>(animal: &T) {
    println!("{} {}", animal.speak(), animal.speak());
}

fn main() {
    announce(&Dog);
    announce_twice(&Cat);
}
```

`impl Trait` in argument position is shorthand for a generic bound. Both are **monomorphized**: the compiler produces specialized machine code per type, so there is no runtime cost (lesson 4).

## Trait objects: dynamic dispatch

When you need a *collection of different types* sharing a trait, use a **trait object**, `dyn Trait`, behind a pointer:

```rust
trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> &str;
}

struct Circle(f64);
struct Square(f64);

impl Shape for Circle {
    fn area(&self) -> f64 { std::f64::consts::PI * self.0 * self.0 }
    fn name(&self) -> &str { "circle" }
}
impl Shape for Square {
    fn area(&self) -> f64 { self.0 * self.0 }
    fn name(&self) -> &str { "square" }
}

fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![Box::new(Circle(1.0)), Box::new(Square(2.0))];

    for s in &shapes {
        println!("{} has area {:.2}", s.name(), s.area());
    }
    let total: f64 = shapes.iter().map(|s| s.area()).sum();
    println!("total {total:.2}");
}
```

`Box<dyn Shape>` is a pointer plus a table of method addresses (a *vtable*); calls are looked up at run time. Trade-off:

| | `impl Trait` / generics | `dyn Trait` |
| --- | --- | --- |
| Dispatch | static (compile time) | dynamic (run time) |
| Speed | fastest; can inline | one indirect call |
| Binary size | grows per type | smaller |
| Mixed types in one `Vec` | no | **yes** |

Default to generics; use `dyn` when you need heterogeneous collections or plugin-style design. Not every trait can be a trait object: its methods can't return `Self` or have their own generic parameters (that's called *dyn compatibility*; the compiler tells you when you break it).

## Return types

```rust
use std::fmt::Display;

fn pick(flag: bool) -> impl Display {
    if flag { 42 } else { 7 }                     // one concrete type: i32
}

fn pick_either(flag: bool) -> Box<dyn Display> {
    if flag { Box::new(42) } else { Box::new("seven") }   // different types: need dyn
}

fn main() {
    println!("{} {}", pick(true), pick_either(false));
}
```

`-> impl Trait` means "some single type implementing it". For truly different types, return `Box<dyn Trait>`.

## Standard-library traits you will implement

### `Display` and `Debug`

```rust
use std::fmt;

struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

fn main() {
    let p = Point { x: 3, y: -1 };
    println!("{p}");                     // (3, -1)
    let s = p.to_string();               // Display gives you ToString for free
    println!("{}", s.len());
}
```

### `From` / `Into`

```rust
struct Celsius(f64);
struct Fahrenheit(f64);

impl From<Celsius> for Fahrenheit {
    fn from(c: Celsius) -> Fahrenheit {
        Fahrenheit(c.0 * 9.0 / 5.0 + 32.0)
    }
}

fn main() {
    let f: Fahrenheit = Celsius(100.0).into();    // implementing From gives you Into
    let g = Fahrenheit::from(Celsius(0.0));
    println!("{} {}", f.0, g.0);                  // 212 32
}
```

### `PartialEq`, `PartialOrd` — operators by hand

```rust
use std::cmp::Ordering;
use std::ops::Add;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Money {
    cents: i64,
}

impl Add for Money {
    type Output = Money;
    fn add(self, other: Money) -> Money {
        Money { cents: self.cents + other.cents }
    }
}

impl PartialOrd for Money {
    fn partial_cmp(&self, other: &Money) -> Option<Ordering> {
        self.cents.partial_cmp(&other.cents)
    }
}

fn main() {
    let total = Money { cents: 250 } + Money { cents: 199 };
    println!("{total:?}");
    println!("{}", total > Money { cents: 400 });     // true
}
```

### `Default`, `Clone`, `Iterator`

```rust
#[derive(Debug, Clone)]
struct Config {
    verbose: bool,
    retries: u32,
    name: String,
}

impl Default for Config {
    fn default() -> Config {
        Config { verbose: false, retries: 3, name: "app".into() }
    }
}

struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0 + 1)
        }
    }
}

fn main() {
    let c = Config { verbose: true, ..Default::default() };
    println!("{c:?}");
    println!("{:?}", Countdown(3).collect::<Vec<_>>());    // [3, 2, 1]
    println!("{}", Countdown(4).map(|n| n * n).sum::<u32>());   // 30
}
```

Implement `Iterator` by writing one method, `next`, and you automatically get `map`, `filter`, `sum`, `collect`, and about seventy others.

## Extending existing types

You can implement your own trait for any type, including built-in ones:

```rust
trait Shout {
    fn shout(&self) -> String;
}

impl Shout for str {
    fn shout(&self) -> String {
        self.to_uppercase() + "!"
    }
}

fn main() {
    println!("{}", "hello".shout());
}
```

The **orphan rule**: you may implement a trait for a type only if *you own the trait or you own the type*. You can't implement `Display` (std's trait) for `Vec<i32>` (std's type). Wrap it in a newtype instead.

## Supertraits and generic bounds

```rust
use std::fmt::{Debug, Display};

trait Entity: Debug {                  // anything implementing Entity must also implement Debug
    fn id(&self) -> u32;
}

#[derive(Debug)]
struct User(u32);
impl Entity for User {
    fn id(&self) -> u32 { self.0 }
}

fn log<T: Entity>(e: &T) {
    println!("{:?} has id {}", e, e.id());       // can use Debug thanks to the supertrait
}

// Multiple bounds, and the `where` clause for readability:
fn show_both<A, B>(a: A, b: B) -> String
where
    A: Display + Clone,
    B: Debug,
{
    format!("{a} / {b:?}")
}

fn main() {
    log(&User(7));
    println!("{}", show_both("x", vec![1, 2]));
}
```

## Common mistakes

- **Forgetting to bring a trait into scope.** Trait methods only work when the trait is imported (`use std::fmt::Write;`).
- **Using `dyn Trait` by value.** It needs a pointer: `&dyn Trait`, `Box<dyn Trait>`.
- **Returning different concrete types from `-> impl Trait`.** It must be one type; use `Box<dyn Trait>` otherwise.
- **Violating the orphan rule.** Use a newtype wrapper.
- **Reaching for trait objects out of habit** (from OOP languages) when an `enum` or a generic would be simpler and faster.
- **Overusing `.clone()` in default methods** — take `&self` and borrow.

## Exercises

1. Define `trait Area { fn area(&self) -> f64; }` for `Circle` and `Rectangle`, and a function that returns the sum of areas in a `&[Box<dyn Area>]`.
2. Implement `Display` for a `Matrix2x2` so it prints on two lines.
3. Implement `From<(i32, i32)> for Point` and use `.into()`.
4. Implement `Iterator` for `Fibonacci` and take the first ten values.
5. Implement `Add` and `Mul<f64>` for a `Vec2 { x, y }` struct.
6. Add a default method `describe()` to a trait that calls two required methods.
7. Write `fn largest<T: PartialOrd + Copy>(items: &[T]) -> T` and call it with `i32`, `f64` and `char`.
8. Create a `trait Plugin { fn run(&self); }`, register three different plugin types in a `Vec<Box<dyn Plugin>>`, and run them all.

## Recap

- A trait names behaviour; `impl Trait for Type` provides it; default methods reduce boilerplate.
- Generics and `impl Trait` give static dispatch (fast, monomorphized); `dyn Trait` gives dynamic dispatch (mixed collections).
- Core std traits — `Display`, `Debug`, `Clone`, `From`, `Default`, `Iterator`, operators — are how your types plug into the language.
- The orphan rule limits who can implement what; use newtypes to work around it.
- Supertraits and `where` clauses express richer requirements.

Next: [Generics →](04-generics.md)
