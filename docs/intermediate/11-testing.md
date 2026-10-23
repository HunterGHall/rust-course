# 11. Testing

## What & why

Rust treats testing as a built-in feature: no framework to install, no configuration. `cargo test` finds and runs every test in your project, including the code examples in your documentation. Tests are also where you cash in everything the type system gives you — functions that take plain values and return plain values are trivially testable.

## Unit tests

Put them in the same file as the code, inside a `tests` module that only compiles for `cargo test`:

```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err("division by zero".to_string())
    } else {
        Ok(a / b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;          // bring the parent module's items into scope

    #[test]
    fn adds_two_numbers() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn divides() {
        assert_eq!(divide(10, 2), Ok(5));
    }

    #[test]
    fn rejects_zero() {
        assert_eq!(divide(1, 0), Err("division by zero".to_string()));
    }
}
```

Run them:

```bash
cargo test
```

```
running 3 tests
test tests::adds_two_numbers ... ok
test tests::divides ... ok
test tests::rejects_zero ... ok

test result: ok. 3 passed; 0 failed; 0 ignored
```

A test **passes** if it returns normally and **fails** if it panics. `#[cfg(test)]` means the module (and its imports) is compiled only when testing. Because the test module is a child of the code under test, it can reach **private** functions through `super::*` — so you can test internals without making them public.

## Assertions

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn assertion_tour() {
        assert!(2 + 2 == 4);                                  // a boolean
        assert!(1 < 2, "math is broken: {} !< {}", 1, 2);     // with a custom message
        assert_eq!(vec![1, 2], [1, 2]);                       // equality; shows both sides on failure
        assert_ne!("a", "b");                                 // inequality
        assert!(matches!(Some(3), Some(n) if n > 2));         // pattern match

        let x = 0.1 + 0.2;
        assert!((x - 0.3_f64).abs() < 1e-9, "floats differ: {x}");   // floats: use a tolerance
    }
}
```

`assert_eq!` needs `PartialEq + Debug` on the compared types — another reason to `#[derive(Debug, PartialEq)]` on your structs and enums.

`debug_assert!` variants are only checked in debug builds — handy for expensive invariants inside real code.

## Testing for panics and errors

```rust
pub fn first(items: &[i32]) -> i32 {
    items[0]
}

pub fn parse(text: &str) -> Result<i32, std::num::ParseIntError> {
    text.trim().parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn empty_slice_panics() {
        first(&[]);
    }

    // Tests can return Result and use `?` — an Err fails the test.
    #[test]
    fn parses_with_question_mark() -> Result<(), std::num::ParseIntError> {
        let n = parse(" 42 ")?;
        assert_eq!(n, 42);
        Ok(())
    }

    #[test]
    fn bad_input_is_an_error() {
        assert!(parse("abc").is_err());
    }

    #[test]
    #[ignore = "slow; run with `cargo test -- --ignored`"]
    fn expensive() {
        // ...
    }
}
```

## Running subsets

```bash
cargo test                       # everything
cargo test parse                 # only tests whose name contains "parse"
cargo test -- --nocapture        # show println! output from passing tests
cargo test -- --test-threads=1   # run tests one at a time
cargo test -- --ignored          # only the #[ignore]d tests
cargo test --lib                 # unit tests only
cargo test --test links          # one integration test file
cargo test -p linkcheck          # one package in a workspace
```

Tests run **in parallel** on separate threads and in no fixed order, so they must not depend on each other or share files carelessly.

## Integration tests

Files in the top-level `tests/` directory are compiled as separate crates that use your library through its public API only — exactly like a user would:

```
linkcheck/
├── src/
│   ├── lib.rs
│   └── links.rs
└── tests/
    ├── links.rs          # each file is its own test crate
    └── common/
        └── mod.rs        # shared helpers (a subdirectory isn't treated as a test)
```

<!-- check: skip -->
```rust
// tests/links.rs
use linkcheck::{classify, LinkKind};

#[test]
fn urls_are_external() {
    assert_eq!(classify("https://rust-lang.org"), LinkKind::Url);
}
```

This is why the library + binary split from lesson 9 matters: integration tests can only call a *library* crate, not a binary's `main`.

## Doc tests

Code in documentation comments runs as a test, so examples can't rot:

