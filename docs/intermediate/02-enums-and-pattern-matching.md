# 2. Enums & pattern matching

## What & why

A Rust **enum** says "a value is exactly one of these variants" — and each variant can carry its own data. That's far more powerful than the C-style enum of named integers. Combined with exhaustive `match`, enums let you model states precisely, so illegal states can't be represented and forgotten cases don't compile.

You've been using two already: `Option<T>` and `Result<T, E>` are ordinary enums.

## Defining an enum

```rust
#[derive(Debug)]
enum Direction {
    North,
    East,
    South,
    West,
}

fn turn_right(d: Direction) -> Direction {
    match d {
        Direction::North => Direction::East,
        Direction::East => Direction::South,
        Direction::South => Direction::West,
        Direction::West => Direction::North,
    }
}

fn main() {
    let d = Direction::North;
    println!("{:?}", turn_right(d));      // East
}
```

`match` on an enum must cover every variant. Add a variant later and the compiler flags every `match` that needs updating. That's one of Rust's most useful refactoring aids.

## Variants with data

```rust
#[derive(Debug)]
enum Shape {
    Circle(f64),                          // tuple-like: radius
    Rectangle { width: f64, height: f64 },// struct-like: named fields
    Point,                                // no data
}

impl Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle(r) => std::f64::consts::PI * r * r,
            Shape::Rectangle { width, height } => width * height,
            Shape::Point => 0.0,
        }
    }
}

fn main() {
    let shapes = [
        Shape::Circle(1.0),
        Shape::Rectangle { width: 2.0, height: 3.0 },
        Shape::Point,
    ];
    for s in &shapes {
        println!("{s:?} -> {:.2}", s.area());
    }
}
```

An enum is one type of one size (as big as its largest variant, plus a tag), so a `Vec<Shape>` holds a mix of variants happily.

## Modelling a state machine

Enums with data shine at "this can be in one of several states, each with different information":

```rust
#[derive(Debug)]
enum Connection {
    Disconnected,
    Connecting { attempt: u32 },
    Connected { session_id: String },
    Failed(String),
}

impl Connection {
    fn describe(&self) -> String {
        match self {
            Connection::Disconnected => "offline".to_string(),
            Connection::Connecting { attempt } => format!("connecting (try {attempt})"),
            Connection::Connected { session_id } => format!("online as {session_id}"),
            Connection::Failed(reason) => format!("failed: {reason}"),
        }
    }
}

fn main() {
    let states = vec![
        Connection::Disconnected,
        Connection::Connecting { attempt: 2 },
        Connection::Connected { session_id: "abc123".into() },
        Connection::Failed("timeout".into()),
    ];
    for s in &states {
        println!("{}", s.describe());
    }
}
```

Compare this to a struct with `connected: bool`, `session_id: Option<String>`, `error: Option<String>` — which allows nonsense like "connected with an error". The enum cannot.

## Pattern matching in depth

Patterns can destructure, nest, and bind:

```rust
#[derive(Debug)]
enum Expr {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
}

fn eval(e: &Expr) -> i64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Add(a, b) => eval(a) + eval(b),
        Expr::Mul(a, b) => eval(a) * eval(b),
        Expr::Neg(inner) => -eval(inner),
    }
}

fn main() {
    use Expr::*;                                   // bring the variants into scope
    // (2 + 3) * -4
    let e = Mul(
        Box::new(Add(Box::new(Num(2)), Box::new(Num(3)))),
        Box::new(Neg(Box::new(Num(4)))),
    );
    println!("{}", eval(&e));                      // -20
}
```

`Box<Expr>` is needed because an `Expr` can't contain another `Expr` directly (infinite size); a `Box` is a pointer to the heap (lesson 7).

More pattern forms:

```rust
fn classify(pair: (i32, i32)) -> &'static str {
    match pair {
        (0, 0) => "origin",
        (x, 0) if x > 0 => "positive x-axis",
        (_, 0) => "negative x-axis",
        (0, _) => "y-axis",
        (x, y) if x == y => "diagonal",
        _ => "somewhere else",
    }
}

fn main() {
    for p in [(0, 0), (3, 0), (-2, 0), (0, 9), (4, 4), (1, 2)] {
        println!("{p:?}: {}", classify(p));
    }

    // Slice patterns
    let values = [1, 2, 3, 4, 5];
    match values {
        [first, .., last] => println!("{first}..{last}"),
    }
    match &values[..] {
        [] => println!("empty"),
        [one] => println!("just {one}"),
        [first, rest @ ..] => println!("{first} then {} more", rest.len()),
    }

    // Binding with @ and ranges
    let age = 42;
    match age {
        n @ 0..=17 => println!("minor {n}"),
        n @ (18..=64) => println!("adult {n}"),
        n => println!("senior {n}"),
    }
}
```

