# Beginner

Start here if you've never written Rust, or never finished a course. By the end you'll be running real programs from Cargo projects, you'll understand ownership and borrowing — the idea that makes Rust different — and you'll have built a command-line app that persists data to disk.

## Lessons

| # | Lesson | You'll learn |
| --- | --- | --- |
| 1 | [Setup & your first program](01-setup-and-first-program.md) | installing with `rustup`, `cargo run`, `println!`, reading input, compiler errors |
| 2 | [Variables, mutability & types](02-variables-and-types.md) | `let` vs `let mut`, shadowing, `const`, scalar types, tuples, arrays |
| 3 | [Numbers & math](03-numbers-and-math.md) | integer division, no implicit conversion, `as` vs `TryFrom`, overflow, floats |
| 4 | [Booleans & conditionals](04-booleans-and-conditionals.md) | comparisons, `if` as an expression, `match`, guards, `if let`, `let else` |
| 5 | [Loops](05-loops.md) | `for`, ranges, `while`, `loop` with `break value`, labels |
| 6 | [Functions](06-functions.md) | parameters, return values, statements vs expressions, `fn` pointers |
| 7 | [Ownership, borrowing & slices](07-ownership-and-borrowing.md) | move, `Clone`, `Copy`, `&` and `&mut`, the borrowing rules, slices |
| 8 | [Strings](08-strings.md) | `String` vs `&str`, UTF-8, `chars`, slicing, `split`, `parse` |
| 9 | [Arrays, vectors & slices](09-vectors-and-slices.md) | `[T; N]`, `Vec<T>`, `&[T]`, safe indexing, sorting, iterator taste |
| 10 | [HashMaps & sets](10-hashmaps-and-sets.md) | `HashMap`, the `entry` API, counting, grouping, `HashSet`, `BTreeMap` |
| 11 | [Option, Result & files](11-option-result-and-files.md) | `Option`, `Result`, `?`, `std::fs`, `Path`, arguments, exit codes |
| 12 | [Capstone: task tracker](12-capstone-task-tracker.md) | put it all together in a real CLI app |

## How to work through it

- One lesson per sitting. Type every example. Do the exercises.
- Don't move on until the "Recap" all makes sense — especially lesson 7.
- When the compiler rejects your code, read the *first* error slowly. It is usually right and often shows the fix.
- Stuck? Re-read the "Common mistakes" section — your bug is probably there.

## When you're done

You should be able to explain every line of the capstone and add a new feature to it unaided. Then go to [Intermediate](../intermediate/index.md).