```rust
/// Doubles a number.
///
/// ```
/// use mylib::double;
/// assert_eq!(double(21), 42);
/// ```
pub fn double(n: i32) -> i32 {
    n * 2
}

fn main() {}
```

Hide setup lines from the rendered docs by starting them with `# `:

```rust
/// ```
/// # use mylib::double;      // compiled, but not shown in the docs
/// assert_eq!(double(2), 4);
/// ```
pub fn double(n: i32) -> i32 { n * 2 }

fn main() {}
```

Tag a block ```` ```no_run ```` to compile but not run it, ```` ```ignore ```` to skip it, or ```` ```should_panic ````.

## Test helpers and fixtures

Setup/teardown has no special syntax; use ordinary functions and `Drop` for cleanup:

```rust
use std::fs;
use std::path::PathBuf;

/// A temporary directory that deletes itself when it goes out of scope.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!("demo-{name}-{}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);     // runs even if the test panicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_reads_a_file() {
        let dir = TempDir::new("rw");
        let file = dir.0.join("hello.txt");
        fs::write(&file, "hi").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "hi");
    }
}

fn main() {}
```

(The `tempfile` crate does this properly. Using `std::process::id()` in the name keeps parallel test runs from colliding.)

## Designing for testability

Notice what made the capstones easy to test:

- **Pure functions** (`extract_links(&str) -> Vec<Link>`) take data and return data. No mocks needed.
- **I/O pushed to the edges.** `main` reads files and prints; the library takes `&str` and returns values.
- **Dependencies passed in** rather than reached for globally (a path, a reader, a closure).
- **Errors as values**, so tests can assert on exactly which failure occurred.

When code is hard to test, it's usually telling you its responsibilities are tangled.

## Beyond the basics

- **Property-based testing** (`proptest`, `quickcheck`): generate thousands of random inputs and check properties like "parse(format(x)) == x".
- **Snapshot testing** (`insta`): compare output to a stored file.
- **Benchmarks** (`criterion`): measure performance reliably (Advanced lesson 10).
- **Code coverage**: `cargo install cargo-llvm-cov`, then `cargo llvm-cov`.
- **Miri**: `cargo +nightly miri test` detects undefined behaviour in `unsafe` code (Advanced lesson 8).
- **Fuzzing** (`cargo fuzz`): feed garbage to your parsers.

## Common mistakes

- **Forgetting `#[cfg(test)]`** on the test module, so the test code ships in your binary.
- **Tests that depend on run order or shared files.** They run in parallel.
- **`assert!(a == b)`** instead of `assert_eq!(a, b)` — you lose the helpful diff.
- **Comparing floats exactly.**
- **Not deriving `Debug`/`PartialEq`** and then being unable to use `assert_eq!`.
- **Testing implementation details** instead of behaviour, which makes refactoring painful.
- **Using `unwrap` everywhere in tests** — fine, but a returned `Result` with `?` gives better messages.

## Exercises

1. Write a `fn is_leap_year(year: u32) -> bool` and test the cases 1900, 2000, 2024 and 2023.
2. Write `fn clamp(x: i32, lo: i32, hi: i32) -> i32` and a test for each boundary. Add a `#[should_panic]` test for `lo > hi`.
3. Write `fn parse_kv(line: &str) -> Option<(&str, &str)>` with at least five tests, including an empty line and a line with two `=`.
4. Add a doc test to one of your functions and run `cargo test --doc`.
5. Convert a binary project into library + binary, then add a test in `tests/`.
6. Write a test that uses `TempDir` to create two files and checks that your directory-walking function finds them sorted.
7. Intentionally break an `assert_eq!` and read the failure output.

## Recap

- `#[test]` functions run under `cargo test`; they pass unless they panic (or return `Err`).
- Unit tests live next to the code in `#[cfg(test)] mod tests`; integration tests live in `tests/` and use only the public API; doc tests are examples in `///` comments.
- Use `assert!`, `assert_eq!`, `assert_ne!`, `#[should_panic]`, and tolerances for floats.
- Tests run in parallel, so isolate their files and state.
- Pure functions with I/O pushed to the edges are the easiest code to test.

Next: [Capstone: Markdown link checker →](12-capstone-link-checker.md)
