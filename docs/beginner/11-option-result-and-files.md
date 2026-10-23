# 11. Option, Result & files

## What & why

Rust has no `null` and no exceptions. Instead, two ordinary enums in the standard library represent the two situations where a function might not give you a plain value:

- **`Option<T>`** — there might be no value: `Some(value)` or `None`.
- **`Result<T, E>`** — the operation might fail: `Ok(value)` or `Err(error)`.

Because they're types, the compiler **forces you to handle** the missing or failing case before you can use the value. This one design decision removes most null-pointer crashes and forgotten error checks.

## `Option<T>`

```rust
fn find_first_even(values: &[i32]) -> Option<i32> {
    for &v in values {
        if v % 2 == 0 {
            return Some(v);
        }
    }
    None
}

fn main() {
    println!("{:?}", find_first_even(&[1, 3, 4, 5]));   // Some(4)
    println!("{:?}", find_first_even(&[1, 3, 5]));      // None
}
```

You can't use an `Option<i32>` as an `i32`:

<!-- check: fail -->
```rust
fn main() {
    let x: Option<i32> = Some(5);
    let y = x + 1;     // error[E0369]: cannot add `{integer}` to `Option<i32>`
    println!("{y}");
}
```

You have to say what happens in the `None` case.

### Handling an `Option`

```rust
fn main() {
    let maybe: Option<i32> = Some(7);

    // 1. match — explicit and exhaustive
    match maybe {
        Some(n) => println!("got {n}"),
        None => println!("nothing"),
    }

    // 2. if let — only the Some case
    if let Some(n) = maybe {
        println!("got {n}");
    }

    // 3. let else — bind or leave
    let Some(n) = maybe else {
        return;
    };
    println!("n = {n}");

    // 4. Defaults
    let missing: Option<i32> = None;
    println!("{}", missing.unwrap_or(0));                 // 0
    println!("{}", missing.unwrap_or_else(|| 40 + 2));    // 42
    println!("{}", missing.unwrap_or_default());          // 0

    // 5. Transform without unwrapping
    println!("{:?}", maybe.map(|n| n * 2));               // Some(14)
    println!("{:?}", maybe.filter(|n| *n > 10));          // None
    println!("{:?}", maybe.and_then(|n| n.checked_mul(1_000_000_000)));  // None (overflow)
    println!("{:?}", missing.or(Some(1)));                // Some(1)
    println!("{}", maybe.is_some());                      // true
}
```

### `unwrap` and `expect`

`unwrap()` returns the value or **panics** (crashes the program). `expect("message")` does the same with your message. They're fine in quick experiments, in tests, and when `None` is truly impossible — and a code smell everywhere else.

<!-- check: wrap panics -->
```rust
let nothing: Option<i32> = None;
nothing.expect("the config must have a port");   // panics with that message
```

### `?` on `Option`

Inside a function that itself returns an `Option`, `?` means "if `None`, return `None` now; otherwise unwrap":

```rust
fn first_char_upper(text: &str) -> Option<char> {
    let c = text.chars().next()?;          // return None early if the text is empty
    Some(c.to_ascii_uppercase())
}

fn main() {
    println!("{:?} {:?}", first_char_upper("rust"), first_char_upper(""));   // Some('R') None
}
```

## `Result<T, E>`

```rust
use std::num::ParseIntError;

fn parse_age(text: &str) -> Result<u8, ParseIntError> {
    text.trim().parse::<u8>()
}

fn main() {
    println!("{:?}", parse_age("42"));       // Ok(42)
    println!("{:?}", parse_age("abc"));      // Err(ParseIntError { kind: InvalidDigit })
    println!("{:?}", parse_age("300"));      // Err(... PosOverflow ...)

    match parse_age("-5") {
        Ok(age) => println!("age {age}"),
        Err(e) => println!("bad age: {e}"),  // bad age: invalid digit found in string
    }
}
```

`Result` has the same helpers as `Option`: `map`, `unwrap_or`, `unwrap_or_default`, `ok()` (turn into an `Option`), `is_ok()`, `is_err()`, and `map_err` to change the error type.

### The `?` operator

`?` is the workhorse of Rust error handling. On an `Err`, it **returns the error from the current function**; on `Ok`, it gives you the value:

```rust
use std::num::ParseIntError;

fn sum_two(a: &str, b: &str) -> Result<i32, ParseIntError> {
    let x: i32 = a.parse()?;       // return early if this fails
    let y: i32 = b.parse()?;
    Ok(x + y)
}

fn main() {
    println!("{:?}", sum_two("3", "4"));      // Ok(7)
    println!("{:?}", sum_two("3", "four"));   // Err(...)
}
```

`?` only works in functions that return a compatible `Result` (or `Option`). That's why `main` is allowed to return `Result`:

