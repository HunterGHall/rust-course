# 1. Setup & your first program

## What & why

Rust is a compiled language focused on three things at once: speed like C and C++, memory safety without a garbage collector, and fearless concurrency. The price is that the compiler is strict. The reward is that when a Rust program compiles, a whole class of bugs (use-after-free, data races, null dereferences) simply cannot be in it.

You'll use two tools every day:

- **`rustc`** — the compiler.
- **`cargo`** — the build tool, package manager, test runner and project scaffolder. You will almost never call `rustc` directly.

Both are installed by **rustup**, the Rust toolchain manager.

## Install

```bash
rustc --version      # rustc 1.85.0 or newer is what this course needs
cargo --version
```

If those fail, install from [rustup.rs](https://rustup.rs):

- **Windows**: `winget install Rustlang.Rustup`, then open a new terminal. You also need the *Visual Studio Build Tools* with the "Desktop development with C++" workload (rustup will tell you).
- **macOS**: `xcode-select --install`, then `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Linux**: install a C compiler (`build-essential` on Debian/Ubuntu), then the same `curl` command.

Keep it current with `rustup update`. A good editor setup is [VS Code](https://code.visualstudio.com/) with the **rust-analyzer** extension — it shows types inline and runs the compiler as you type.

## Hello, world

Create a file `hello.rs`:

```rust
fn main() {
    println!("Hello, world!");
}
```

Compile and run it:

```bash
rustc hello.rs
./hello          # on Windows: .\hello.exe
```

Three things to notice:

- `fn main()` is where every program starts.
- `println!` ends in `!` because it is a **macro**, not a function. Macros can take a variable number of arguments, which ordinary functions can't.
- Statements end in `;`, and blocks use `{ }`.

## Your first Cargo project

Real programs use Cargo:

```bash
cargo new hello-cargo
cd hello-cargo
```

That creates:

```
hello-cargo/
├── Cargo.toml     # name, version, edition, dependencies
├── .gitignore
└── src/
    └── main.rs    # the entry point
```

`Cargo.toml`:

```toml
[package]
name = "hello-cargo"
version = "0.1.0"
edition = "2024"

[dependencies]
```

The **edition** is how Rust evolves without breaking old code: each package picks one (2015, 2018, 2021, 2024), and crates of different editions work together. Use the newest for new code.

The commands you will run constantly:

| Command | What it does |
| --- | --- |
| `cargo run` | build (if needed) and run |
| `cargo build` | just build, into `target/debug/` |
| `cargo build --release` | build with optimizations, into `target/release/` |
| `cargo check` | type-check only — much faster than a full build |
| `cargo test` | run the tests |
| `cargo fmt` | format your code the standard way |
| `cargo clippy` | the linter; it teaches idiomatic Rust |

!!! tip
    Use `cargo check` in a loop while you work. It finds compile errors in a fraction of the time a full build takes.

## `println!` and formatting

```rust
fn main() {
    let name = "Ferris";
    let year = 2026;

    println!("Hello, {name}!");                    // inline variable names
    println!("{} was started in {}", name, year);  // positional {}
    println!("{0} {1} {0}", "a", "b");             // a b a
    println!("{:>8}|{:<8}|{:^8}|", "r", "l", "c"); // align: right, left, center
    println!("{:05}", 42);                          // 00042
    println!("{:.2}", 3.14159);                     // 3.14
    println!("{:?}", "debug");                      // "debug" (with quotes)
    println!("{{literal braces}}");                 // {literal braces}

    print!("no newline, ");
    println!("then a newline");
    eprintln!("this goes to standard error");
}
```

`{}` uses the **Display** trait (human-readable). `{:?}` uses **Debug** (programmer-readable, works on almost everything). You'll meet traits properly in Intermediate; for now just know both exist.

## Reading input

```rust
use std::io;

fn main() {
    println!("What's your name?");

    let mut name = String::new();
    io::stdin()
        .read_line(&mut name)
        .expect("failed to read line");

    println!("Hello, {}!", name.trim());
}
```

Three new ideas, all explained properly later:

- `let mut name` — variables are immutable unless you say `mut` (next lesson).
- `&mut name` — lend `name` out so `read_line` can modify it (lesson 7).
- `.expect(...)` — `read_line` returns a `Result` that might be an error; `expect` crashes with your message if it is (lesson 11).

`read_line` keeps the trailing newline, hence the `trim()`.

## Comments

```rust
// A line comment.

/* A block comment,
   which can span lines. */

/// A doc comment: becomes documentation for the item below it.
fn documented() {}

fn main() {
    documented();
}
```

## Reading compiler errors

Rust's compiler messages are famously good. Break something on purpose:

<!-- check: fail -->
```rust
fn main() {
    let x = 5;
    x = 6;
    println!("{x}");
}
```

```
error[E0384]: cannot assign twice to immutable variable `x`
 --> src/main.rs:3:5
  |
2 |     let x = 5;
  |         - first assignment to `x`
3 |     x = 6;
  |     ^^^^^ cannot assign twice to immutable variable
  |
help: consider making this binding mutable
  |
2 |     let mut x = 5;
  |         +++
```

It tells you what's wrong, where, and often the exact fix. **Read the first error first** — later ones are frequently fallout from it. For more detail run `rustc --explain E0384`.

## Common mistakes

- **Forgetting the `!`** in `println!` — you'll get "expected function, found macro".
- **Forgetting `;`** or adding one where a value is expected (lesson 6 explains the difference).
- **Running `rustc` on a Cargo project.** Inside a project, use `cargo run`.
- **Benchmarking a debug build.** `cargo run` is unoptimized and can be 10–100× slower than `cargo run --release`.
- **Editing in the wrong folder.** Cargo commands must run inside the project (where `Cargo.toml` lives).
- **Ignoring warnings.** Warnings are almost always telling you something true.

## Exercises

1. Install Rust and run `rustc --version` and `cargo --version`.
2. Make a new project with `cargo new greet`, change it to print your name, and run it with `cargo run`.
3. Print a small ASCII-art box using several `println!` calls.
4. Ask for the user's name *and* age (two prompts) and print a sentence using both.
5. Use `{:>10}` and `{:<10}` to print a two-column table of three items and prices.
6. Introduce three different errors on purpose (a typo, a missing `;`, a wrong type) and read each message.
7. Run `cargo build` and `cargo build --release`, then look inside `target/` to see where each binary went.

## Recap

- `rustup` installs the toolchain; `cargo` drives everything.
- `fn main()` is the entry point; `println!` is a macro (note the `!`).
- `cargo run`, `cargo check`, `cargo test`, `cargo fmt`, `cargo clippy` are your daily tools.
- `{}` is Display, `{:?}` is Debug; `{name}` interpolates a variable.
- Read compiler errors top to bottom — they're written to help.

Next: [Variables, mutability & types →](02-variables-and-types.md)
