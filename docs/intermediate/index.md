# Intermediate

For you if you know the syntax but structs vs enums, traits, generics and lifetimes still feel shaky — or if you keep fighting the borrow checker and want to understand *why*. By the end you'll model data with types, write generic and trait-based code, handle errors properly, split programs into modules, depend on crates, and test it all.

## Lessons

| # | Lesson | You'll learn |
| --- | --- | --- |
| 1 | [Structs & methods](01-structs-and-methods.md) | `struct`, `impl`, `&self`/`&mut self`/`self`, `derive`, newtypes, private fields |
| 2 | [Enums & pattern matching](02-enums-and-pattern-matching.md) | variants with data, exhaustive `match`, guards, slice patterns, state machines |
| 3 | [Traits](03-traits.md) | defining and implementing traits, `impl Trait`, `dyn Trait`, `Display`, `From`, operators, `Iterator` |
| 4 | [Generics](04-generics.md) | type parameters, bounds, `where`, associated types, const generics, `PhantomData` |
| 5 | [Lifetimes](05-lifetimes.md) | why references need lifetimes, annotations, elision, structs with references, `'static` |
| 6 | [Closures & iterators](06-closures-and-iterators.md) | `Fn`/`FnMut`/`FnOnce`, `move`, adapters, consumers, `collect`, laziness |
| 7 | [Smart pointers](07-smart-pointers.md) | `Box`, `Rc`, `RefCell`, `Weak`, `Cow`, `Deref`, `Drop` |
| 8 | [Error handling in depth](08-error-handling.md) | custom errors, `From` + `?`, `Box<dyn Error>`, `thiserror`, `anyhow` |
| 9 | [Modules & packages](09-modules-and-packages.md) | modules, visibility, `use`, library + binary, workspaces, `cfg` |
| 10 | [Dependencies & serde](10-dependencies-and-serde.md) | `cargo add`, semver, features, `rand`, JSON with `serde` |
| 11 | [Testing](11-testing.md) | unit, integration and doc tests, assertions, fixtures, designing for testability |
| 12 | [Capstone: link checker](12-capstone-link-checker.md) | a tested library + CLI that checks Markdown links |

## How to work through it

- Type the examples; then change them to break things and read the compiler's complaint.
- Lessons 3–5 (traits, generics, lifetimes) build on each other. If lifetimes feel hard, re-read Beginner lesson 7 on borrowing first.
- Before moving on, you should be able to explain *why* a design uses an enum rather than a trait object (or vice versa).
- Build the capstone yourself first, then compare with the reference code.

## When you're done

You should be able to build and test a multi-module library with a CLI on top, define your own error and data types, and read most of the standard library's documentation. Then go to [Advanced](../advanced/index.md).
