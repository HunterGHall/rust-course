# 6. Functions

## What & why

Functions let you name a piece of work, reuse it, and test it. Rust's functions are explicit about their inputs and output: every parameter has a type, and so does the return value. Combined with the "last expression is the value" rule, they end up compact.

## Defining and calling

```rust
fn greet(name: &str) {
    println!("Hello, {name}!");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    greet("Ada");
    let total = add(2, 3);
    println!("{total}");
}
```

- Parameters are `name: Type` — types are **required** on function signatures.
- `-> Type` declares the return type. No arrow means the function returns `()` (nothing).
- Functions can be defined before or after use; order doesn't matter.
- Names are `snake_case`.

## Statements vs expressions

This is the single most important thing to understand about Rust syntax.

- A **statement** does something and produces no value: `let x = 5;`
- An **expression** produces a value: `5`, `x + 1`, `if a { b } else { c }`, a function call, or a `{ }` block.

The last expression of a block, **with no semicolon**, is the block's value:

```rust
fn square(x: i32) -> i32 {
    x * x          // no semicolon: this is the return value
}

fn main() {
    let y = {
        let a = 3;
        a * a + 1  // the block evaluates to 10
    };
    println!("{} {y}", square(4));
}
```

Add a `;` and the expression becomes a statement; the block then returns `()`:

<!-- check: fail -->
```rust
fn square(x: i32) -> i32 {
    x * x;         // error[E0308]: mismatched types — expected `i32`, found `()`
}

fn main() {
    println!("{}", square(4));
}
```

The compiler even points at the semicolon and says "remove this semicolon". Rust programmers use `return` only for **early exits**:

```rust
fn describe(n: i32) -> &'static str {
    if n < 0 {
        return "negative";     // early return
    }
    if n == 0 {
        return "zero";
    }
    "positive"                 // normal return: last expression
}

fn main() {
    println!("{} {} {}", describe(-1), describe(0), describe(9));
}
```

## Returning several values

Return a tuple and destructure it:

```rust
fn min_max(values: &[i32]) -> (i32, i32) {
    let mut min = values[0];
    let mut max = values[0];
    for &v in values {
        if v < min { min = v; }
        if v > max { max = v; }
    }
    (min, max)
}

fn main() {
    let (lo, hi) = min_max(&[4, -2, 9, 0]);
    println!("{lo} {hi}");   // -2 9
}
```

(`&[i32]` is a *slice* — a borrowed view of a list. Lesson 9 covers it.)

## No default or named arguments

Rust has no default parameter values, no keyword arguments and no overloading. Instead you use:

- different function names (`connect`, `connect_with_timeout`),
- `Option` parameters (`timeout: Option<u32>`),
- or, for many settings, a struct (Intermediate lesson 1).

## Functions are values

A function name can be passed around like any other value. The type is `fn(args) -> ret`:

```rust
fn double(x: i32) -> i32 { x * 2 }
fn negate(x: i32) -> i32 { -x }

fn apply(f: fn(i32) -> i32, value: i32) -> i32 {
    f(value)
}

fn main() {
    println!("{}", apply(double, 21));    // 42
    println!("{}", apply(negate, 21));    // -21

    let table: [(&str, fn(i32) -> i32); 2] = [("double", double), ("negate", negate)];
    for (name, f) in table {
        println!("{name}(5) = {}", f(5));
    }
}
```

Closures (anonymous functions that capture their surroundings) come in Intermediate lesson 6.

## Recursion

```rust
fn factorial(n: u64) -> u64 {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

fn fibonacci(n: u32) -> u64 {
    let (mut a, mut b) = (0, 1);
    for _ in 0..n {
        (a, b) = (b, a + b);
    }
    a
}

fn main() {
    println!("{}", factorial(10));    // 3628800
    println!("{}", fibonacci(50));    // 12586269025
}
```

Rust does not guarantee tail-call optimization, so very deep recursion can overflow the stack. When a loop is just as clear, as in `fibonacci` above, prefer the loop.

## Nested functions and shadowing

Functions may be defined inside other functions; they are private to that body:

```rust
fn main() {
    fn helper(x: i32) -> i32 { x + 1 }
    println!("{}", helper(1));
}
```

## Documenting functions

Triple-slash comments become documentation (`cargo doc --open`), and code examples in them are *tested*:

```rust
/// Returns the larger of two numbers.
///
/// # Examples
///
/// ```
/// assert_eq!(max_of(3, 9), 9);
/// ```
fn max_of(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}

fn main() {
    println!("{}", max_of(3, 9));
}
```

## The `main` function

`main` can also return a `Result`, which lets you use `?` for errors (lesson 11):

```rust
fn main() -> Result<(), std::num::ParseIntError> {
    let n: i32 = "42".parse()?;
    println!("{n}");
    Ok(())
}
```

## Common mistakes

- **A semicolon on the last line** of a function that should return a value.
- **Forgetting parameter types.** Unlike local variables, signatures are never inferred.
- **Using `return` at the end** of every function. It works, but isn't idiomatic; clippy will tell you.
- **Mismatched return types in branches** (`if` returns `i32`, `else` returns `&str`).
- **Deep recursion on big inputs** causing a stack overflow.
- **Trying to give a parameter a default value** — it's not supported; use `Option` or a struct.

## Exercises

1. Write `fn is_even(n: i32) -> bool` in one line, with no `return`.
2. Write `fn average(values: &[f64]) -> f64` (what should it do for an empty slice? decide, then document it).
3. Write `fn swap(pair: (i32, i32)) -> (i32, i32)`.
4. Write `fn gcd(a: u64, b: u64) -> u64` using Euclid's algorithm, first recursively and then with a loop.
5. Write a function that takes another function: `fn twice(f: fn(i32) -> i32, x: i32) -> i32`.
6. Remove the semicolon trick: write a function whose body ends with a `;` on purpose and read the compiler's suggestion.
7. Write `fn collatz_steps(n: u64) -> u32` using the loop from lesson 5.

## Recap

- Signatures need types; `-> T` declares the return type, no arrow means `()`.
- The last expression with **no semicolon** is the return value; `return` is for early exits.
- Return tuples for several values; there are no defaults or overloads.
- Functions are values of type `fn(A) -> B`.
- `///` documents; examples inside run as tests.

Next: [Ownership, borrowing & slices →](07-ownership-and-borrowing.md)
