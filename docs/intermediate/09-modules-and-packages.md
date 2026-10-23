# 9. Modules & packages

## What & why

As programs grow you need to split them into files, hide implementation details, and reuse code between programs. Rust organizes this with a small set of terms that are easy to mix up, so here they are once:

| Term | What it is |
| --- | --- |
| **Package** | what `cargo new` creates: a `Cargo.toml` plus one or more crates |
| **Crate** | one compilation unit — a **binary** (has `main`) or a **library** |
| **Module** | a namespace inside a crate; controls privacy and paths |
| **Path** | how you name an item: `crate::geometry::area` |

## Modules in one file

```rust
mod geometry {
    pub const PI: f64 = 3.14159;

    pub fn circle_area(radius: f64) -> f64 {
        PI * radius * radius
    }

    fn helper() -> f64 {            // private: only visible inside `geometry`
        1.0
    }

    pub mod shapes {
        pub struct Square {
            pub side: f64,          // public field
            id: u32,                // private field
        }

        impl Square {
            pub fn new(side: f64) -> Square {
                Square { side, id: super::next_id() }   // super:: goes up one module
            }
            pub fn id(&self) -> u32 { self.id }
        }
    }

    fn next_id() -> u32 { 7 }
}

fn main() {
    println!("{}", geometry::circle_area(2.0));
    let sq = geometry::shapes::Square::new(3.0);
    println!("{} {}", sq.side, sq.id());
    // geometry::helper();            // error: function `helper` is private
    // sq.id;                         // error: field `id` is private
}
```

Everything is **private by default**: visible inside the module where it's defined (and its children), invisible outside. Add `pub` to expose it. Privacy is what lets a module guarantee its own invariants (lesson 1).

Visibility levels: `pub` (everyone), `pub(crate)` (anywhere in this crate), `pub(super)` (the parent module), `pub(in path)`, and nothing (private).

## Paths and `use`

```rust
mod network {
    pub mod server {
        pub fn start() -> &'static str { "started" }
    }
    pub fn ping() -> &'static str { "pong" }
}

use network::server;                       // bring a module into scope
use network::server::start as launch;     // rename
use network::{ping, server as srv};       // several at once

fn main() {
    println!("{}", server::start());
    println!("{}", launch());
    println!("{} {}", ping(), srv::start());
    println!("{}", crate::network::ping());   // absolute path from the crate root
}
```

Conventions:

- Import **modules** for functions (`use std::fs; fs::read_to_string(...)`) so the origin is clear at the call site.
- Import **types** directly (`use std::collections::HashMap;`).
- `use std::io::{self, Write};` imports `io` and `Write` together.
- Glob imports (`use module::*`) are mostly for preludes and tests.

`pub use` **re-exports**, letting you flatten an internal layout into a tidy public API:

```rust
mod internal {
    pub mod parser {
        pub fn parse(s: &str) -> usize { s.len() }
    }
}

pub use internal::parser::parse;     // users call crate::parse, not crate::internal::parser::parse

fn main() {
    println!("{}", parse("hello"));
}
```

## Modules in files

Real projects put modules in separate files. For a binary crate:

```
my-app/
├── Cargo.toml
└── src/
    ├── main.rs          # crate root
    ├── config.rs        # mod config
    ├── network.rs       # mod network
    └── network/
        ├── client.rs    # mod network::client
        └── server.rs    # mod network::server
```

In `src/main.rs` you **declare** the modules:

<!-- check: skip -->
```rust
mod config;      // the compiler looks for src/config.rs or src/config/mod.rs
mod network;     // src/network.rs (+ submodules in src/network/)

fn main() {
    config::load();
    network::server::run();
}
```

and in `src/network.rs` you declare *its* children:

<!-- check: skip -->
```rust
pub mod client;    // src/network/client.rs
pub mod server;    // src/network/server.rs
```

Key point: `mod name;` is the only thing that adds a file to the build. A `.rs` file nobody declared with `mod` is simply ignored. (The older `mod.rs` layout — `src/network/mod.rs` instead of `src/network.rs` — still works but the newer style avoids a sea of identically named files.)