```rust
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let n: i32 = "42".parse()?;
    println!("{n}");
    Ok(())
}
```

`Box<dyn Error>` means "any error". It's the quick option for small programs; Intermediate lesson 8 shows how to define proper error types.

## Files and paths

Reading and writing files returns `io::Result`, so the same tools apply:

```rust
use std::fs;
use std::io;

fn main() -> io::Result<()> {
    fs::write("hello.txt", "first line\nsecond line\n")?;

    let text = fs::read_to_string("hello.txt")?;
    for (i, line) in text.lines().enumerate() {
        println!("{}: {line}", i + 1);
    }

    fs::remove_file("hello.txt")?;
    Ok(())
}
```

Handling the "doesn't exist" case specifically:

```rust
use std::fs;
use std::io::ErrorKind;

fn main() {
    match fs::read_to_string("does-not-exist.txt") {
        Ok(text) => println!("{} bytes", text.len()),
        Err(e) if e.kind() == ErrorKind::NotFound => println!("no such file (that's fine)"),
        Err(e) => println!("real problem: {e}"),
    }
}
```

Appending, and working with paths:

```rust
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

fn main() -> io::Result<()> {
    let path = Path::new("log.txt");

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "an appended line")?;

    println!("{}", path.exists());                       // true
    println!("{:?}", path.extension());                  // Some("txt")
    println!("{:?}", path.file_stem());                  // Some("log")
    let joined = Path::new("data").join("notes.md");     // portable path building
    println!("{}", joined.display());

    fs::remove_file(path)?;
    Ok(())
}
```

Use `Path` / `PathBuf` (not plain strings) for file paths — they handle Windows vs Unix separators, and not every path is valid UTF-8.

## Command-line arguments and environment

```rust
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    // args[0] is the program name.
    println!("{} argument(s) after the program name", args.len() - 1);

    for arg in env::args().skip(1) {
        println!("arg: {arg}");
    }

    match env::var("HOME") {
        Ok(home) => println!("home = {home}"),
        Err(_) => println!("HOME is not set"),
    }
}
```

Run with `cargo run -- one two` — everything after `--` goes to your program.

## Exit codes

```rust
use std::process::ExitCode;

fn main() -> ExitCode {
    let ok = "42".parse::<i32>().is_ok();
    if ok {
        ExitCode::SUCCESS
    } else {
        eprintln!("something failed");
        ExitCode::FAILURE       // exit status 1
    }
}
```

Tools like shells, `make` and CI decide success or failure from the exit code, so a command-line program should return non-zero on error. Print messages meant for humans-in-trouble to **standard error** (`eprintln!`), not standard output.

## Common mistakes

- **`unwrap()` on user input or file operations.** It turns a recoverable error into a crash.
- **Ignoring a `Result`.** The compiler warns (`unused_must_use`) — don't silence it; handle or propagate it.
- **Using `?` in `main` without a `Result` return type.** Give `main` a return type.
- **Discarding the error detail** with `.ok()` or `_` when the user needs to know what failed.
- **Building file paths by string concatenation** instead of `Path::join`.
- **Treating `NotFound` as a failure** when it simply means "first run".
- **Printing errors to stdout** instead of stderr.

## Exercises

1. Write `fn divide(a: i32, b: i32) -> Option<i32>` that returns `None` when `b == 0`; call it with and without a zero divisor.
2. Write `fn parse_pair(text: &str) -> Option<(i32, i32)>` for input like `"3,4"` using `split_once` and `?`.
3. Write `fn read_number(prompt: &str) -> Result<i32, std::num::ParseIntError>` that reads a line from stdin.
4. Write a program that reads a file given as the first argument and prints its line count; handle the missing-file case with a friendly message and a non-zero exit code.
5. Write `fn average(path: &str) -> Result<f64, Box<dyn std::error::Error>>` that reads numbers (one per line) from a file and returns their mean; use `?` throughout.
6. Chain `Option` helpers: given `Some("  42 ")`, produce `Some(42)` using `map`, `trim`, and `and_then` with `parse().ok()`.
7. Write a program that appends a timestamp-free line to `journal.txt` each time it's run, then prints the whole file.

## Recap

- No null, no exceptions: `Option<T>` for "maybe a value", `Result<T, E>` for "maybe an error".
- Handle with `match`, `if let`, `let else`, or helpers (`map`, `unwrap_or`, `and_then`, `ok`, `map_err`).
- `?` propagates `None`/`Err` to the caller; `main` can return `Result`.
- `unwrap`/`expect` panic — use them only when failure is truly impossible or in throwaway code.
- `std::fs`, `Path`, `env::args`, and `ExitCode` cover small command-line programs.

Next: [Capstone: task tracker →](12-capstone-task-tracker.md)
