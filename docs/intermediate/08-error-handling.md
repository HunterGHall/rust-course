# 8. Error handling in depth

## What & why

Beginner level introduced `Result` and `?`. Real programs add more: several kinds of failure from different libraries, errors that should explain what went wrong and where, and a clear split between **errors you expect** (bad input, missing file) and **bugs** (a broken invariant).

Rust's model:

- **Recoverable errors** → return `Result<T, E>`.
- **Bugs / impossible states** → `panic!` (or `unwrap`/`expect`/`assert!`), which unwinds and ends the thread.

## Panics

```rust
fn main() {
    let v = vec![1, 2, 3];
    // Each of these panics:
    // v[10];                            // index out of bounds
    // None::<i32>.unwrap();             // unwrapping None
    // "x".parse::<i32>().expect("n");   // expect on Err
    // 1 / (v.len() - 3);                // divide by zero
    // panic!("something is badly wrong");
    // assert!(v.len() > 5, "need more than {} items", v.len());
    // unreachable!("this state cannot happen");
    // todo!("not written yet");
    println!("{}", v.len());
}
```

Rule of thumb: use a panic for violated *invariants inside your own code*; use `Result` for anything a caller could reasonably cause or recover from. Libraries should almost never panic on bad input.

## A custom error type

Real error types are enums, one variant per kind of failure, implementing `Display` and `Error`:

```rust
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum ConfigError {
    Missing(String),
    InvalidNumber { key: String, source: ParseIntError },
    OutOfRange { key: String, value: i64 },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::Missing(key) => write!(f, "missing key '{key}'"),
            ConfigError::InvalidNumber { key, source } => write!(f, "'{key}' is not a number: {source}"),
            ConfigError::OutOfRange { key, value } => write!(f, "'{key}' = {value} is out of range"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::InvalidNumber { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn port(text: &str) -> Result<u16, ConfigError> {
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("port="))
        .ok_or_else(|| ConfigError::Missing("port".into()))?;
    let value: i64 = line
        .trim()
        .parse()
        .map_err(|source| ConfigError::InvalidNumber { key: "port".into(), source })?;
    u16::try_from(value).map_err(|_| ConfigError::OutOfRange { key: "port".into(), value })
}

fn main() {
    println!("{:?}", port("host=x\nport=8080"));
    for bad in ["host=x", "port=abc", "port=70000"] {
        match port(bad) {
            Ok(p) => println!("{p}"),
            Err(e) => println!("error: {e}"),
        }
    }
}
```

`Error::source` lets error reporters print the full chain ("failed to load config: port is not a number: invalid digit").

## `From` and automatic `?` conversion

`?` converts the error using `From`. Implement `From<LowerError> for YourError` and `?` just works across layers:

```rust
use std::fmt;
use std::fs;
use std::io;
use std::num::ParseIntError;

#[derive(Debug)]
enum LoadError {
    Io(io::Error),
    Parse(ParseIntError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "I/O error: {e}"),
            LoadError::Parse(e) => write!(f, "parse error: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<io::Error> for LoadError {
    fn from(e: io::Error) -> LoadError { LoadError::Io(e) }
}
impl From<ParseIntError> for LoadError {
    fn from(e: ParseIntError) -> LoadError { LoadError::Parse(e) }
}

fn sum_file(path: &str) -> Result<i64, LoadError> {
    let text = fs::read_to_string(path)?;       // io::Error  -> LoadError via From
    let mut total = 0;
    for line in text.lines() {
        total += line.trim().parse::<i64>()?;   // ParseIntError -> LoadError via From
    }
    Ok(total)
}

fn main() {
    match sum_file("no-such-file.txt") {
        Ok(n) => println!("{n}"),
        Err(e) => println!("{e}"),
    }
}
```

## `Box<dyn Error>`: quick and flexible

For applications (not libraries) that just need to report errors, `Box<dyn Error>` accepts any error and `?` boxes it automatically:

```rust
use std::error::Error;

fn run() -> Result<i32, Box<dyn Error>> {
    let a: i32 = "40".parse()?;                      // ParseIntError -> Box<dyn Error>
    let b: f64 = "1.5".parse()?;                     // ParseFloatError -> Box<dyn Error>
    if b < 0.0 {
        return Err("b must not be negative".into()); // &str -> Box<dyn Error>
    }
    Ok(a + b as i32)
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", run()?);
    Ok(())
}
```

