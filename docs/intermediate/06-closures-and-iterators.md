# 6. Closures & iterators

## What & why

Two features that together make Rust code concise *and* fast:

- A **closure** is an anonymous function that can capture variables from the place where it's written.
- An **iterator** produces a sequence of values one at a time. Iterator *adapters* (`map`, `filter`, `take`...) chain together into a pipeline, and the compiler optimizes the whole chain into the same machine code as a hand-written loop — Rust calls this a **zero-cost abstraction**.

## Closures

```rust
fn main() {
    let add_one = |x: i32| x + 1;                 // types can usually be inferred
    let add = |a, b| a + b;
    let greet = || println!("hi");
    let block = |x: i32| {
        let y = x * 2;
        y + 1
    };

    println!("{} {} {}", add_one(1), add(2, 3), block(4));
    greet();
}
```

Syntax: `|parameters| expression`. Types are inferred from the first use and then fixed — a closure used with `i32` once can't later be used with `String`.

### Capturing the environment

Unlike plain functions, closures can use variables from their surroundings. Rust captures in the least demanding way the body allows:

```rust
fn main() {
    let name = String::from("Ada");
    let print_name = || println!("{name}");        // borrows `name` immutably
    print_name();
    print_name();
    println!("{name}");                            // still usable

    let mut count = 0;
    let mut increment = || count += 1;             // borrows `count` mutably
    increment();
    increment();
    println!("{count}");                           // 2 — fine once `increment` is no longer used

    let owned = String::from("moved in");
    let consume = move || println!("{owned}");     // `move` forces the closure to take ownership
    consume();
    // println!("{owned}");                        // error: value moved into the closure
}
```

`move` is essential when a closure outlives its creator, e.g. when sending it to another thread or returning it from a function.

### The three closure traits

The compiler gives each closure one or more of these, based on what its body does with captured values:

| Trait | The closure... | Can be called |
| --- | --- | --- |
| `Fn` | only reads captures | many times, even concurrently |
| `FnMut` | mutates captures | many times, one at a time |
| `FnOnce` | consumes (moves out) a capture | once |

Every closure is `FnOnce`; those that don't move out are `FnMut`; those that also don't mutate are `Fn`. You'll mostly see them as bounds on function parameters:

```rust
fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 {
    f(x)
}

fn apply_n_times(mut f: impl FnMut(), n: usize) {
    for _ in 0..n {
        f();
    }
}

fn run_once<F: FnOnce() -> String>(f: F) -> String {
    f()
}

fn main() {
    let factor = 3;
    println!("{}", apply(|x| x * factor, 5));     // 15

    let mut total = 0;
    apply_n_times(|| total += 10, 4);
    println!("{total}");                          // 40

    let s = String::from("consumed");
    println!("{}", run_once(move || s));          // returns the captured String
}
```

Rule of thumb: **accept the loosest trait you can** (`FnOnce` if you call it once, else `FnMut`, else `Fn`).

### Returning closures

```rust
fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

fn make_op(kind: &str) -> Box<dyn Fn(i32, i32) -> i32> {
    match kind {
        "add" => Box::new(|a, b| a + b),
        "mul" => Box::new(|a, b| a * b),
        _ => Box::new(|_, _| 0),
    }
}

fn main() {
    let add5 = make_adder(5);
    println!("{}", add5(10));              // 15
    println!("{}", make_op("mul")(6, 7));  // 42
}
```

`impl Fn` for one concrete closure; `Box<dyn Fn>` when different branches return different closures.

## Iterators

Anything implementing `Iterator` has a `next()` method returning `Some(item)` until it finishes with `None`:

```rust
fn main() {
    let v = vec![10, 20, 30];
    let mut it = v.iter();
    println!("{:?}", it.next());   // Some(10)
    println!("{:?}", it.next());   // Some(20)
    println!("{:?}", it.next());   // Some(30)
    println!("{:?}", it.next());   // None
}
```

A `for` loop is just repeated `next()` calls. Three ways to get an iterator from a collection:

| Method | Yields | Collection afterwards |
| --- | --- | --- |
| `.iter()` | `&T` | untouched |
| `.iter_mut()` | `&mut T` | modified in place |
| `.into_iter()` | `T` (owned) | consumed |

### Adapters: lazy transformations

