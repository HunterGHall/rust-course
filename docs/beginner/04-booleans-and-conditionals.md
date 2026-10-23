# 4. Booleans & conditionals

## What & why

Programs make decisions. Rust's decision tools are `if`, `match`, and a few friends — and a distinctive feature runs through all of them: **they are expressions**, meaning they produce values. That removes a lot of temporary variables and makes it hard to forget a case.

## Booleans and comparisons

```rust
fn main() {
    let age = 20;
    let adult = age >= 18;
    println!("{adult}");               // true

    println!("{}", 3 == 3);            // true
    println!("{}", 3 != 4);            // true
    println!("{}", 2 < 5 && 5 <= 5);   // true
    println!("{}", "a" < "b");         // true — strings compare lexicographically
    println!("{}", !adult);            // false
}
```

Operators: `==` `!=` `<` `>` `<=` `>=` compare; `&&` (and), `||` (or), `!` (not) combine. `&&` and `||` **short-circuit**: the right side is only evaluated if needed.

Rust has no "truthy" values. The condition of an `if` must be a `bool` — `if 1 { }` or `if name { }` is a compile error.

## `if` / `else`

```rust
fn main() {
    let temperature = 28;

    if temperature > 30 {
        println!("hot");
    } else if temperature > 20 {
        println!("warm");
    } else {
        println!("cool");
    }
}
```

No parentheses around the condition; braces are required.

## `if` is an expression

```rust
fn main() {
    let n = 7;
    let kind = if n % 2 == 0 { "even" } else { "odd" };
    println!("{n} is {kind}");

    let abs = if n < 0 { -n } else { n };
    println!("{abs}");
}
```

Both branches must produce the **same type**, and when used as a value there must be an `else`. Notice there's no `;` after `"even"` — the last expression in a block, without a semicolon, is the block's value (more in lesson 6). Rust has no `?:` ternary operator because `if` already does the job.

## `match`

`match` compares a value against **patterns** and runs the first one that fits. The compiler checks that you covered every possibility.

```rust
fn main() {
    let number = 3;

    match number {
        1 => println!("one"),
        2 | 3 => println!("two or three"),      // | means "or"
        4..=9 => println!("four to nine"),      // inclusive range
        _ => println!("something else"),        // _ catches everything left
    }
}
```

As an expression:

```rust
fn main() {
    let score = 87;
    let grade = match score {
        90..=100 => 'A',
        80..=89 => 'B',
        70..=79 => 'C',
        _ => 'F',
    };
    println!("{grade}");   // B
}
```

Remove the `_` arm and the compiler refuses to build, because integers have many more values than you listed. That exhaustiveness check is a major source of Rust's reliability.

### Guards and bindings

```rust
fn main() {
    let n = 15;
    let label = match n {
        x if x % 15 == 0 => "FizzBuzz",
        x if x % 3 == 0 => "Fizz",
        x if x % 5 == 0 => "Buzz",
        _ => "number",
    };
    println!("{label}");

    let age = 34;
    match age {
        0 => println!("newborn"),
        a @ 1..=17 => println!("minor, {a}"),   // bind the value while matching a range
        a => println!("adult, {a}"),
    }
}
```

### Matching several values at once

```rust
fn main() {
    let (x, y) = (3, -3);
    match (x, y) {
        (0, 0) => println!("origin"),
        (x, 0) | (0, x) => println!("on an axis at {x}"),
        (x, y) if x == -y => println!("on the anti-diagonal"),
        _ => println!("elsewhere"),
    }
}
```

You'll use `match` constantly with enums, `Option` and `Result` — the real payoff comes in Intermediate.

## `if let` and `let else`

When you only care about one pattern, `match` is wordy. `if let` is the short form:

```rust
fn main() {
    let favourite: Option<&str> = Some("Rust");

    if let Some(name) = favourite {
        println!("favourite: {name}");
    } else {
        println!("no favourite");
    }

    // let-else: bind or bail out early.
    let Some(name) = favourite else {
        println!("nothing to do");
        return;
    };
    println!("still here with {name}");
}
```

`Option` is covered in lesson 11. `let ... else` is excellent for early exits: the happy path stays un-indented.

## `matches!`

```rust
fn main() {
    let c = 'k';
    println!("{}", matches!(c, 'a'..='z'));          // true
    println!("{}", matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'));   // false
}
```

## Common mistakes

- **Non-bool conditions.** `if x` where `x` is an integer won't compile; write `if x != 0`.
- **A missing `else` when `if` is used as a value.** `let v = if c { 1 };` is an error.
- **Branches with different types** (`"even"` vs `0`).
- **A stray `;` after the last expression** in a branch, which turns the value into `()`.
- **Forgetting the catch-all** in a `match` over integers or strings.
- **Putting a catch-all first.** `_` matches everything, so arms after it are unreachable (the compiler warns).
- **Using `=` instead of `==`.**

## Exercises

1. Write a program that reads an integer and prints whether it is negative, zero or positive.
2. Compute a letter grade with `match` and ranges, then add a guard that prints "perfect!" for 100.
3. Implement FizzBuzz for 1..=30 with a single `match` on `(n % 3, n % 5)`.
4. Use `if` as an expression to set `max` of two numbers (without calling `.max()`).
5. Read a month number and print how many days it has (ignore leap years) with `match` and `|`.
6. Use `matches!` to check whether a `char` is a vowel.
7. Write `fn first_even(a: i32, b: i32) -> Option<i32>` using `if`/`else` branches that return `Some`/`None`.

## Recap

- Conditions must be `bool`; there is no truthiness.
- `if`, `else if`, `else` — and `if` is an expression, so both arms need the same type.
- `match` checks patterns top-down and must be exhaustive; use `|`, ranges, guards, `@`, and `_`.
- `if let` and `let else` are shorthand for single-pattern matches.
- `matches!` returns a `bool` for a pattern test.

Next: [Loops →](05-loops.md)