When `main` returns `Err`, Rust prints `Error: ...` (using `Debug`) and exits with code 1.

## `thiserror` and `anyhow`

The two crates nearly every Rust project uses (you'll add crates in lesson 10):

- **`thiserror`** generates the `Display`/`Error`/`From` boilerplate for *library* error enums.
- **`anyhow`** gives an easy `anyhow::Result<T>` for *applications*, with `.context("what we were doing")`.

<!-- check: skip -->
```rust
// With thiserror: the same ConfigError as above, in a few lines.
use thiserror::Error;

#[derive(Debug, Error)]
enum ConfigError {
    #[error("missing key '{0}'")]
    Missing(String),
    #[error("'{key}' is not a number")]
    InvalidNumber { key: String, #[source] source: std::num::ParseIntError },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

<!-- check: skip -->
```rust
// With anyhow, in an application:
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let text = std::fs::read_to_string("config.toml")
        .context("reading config.toml")?;
    let port: u16 = text.trim().parse().context("config must contain a port number")?;
    println!("{port}");
    Ok(())
}
```

Rule of thumb: **libraries return typed errors (`thiserror`); applications use `anyhow`.**

## Converting between `Option` and `Result`

```rust
fn main() {
    let a: Option<i32> = Some(3);
    let r: Result<i32, &str> = a.ok_or("nothing there");          // Option -> Result
    let back: Option<i32> = r.ok();                               // Result -> Option (drops the error)
    println!("{r:?} {back:?}");

    let parsed: Result<i32, _> = "12".parse::<i32>();
    println!("{}", parsed.as_ref().map(|n| n * 2).unwrap_or(0));  // 24
    println!("{:?}", parsed.clone().map_err(|e| e.to_string()));  // Ok(12)

    // Result<Option<T>> <-> Option<Result<T>>
    let x: Option<Result<i32, String>> = Some(Ok(3));
    println!("{:?}", x.transpose());                              // Ok(Some(3))
}
```

## Handling errors at the boundary

A good structure: low-level functions return precise errors; the top level decides what to show and what exit code to use.

<!-- check: norun -->
```rust
use std::process::ExitCode;

#[derive(Debug)]
enum AppError {
    BadInput(String),
}

fn run(args: &[String]) -> Result<u32, AppError> {
    let n = args
        .first()
        .ok_or_else(|| AppError::BadInput("expected a number".into()))?;
    n.parse::<u32>().map_err(|_| AppError::BadInput(format!("'{n}' is not a number")))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(n) => {
            println!("{n}");
            ExitCode::SUCCESS
        }
        Err(AppError::BadInput(message)) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}
```

## Common mistakes

- **`unwrap()` on anything external.** Fine in tests and examples; a crash in production.
- **Stringly-typed errors** (`Result<T, String>`) — callers can't match on the kind. Use an enum.
- **Throwing away the original error** with `map_err(|_| ...)` when the cause would help; keep it in a `source`.
- **Panicking in library code** for bad user input.
- **Converting every error to a message too early.** Keep structured errors until the very edge.
- **Using `Box<dyn Error>` in a public library API** — callers lose the ability to match variants.

## Exercises

1. Define `enum MathError { DivideByZero, Negative(f64) }` and write `safe_sqrt` and `safe_div` returning it.
2. Add `Display` and `Error` impls; call `.to_string()` on each variant.
3. Write a function that reads a file of numbers and sums them, with a custom `Error` enum and `From` impls so `?` works for both I/O and parsing.
4. Rewrite exercise 3 using `Box<dyn Error>` and compare the amount of code.
5. Make the failing parse report the line number (an enum variant carrying `line: usize`).
6. Print the whole chain of an error by walking `source()` in a loop.
7. Convert a `Vec<&str>` into `Result<Vec<i32>, ParseIntError>` with `collect` and explain what happens on the first failure.

## Recap

- `Result` for expected failures, `panic!` for bugs.
- A good error type is an enum implementing `Display` + `Error`, with `From` impls so `?` converts automatically.
- `Box<dyn Error>` for quick applications; `thiserror` for libraries; `anyhow` for applications with context.
- Keep errors structured until the program's edge, then print them and pick an exit code.

Next: [Modules & packages →](09-modules-and-packages.md)