Adapters build a new iterator and do **nothing until something pulls values**:

```rust
fn main() {
    let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let result: Vec<i32> = numbers
        .iter()
        .filter(|&&n| n % 2 == 0)       // keep evens
        .map(|n| n * n)                 // square them
        .take(3)                        // only the first three
        .collect();                     // run the pipeline, gather into a Vec
    println!("{result:?}");             // [4, 16, 36]
}
```

The commonly used adapters:

```rust
fn main() {
    let words = ["apple", "bob", "cat", "doge"];

    println!("{:?}", words.iter().map(|w| w.len()).collect::<Vec<_>>());          // [5, 3, 3, 4]
    println!("{:?}", words.iter().enumerate().map(|(i, w)| format!("{i}{w}")).collect::<Vec<_>>());
    println!("{:?}", words.iter().zip(1..).collect::<Vec<_>>());                  // pairs with 1, 2, 3...
    println!("{:?}", words.iter().skip(1).step_by(2).collect::<Vec<_>>());        // ["bob", "doge"]
    println!("{:?}", words.iter().rev().take_while(|w| w.len() <= 4).collect::<Vec<_>>());
    println!("{:?}", words.iter().flat_map(|w| w.chars().take(1)).collect::<String>());   // "abcd"
    println!("{:?}", [[1, 2], [3, 4]].iter().flatten().collect::<Vec<_>>());       // [1, 2, 3, 4]
    println!("{:?}", (1..4).chain(7..9).collect::<Vec<_>>());                      // [1, 2, 3, 7, 8]
    println!("{:?}", [1, 2, 3, 4].windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>());   // [1, 1, 1]
    println!("{:?}", [1, 5, 2].iter().scan(0, |acc, &x| { *acc += x; Some(*acc) }).collect::<Vec<_>>());  // [1, 6, 8]
}
```

### Consumers: producing a result

```rust
fn main() {
    let nums = [3, 1, 4, 1, 5, 9, 2, 6];

    println!("{}", nums.iter().sum::<i32>());                       // 31
    println!("{}", nums.iter().product::<i32>());                   // 6480
    println!("{}", nums.iter().count());                            // 8
    println!("{:?}", nums.iter().max());                            // Some(9)
    println!("{:?}", nums.iter().min_by_key(|&&n| (n as i32 - 5).abs()));   // Some(4) — first closest to 5
    println!("{}", nums.iter().any(|&n| n > 8));                    // true
    println!("{}", nums.iter().all(|&n| n > 0));                    // true
    println!("{:?}", nums.iter().find(|&&n| n > 4));                // Some(5)
    println!("{:?}", nums.iter().position(|&n| n == 4));            // Some(2)
    println!("{}", nums.iter().fold(0, |acc, n| acc * 2 + n));      // fold: general accumulation
    println!("{:?}", nums.iter().copied().reduce(|a, b| a.max(b))); // Some(9)

    let (small, big): (Vec<i32>, Vec<i32>) = nums.iter().partition(|&&n| n < 4);
    println!("{small:?} {big:?}");
    let (a, b): (Vec<usize>, Vec<i32>) = nums.iter().enumerate().map(|(i, &n)| (i, n)).unzip();
    println!("{} {}", a.len(), b.len());
}
```

### `collect` into many things

`collect` is generic over the output; the target type tells it what to build:

```rust
use std::collections::{HashMap, HashSet};

fn main() {
    let words = ["a", "bb", "a", "ccc"];

    let v: Vec<&str> = words.iter().copied().collect();
    let set: HashSet<&str> = words.iter().copied().collect();
    let lengths: HashMap<&str, usize> = words.iter().map(|w| (*w, w.len())).collect();
    let joined: String = words.iter().copied().collect();                  // concatenates
    println!("{} {} {} {joined}", v.len(), set.len(), lengths.len());

    // Collecting Results: stop at the first error.
    let parsed: Result<Vec<i32>, _> = "1,2,3".split(',').map(|s| s.parse::<i32>()).collect();
    println!("{parsed:?}");                                                // Ok([1, 2, 3])
    let bad: Result<Vec<i32>, _> = "1,x,3".split(',').map(|s| s.parse::<i32>()).collect();
    println!("{}", bad.is_err());                                          // true

    // Collecting Options: None if any element is None.
    let all: Option<Vec<u32>> = ['1', '2', '3'].iter().map(|c| c.to_digit(10)).collect();
    println!("{all:?}");                                                   // Some([1, 2, 3])
}
```

