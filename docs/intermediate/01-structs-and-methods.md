# 1. Structs & methods

## What & why

Beginner code works with built-in types. Real programs model things — a user, a task, a point on a map — and Rust's tool for that is the **struct**: a named bundle of fields. Behaviour attaches to a struct with an **`impl`** block of methods. Rust has no classes and no inheritance; structs plus traits (lesson 3) cover the same ground with fewer surprises.

## Defining and creating a struct

```rust
struct User {
    name: String,
    email: String,
    age: u32,
    active: bool,
}

fn main() {
    let mut user = User {
        name: String::from("Ada"),
        email: String::from("ada@example.com"),
        age: 36,
        active: true,
    };

    println!("{} <{}>", user.name, user.email);
    user.age += 1;                       // the whole variable must be `mut`
    println!("{}", user.age);
}
```

Rust has no per-field `mut`: either the binding is mutable and every field can change, or it isn't.

### Shorthand features

```rust
#[derive(Debug, Clone)]
struct User {
    name: String,
    email: String,
    active: bool,
}

fn build(name: String, email: String) -> User {
    User { name, email, active: true }          // field init shorthand
}

fn main() {
    let a = build("Ada".to_string(), "ada@example.com".to_string());

    // Struct update syntax: copy the remaining fields from another value.
    let b = User { name: "Grace".to_string(), ..a.clone() };
    println!("{b:?}");
}
```

`#[derive(Debug)]` generates the code behind `{:?}`. Use `{:#?}` for a pretty-printed, multi-line version. `..a` *moves* any non-`Copy` fields out of `a`, which is why the example clones it first.

## Tuple structs and unit structs

```rust
struct Meters(f64);                 // a tuple struct: fields accessed as .0
struct Rgb(u8, u8, u8);
struct Marker;                      // a unit struct: no data at all

fn main() {
    let height = Meters(1.82);
    let red = Rgb(255, 0, 0);
    let _m = Marker;
    println!("{} {}", height.0, red.0);
}
```

The "newtype" pattern — a tuple struct with one field — gives a primitive a distinct type, so you can't pass `Meters` where `Seconds` is expected:

```rust
struct Meters(f64);
struct Seconds(f64);

fn speed(distance: Meters, time: Seconds) -> f64 {
    distance.0 / time.0
}

fn main() {
    println!("{}", speed(Meters(100.0), Seconds(9.58)));
}
```

## Methods with `impl`

```rust
#[derive(Debug)]
struct Rectangle {
    width: f64,
    height: f64,
}

impl Rectangle {
    // An associated function (no self): called as Rectangle::new(...). Often a constructor.
    fn new(width: f64, height: f64) -> Rectangle {
        Rectangle { width, height }
    }

    fn square(side: f64) -> Self {              // `Self` is an alias for Rectangle
        Self::new(side, side)
    }

    // &self: read the struct.
    fn area(&self) -> f64 {
        self.width * self.height
    }

    fn can_hold(&self, other: &Rectangle) -> bool {
        self.width > other.width && self.height > other.height
    }

    // &mut self: modify the struct.
    fn scale(&mut self, factor: f64) {
        self.width *= factor;
        self.height *= factor;
    }

    // self: consume the struct and return something new.
    fn into_square(self) -> Rectangle {
        let side = self.width.max(self.height);
        Rectangle::square(side)
    }
}

fn main() {
    let mut r = Rectangle::new(3.0, 4.0);
    println!("{}", r.area());                 // 12
    r.scale(2.0);
    println!("{r:?}");
    println!("{}", r.can_hold(&Rectangle::square(1.0)));   // true

    let sq = r.into_square();                 // r is moved; can't be used afterwards
    println!("{sq:?}");
}
```

The first parameter decides how the method borrows the value, tying straight back to ownership:

| Receiver | Meaning | Caller keeps the value? |
| --- | --- | --- |
| `&self` | read-only borrow | yes |
| `&mut self` | exclusive borrow, may modify | yes |
| `self` | takes ownership | no (moved) |

Calling `r.area()` is sugar for `Rectangle::area(&r)`; Rust inserts the `&` or `&mut` for you ("automatic referencing"). You can have several `impl` blocks for one type.

## Associated constants