## Library + binary in one package

The most useful structure for anything you want to test is a thin binary over a library:

```
linkcheck/
├── Cargo.toml
├── src/
│   ├── lib.rs        # the library crate: all the logic
│   ├── checker.rs
│   ├── links.rs
│   └── main.rs       # the binary crate: argument parsing and printing only
└── tests/
    └── links.rs      # integration tests use the library like any other user
```

`src/main.rs` imports the library by the package name:

<!-- check: skip -->
```rust
use linkcheck::{check_file, find_markdown};
```

The intermediate capstone uses exactly this layout. Benefits: logic is testable without spawning processes, and other programs can depend on your library.

## Workspaces

Several related packages can share one `Cargo.lock` and `target/` directory:

```toml
# Cargo.toml at the repository root
[workspace]
members = ["app", "core", "tools/*"]
resolver = "3"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
```

```toml
# app/Cargo.toml
[package]
name = "app"
version = "0.1.0"
edition = "2024"

[dependencies]
core = { path = "../core" }        # a local path dependency
serde = { workspace = true }       # inherit the shared version
```

`cargo build` at the root builds everything; `cargo test -p core` targets one package; `cargo run -p app` runs a binary.

## Conditional compilation

```rust
#[cfg(target_os = "windows")]
fn platform() -> &'static str { "windows" }

#[cfg(target_os = "linux")]
fn platform() -> &'static str { "linux" }

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn platform() -> &'static str { "something else" }

fn main() {
    println!("{}", platform());
    if cfg!(debug_assertions) {
        println!("debug build");
    }
}
```

Only the matching `platform` is compiled; the others don't even need to be valid on your OS. Cargo **features** (lesson 10 and Advanced 11) use the same mechanism: `#[cfg(feature = "json")]`.

## Naming conventions

- Crates, modules, functions, variables, files: `snake_case`
- Types, traits, enum variants: `UpperCamelCase`
- Constants and statics: `SCREAMING_SNAKE_CASE`
- Package names on crates.io often use `kebab-case`; inside code the crate name becomes `snake_case` (`my-app` → `my_app`).

`cargo fmt` and `cargo clippy` will nudge you.

## Common mistakes

- **Forgetting `mod foo;`** in the parent — then the file is never compiled and `use foo::...` fails with "unresolved import".
- **Forgetting `pub`** on an item or on a struct *field*. Making a struct `pub` doesn't make its fields public.
- **Mixing up `mod` and `use`.** `mod` defines/loads a module; `use` only shortens a path to something that already exists.
- **Cyclic thinking.** Modules can refer to each other freely within a crate (unlike packages, which can't have dependency cycles).
- **Putting all logic in `main.rs`** — untestable. Move it to `lib.rs`.
- **Writing `use crate::` in a binary to import from the library crate** — the library is a separate crate, referred to by package name (`use linkcheck::...`).

## Exercises

1. Create a module `temperature` with `pub fn c_to_f` and `pub fn f_to_c`, and call them from `main`.
2. Create a `bank` module with a `pub struct Account` whose balance is private and has `deposit`/`balance` methods. Confirm `account.balance = 5` doesn't compile outside the module.
3. Split exercise 2 into `src/bank.rs` and declare it with `mod bank;`.
4. Add `pub(crate)` to a helper that should be shared between your modules but not exposed to library users.
5. Re-export two deeply nested functions at the top of your `lib.rs` with `pub use`.
6. Turn a binary project into library + binary and add one integration test in `tests/`.
7. Create a workspace with two packages, one depending on the other by path.

## Recap

- Package ⊃ crate ⊃ module. `cargo new` makes a package; `src/main.rs` and `src/lib.rs` are crate roots.
- Items are private by default; `pub`, `pub(crate)`, `pub(super)` widen visibility. Struct fields have their own `pub`.
- `mod name;` declares a file-backed module; `use` shortens paths; `pub use` re-exports.
- A thin binary over a testable library is the standard structure.
- Workspaces share a lock file and build directory across packages.

Next: [Dependencies & serde →](10-dependencies-and-serde.md)