### Matching references and moving out

`match` on a reference (`match self`, `match &value`) borrows the data inside. `match value` by value **moves** non-`Copy` parts into the bindings:

```rust
enum Message {
    Text(String),
    Quit,
}

fn main() {
    let msg = Message::Text("hello".to_string());
    match &msg {                                   // borrow: msg remains usable
        Message::Text(t) => println!("text: {t}"),
        Message::Quit => println!("quit"),
    }
    if let Message::Text(t) = msg {                // moves the String out of msg
        println!("took {t}");
    }
}
```

## `Option` and `Result` are enums

Their definitions are in the standard library:

```rust
enum MyOption<T> {
    Some(T),
    None,
}

enum MyResult<T, E> {
    Ok(T),
    Err(E),
}

fn main() {
    let a: MyOption<i32> = MyOption::Some(3);
    let b: MyResult<i32, String> = MyResult::Err("no".to_string());
    if let MyOption::Some(n) = a { println!("{n}"); }
    if let MyResult::Err(e) = b { println!("{e}"); }
}
```

Everything you learned about them — `match`, `if let`, `?` — is just enum pattern matching.

## `if let`, `while let`, `let else`, `matches!`

```rust
fn main() {
    let mut stack = vec![1, 2, 3];

    while let Some(top) = stack.pop() {      // loop until pop returns None
        print!("{top} ");
    }
    println!();

    let value: Option<u32> = Some(9);
    if let Some(n @ 5..=10) = value {
        println!("in range: {n}");
    }

    let words = vec!["a", "bb", "ccc"];
    let long = words.iter().filter(|w| matches!(w.len(), 2..=3)).count();
    println!("{long}");
}
```

## Enums with explicit values and conversion

C-like enums can have discriminants and be cast to integers:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum Level {
    Low = 1,
    Medium = 5,
    High = 10,
}

impl TryFrom<u8> for Level {
    type Error = String;

    fn try_from(n: u8) -> Result<Level, String> {
        match n {
            1 => Ok(Level::Low),
            5 => Ok(Level::Medium),
            10 => Ok(Level::High),
            other => Err(format!("{other} is not a level")),
        }
    }
}

fn main() {
    println!("{}", Level::High as u8);                 // 10
    println!("{:?}", Level::try_from(5));              // Ok(Medium)
    println!("{:?}", Level::try_from(7));              // Err("7 is not a level")
}
```

The cast goes enum → integer only; integer → enum must be checked, hence `TryFrom`.

## Common mistakes

- **A catch-all `_` that hides new variants.** Prefer listing every variant for enums you own, so adding one forces a review.
- **Matching a non-reference by value when you needed to keep it** — match on `&value` or `self` by reference.
- **Forgetting `Box`** in a recursive enum: `error[E0072]: recursive type has infinite size`.
- **Using several `bool`/`Option` fields** where one enum would make invalid combinations impossible.
- **Writing `Direction::North` repeatedly** — `use Direction::*;` inside a function is fine; avoid it at module level.
- **Binding a name by accident** in a pattern: `match x { MAX => ... }` with a lowercase or unknown name creates a *new variable* that matches everything. Constants must be UPPERCASE, in scope, and (for enum variants) qualified or imported.

## Exercises

1. Define `enum Coin { Penny, Nickel, Dime, Quarter }` and `fn cents(c: &Coin) -> u32`.
2. Model a traffic light as an enum with `fn next(&self) -> Light` and `fn duration_secs(&self) -> u32`.
3. Extend `Shape` with `Triangle { base, height }` and watch the compiler list the `match` you must update.
4. Model a vending machine: `Idle`, `HasMoney(u32)`, `Dispensing { item: String }`. Write `fn insert_coin(self, cents: u32) -> VendingState`.
5. Add `Sub` and `Div` to the `Expr` evaluator; make `Div` return `Result<i64, String>` for division by zero.
6. Write `fn describe(values: &[i32]) -> String` with slice patterns for 0, 1, 2 and many elements.
7. Parse command strings (`"move 3 4"`, `"say hello"`, `"quit"`) into an `enum Command` using `split_whitespace` and `match`.

## Recap

- Enums are "one of" types; variants may carry tuple- or struct-like data.
- `match` must be exhaustive; patterns destructure, nest, use ranges, guards (`if`), `@` bindings, and slice patterns.
- `Option` and `Result` are just enums.
- `if let`, `while let`, `let else` and `matches!` are shorthand when you only care about one pattern.
- Recursive types need `Box` for indirection.

Next: [Traits →](03-traits.md)