### Laziness

```rust
fn main() {
    let it = (1..).map(|n| {
        println!("computing {n}");
        n * 2
    });
    // Nothing printed yet: the map hasn't been pulled.
    let first_two: Vec<i32> = it.take(2).collect();   // prints "computing 1", "computing 2"
    println!("{first_two:?}");
}
```

That's why infinite ranges like `1..` are fine: `take`, `find` and `take_while` stop pulling.

## Iterators vs loops

```rust
fn main() {
    let data = vec![4, 8, 15, 16, 23, 42];

    // Loop version
    let mut total = 0;
    for &n in &data {
        if n % 2 == 0 {
            total += n * 10;
        }
    }

    // Iterator version
    let total2: i32 = data.iter().filter(|&&n| n % 2 == 0).map(|&n| n * 10).sum();

    println!("{total} {total2}");
}
```

Both compile to equivalent machine code. Choose by readability: iterators when you're transforming a sequence; loops when there's complicated control flow (early returns, several mutable variables).

## Writing your own iterator

Implement one method, `next`, and every adapter is yours (see lesson 3). Return an iterator from a function with `impl Iterator`:

```rust
fn primes() -> impl Iterator<Item = u32> {
    (2..).filter(|&n| (2..n).take_while(|d| d * d <= n).all(|d| n % d != 0))
}

fn main() {
    println!("{:?}", primes().take(8).collect::<Vec<_>>());   // [2, 3, 5, 7, 11, 13, 17, 19]
}
```

## Sorting with closures

```rust
#[derive(Debug)]
struct Player {
    name: &'static str,
    score: u32,
}

fn main() {
    let mut players = vec![
        Player { name: "Ada", score: 90 },
        Player { name: "Bob", score: 95 },
        Player { name: "Cy", score: 90 },
    ];
    players.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(b.name)));
    println!("{players:?}");
    players.sort_by_key(|p| std::cmp::Reverse(p.name.len()));
    let best = players.iter().max_by_key(|p| p.score).unwrap();
    println!("{}", best.name);
}
```

## Common mistakes

- **Forgetting to consume.** `v.iter().map(...)` alone does nothing (the compiler warns "iterators are lazy"). End with `collect`, `sum`, `for_each`, etc.
- **Annotating `collect`'s type** — it needs to know the target: `collect::<Vec<_>>()` or `let x: Vec<_> = ...`.
- **`&&n` patterns confusing you.** `.iter()` yields `&T`, and `filter` passes `&&T`; use `.copied()` / `.cloned()` to simplify.
- **A `move` closure used after its captures moved** (the original variable is gone).
- **Borrow conflicts in `FnMut` closures** — you can't use the captured variable elsewhere while the closure is alive.
- **Writing long iterator chains nobody can read.** Name intermediate steps.

## Exercises

1. Square the odd numbers of `1..=20` and sum them.
2. From a list of words, build a `Vec` of the lengths of those starting with an uppercase letter.
3. Write `fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C`.
4. Parse a comma-separated line of numbers into `Result<Vec<f64>, _>` and compute the mean.
5. Use `fold` to find the longest string in a list.
6. Write a `make_counter()` that returns a closure which gives 1, 2, 3, ... each time it is called.
7. Use `windows(2)` and `all` to test whether a slice is strictly increasing.
8. Group words by first letter into a `HashMap<char, Vec<&str>>` using `fold` instead of a `for` loop.

## Recap

- Closures capture by borrow, mutable borrow, or move; `Fn`, `FnMut`, `FnOnce` describe how.
- Iterators are lazy: adapters (`map`, `filter`, `zip`, `take`...) build a pipeline, consumers (`collect`, `sum`, `fold`...) run it.
- `.iter()` borrows, `.iter_mut()` mutates, `.into_iter()` consumes.
- `collect` can build `Vec`, `HashMap`, `String`, and even `Result<Vec<_>, _>`.
- Iterator pipelines compile to loop-quality code: zero-cost abstractions.

Next: [Smart pointers →](07-smart-pointers.md)