```rust
struct Circle {
    radius: f64,
}

impl Circle {
    const PI: f64 = 3.141_592_653_589_793;

    fn area(&self) -> f64 {
        Self::PI * self.radius * self.radius
    }
}

fn main() {
    println!("{:.3}", Circle { radius: 2.0 }.area());   // 12.566
}
```

## Derive: free behaviour

Common traits you can derive:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let a = Point { x: 1, y: 2 };
    let b = a;                                   // Copy: a is still usable
    println!("{}", a == b);                      // PartialEq
    println!("{:?}", Point::default());          // Default -> Point { x: 0, y: 0 }
    println!("{:?}", a.cmp(&Point { x: 1, y: 3 }));   // Ord — compares field by field: Less
    let p = Point { x: 5, ..Default::default() };
    println!("{p:?}");
}
```

| Derive | Gives you |
| --- | --- |
| `Debug` | `{:?}` formatting |
| `Clone` / `Copy` | `.clone()` / implicit copying (all fields must be `Clone`/`Copy`) |
| `PartialEq` / `Eq` | `==`, `!=` |
| `PartialOrd` / `Ord` | `<`, `>`, sorting |
| `Hash` | use as a `HashMap` key |
| `Default` | `Type::default()` |

## Ownership inside structs

A struct that holds a `String` **owns** it. A struct holding a reference needs a *lifetime* (lesson 5), so start with owned fields:

```rust
struct Article {
    title: String,        // owned: simple, no lifetimes needed
}

fn main() {
    let title = String::from("Ownership");
    let article = Article { title };     // title moves into the struct
    println!("{}", article.title);
}
```

Moving a single field out of a struct "partially moves" it; prefer borrowing (`&article.title`) or cloning.

## Getters, privacy and invariants

Fields are private to their module by default (lesson 9). Making them private and exposing methods lets a type protect its own rules:

```rust
pub struct Percentage(u8);

impl Percentage {
    pub fn new(value: u8) -> Option<Percentage> {
        if value <= 100 { Some(Percentage(value)) } else { None }
    }

    pub fn get(&self) -> u8 {
        self.0
    }
}

fn main() {
    println!("{:?}", Percentage::new(50).map(|p| p.get()));    // Some(50)
    println!("{:?}", Percentage::new(150).map(|p| p.get()));   // None
}
```

Once built through `new`, a `Percentage` is *always* valid — no checks needed elsewhere. This is "make illegal states unrepresentable".

## Common mistakes

- **Forgetting `mut`** on the binding when calling a `&mut self` method.
- **Using a value after a `self` method consumed it.** Use `&self` unless you truly need to take ownership.
- **`Self` vs `self`.** `Self` is the type, `self` is the value.
- **Deriving `Copy` on types holding `String`/`Vec`** — impossible; use `Clone`.
- **Reaching for `.clone()` to avoid borrowing**; borrow with `&self` methods instead.
- **Adding a field without updating every struct literal** — the compiler lists the ones you missed, which is a feature.

## Exercises

1. Define `struct Book { title: String, pages: u32, read: bool }` with `new`, a `finish(&mut self)` method, and a `summary(&self) -> String`.
2. Add `#[derive(Debug)]` and print a `Book` with `{:#?}`.
3. Write a `Temperature(f64)` newtype with `from_celsius`, `to_fahrenheit(&self)` and `to_kelvin(&self)`.
4. Make `Point` with `distance_to(&self, other: &Point) -> f64` and `translate(&mut self, dx, dy)`.
5. Write `struct Counter` with a private field and methods `increment`, `reset` and `value`.
6. Create a `BankAccount` whose `withdraw(&mut self, amount: u64) -> bool` refuses to overdraw.
7. Derive `PartialOrd, Ord` on a `Version { major, minor, patch }` struct and sort a vector of versions.
8. Write a method that takes `self` and returns a modified copy (the "builder" style: `Config::new().port(8080).verbose(true)`).

## Recap

- A `struct` bundles named fields; tuple structs and unit structs are lighter variants.
- `impl` blocks add methods (`&self`, `&mut self`, `self`) and associated functions like `new`.
- `Self` names the type; associated consts live in `impl` too.
- `#[derive(...)]` gives you `Debug`, `Clone`, `PartialEq`, `Default`, and more.
- Private fields + constructors let a type guarantee its own invariants.

Next: [Enums & pattern matching →](02-enums-and-pattern-matching.md)
