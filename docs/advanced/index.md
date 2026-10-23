# Advanced

For you if you ship Rust and want depth. This level covers the parts of the language and ecosystem that separate "it compiles" from "it's fast, safe, and well designed": the machinery behind traits, type-level design, lifetimes and variance, macros, threads and async, unsafe code and FFI, performance, and publishing.

## Lessons

| # | Lesson | You'll learn |
| --- | --- | --- |
| 1 | [Advanced traits & dispatch](01-advanced-traits.md) | static vs dynamic dispatch, dyn compatibility, blanket impls, extension and sealed traits, GATs, `Any` |
| 2 | [Advanced generics & type-level design](02-advanced-generics.md) | typed IDs, typestate, builders, associated consts, `const fn`, const generics, compile-time costs |
| 3 | [Advanced lifetimes, variance & Pin](03-advanced-lifetimes.md) | outlives, subtyping, variance, `for<'a>`, `'static` bounds, trait-object lifetimes, `Pin` |
| 4 | [Macros](04-macros.md) | `macro_rules!`, repetition, hygiene, tt-munchers, derive/attribute/function-like proc macros |
| 5 | [Concurrency with threads](05-threads-and-shared-state.md) | `Send`/`Sync`, scoped threads, channels, `Mutex`, `RwLock`, atomics, `Condvar`, `rayon` |
| 6 | [Async Rust](06-async-fundamentals.md) | futures as state machines, `poll`/`Waker`, writing an executor, `Send` across `.await` |
| 7 | [Async patterns with Tokio](07-async-patterns-with-tokio.md) | `spawn`, `select!`, timeouts, cancellation, channels, semaphores, streams, a TCP server |
| 8 | [Unsafe Rust](08-unsafe-rust.md) | the five superpowers, UB, raw pointers, safe abstractions, a `Vec` from scratch, Miri |
| 9 | [FFI & C interoperability](09-ffi-and-c-interop.md) | `extern "C"`, `repr(C)`, `CString`, callbacks, exporting Rust, `bindgen`/`cbindgen` |
| 10 | [Performance & profiling](10-performance.md) | release profiles, benchmarking, allocation, layout and cache, flame graphs |
| 11 | [Publishing, API design & distribution](11-publishing-and-api-design.md) | API guidelines, rustdoc, SemVer, features, crates.io, cross-compiling, releases |
| 12 | [Capstone: parallel file indexer](12-capstone-parallel-indexer.md) | a concurrent, tested, race-free application, end to end |

## How to work through it

- Treat lessons as reference material you revisit. Several (macros, unsafe, FFI, async internals) reward a second pass after you've used the feature for real.
- Lessons 5–7 (threads, async) build on each other and on Intermediate lesson 7 (`Arc`, `Rc`, `RefCell`).
- Run Miri (`cargo +nightly miri test`) on anything involving `unsafe`.
- Measure before you optimize; the performance lesson is a method, not a checklist.

## When you're done

Build the indexer capstone to the self-check bar: warning-free and clippy-clean, no `unsafe` and no `Mutex`, byte-identical output at any thread count, and a measured speedup. Then stop studying and go build something real.
