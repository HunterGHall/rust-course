# The Rust Course

A complete, self-paced Rust curriculum in Markdown, from your first `println!` to ownership, traits, lifetimes, macros, `unsafe`, async and publishing a crate. Work through it top to bottom, or jump to the level that matches your experience.

## How this course is organized

| Folder | For you if... | Outcome |
| --- | --- | --- |
| [Beginner](beginner/index.md) | You have never written Rust (or never finished a course) | Write and run real programs, understand ownership and borrowing, build a command-line app that saves data |
| [Intermediate](intermediate/index.md) | You know the syntax but structs vs enums, traits, generics and lifetimes still feel shaky | Model data with types, generic and trait-based code, iterators, smart pointers, errors, modules, crates, testing, a real project |
| [Advanced](advanced/index.md) | You ship Rust and want depth | Advanced traits and type-level design, variance and `Pin`, macros, threads, async, `unsafe`, FFI, performance, publishing, a concurrent capstone |

Each lesson is one file and follows the same shape:

1. **What & why** – the idea in plain language
2. **Examples** – code you should type out yourself
3. **Common mistakes** – the traps everyone hits
4. **Exercises** – do these; reading is not learning
5. **Recap**

Each level ends with a **capstone** project. The complete, buildable source for all three lives in the [`capstones/`](https://github.com/HunterGHall/rust-course/tree/main/capstones) folder of the repository, and CI builds and tests them on Linux, macOS and Windows. CI also **compiles every Rust code block in the lessons**, so the examples are known to build.

## Setup (do this first)

You need **Rust 1.85 or newer** (the 2024 edition). It runs on Windows, macOS and Linux.

```bash
rustc --version
cargo --version
```

If those fail, install the toolchain with **rustup**:

- **Windows**: `winget install Rustlang.Rustup`, then open a new terminal (you'll also need the Visual Studio Build Tools with the C++ workload)
- **macOS**: `xcode-select --install`, then `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Linux**: install a C compiler (`build-essential`), then the same `curl` command. See [rustup.rs](https://rustup.rs)

Keep it current with `rustup update`.

A good editor helps a lot: [VS Code](https://code.visualstudio.com/) with the **rust-analyzer** extension (it shows inferred types and compiler errors as you type), or [RustRover](https://www.jetbrains.com/rust/).

Every example in this course is meant to be run either as a one-file program or inside a Cargo project:

```bash
rustc scratch.rs && ./scratch      # compile and run a single file
cargo new myproject                # create a project
cargo run                          # build and run it
cargo test                         # run its tests
cargo check                        # type-check quickly while you work
```

## How to study

- **Type every example.** Copy-paste teaches you nothing.
- **Read the compiler errors.** Rust's errors are detailed and often include the fix; start at the *first* one.
- **Break things on purpose.** Change a line, predict the result, compile, run.
- **Do the exercises before looking anything up.**
- **Don't fight the borrow checker — listen to it.** When it complains, it has found a real aliasing or lifetime problem; the lesson on ownership explains how to think about it.
- **One lesson per sitting** is plenty. Consistency beats marathons.

## Curriculum at a glance

### Beginner
1. [Setup & your first program](beginner/01-setup-and-first-program.md)
2. [Variables, mutability & types](beginner/02-variables-and-types.md)
3. [Numbers & math](beginner/03-numbers-and-math.md)
4. [Booleans & conditionals](beginner/04-booleans-and-conditionals.md)
5. [Loops](beginner/05-loops.md)
6. [Functions](beginner/06-functions.md)
7. [Ownership, borrowing & slices](beginner/07-ownership-and-borrowing.md)
8. [Strings](beginner/08-strings.md)
9. [Arrays, vectors & slices](beginner/09-vectors-and-slices.md)
10. [HashMaps & sets](beginner/10-hashmaps-and-sets.md)
11. [Option, Result & files](beginner/11-option-result-and-files.md)
12. [Capstone: command-line task tracker](beginner/12-capstone-task-tracker.md)

### Intermediate
1. [Structs & methods](intermediate/01-structs-and-methods.md)
2. [Enums & pattern matching](intermediate/02-enums-and-pattern-matching.md)
3. [Traits](intermediate/03-traits.md)
4. [Generics](intermediate/04-generics.md)
5. [Lifetimes](intermediate/05-lifetimes.md)
6. [Closures & iterators](intermediate/06-closures-and-iterators.md)
7. [Smart pointers](intermediate/07-smart-pointers.md)
8. [Error handling in depth](intermediate/08-error-handling.md)
9. [Modules & packages](intermediate/09-modules-and-packages.md)
10. [Dependencies & serde](intermediate/10-dependencies-and-serde.md)
11. [Testing](intermediate/11-testing.md)
12. [Capstone: Markdown link checker](intermediate/12-capstone-link-checker.md)

### Advanced
1. [Advanced traits & dispatch](advanced/01-advanced-traits.md)
2. [Advanced generics & type-level design](advanced/02-advanced-generics.md)
3. [Advanced lifetimes, variance & Pin](advanced/03-advanced-lifetimes.md)
4. [Macros](advanced/04-macros.md)
5. [Concurrency with threads](advanced/05-threads-and-shared-state.md)
6. [Async Rust](advanced/06-async-fundamentals.md)
7. [Async patterns with Tokio](advanced/07-async-patterns-with-tokio.md)
8. [Unsafe Rust](advanced/08-unsafe-rust.md)
9. [FFI & C interoperability](advanced/09-ffi-and-c-interop.md)
10. [Performance & profiling](advanced/10-performance.md)
11. [Publishing, API design & distribution](advanced/11-publishing-and-api-design.md)
12. [Capstone: parallel file indexer](advanced/12-capstone-parallel-indexer.md)

## License

Use freely for learning and teaching.
