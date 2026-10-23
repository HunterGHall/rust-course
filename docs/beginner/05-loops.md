# 5. Loops

## What & why

Rust has three loops: `loop` (forever, until you `break`), `while` (while a condition holds), and `for` (over anything iterable). In practice `for` is by far the most common — and it's the safest, because it can't run off the end of a collection.

## `for` over a range

```rust
fn main() {
    for i in 0..5 {          // 0, 1, 2, 3, 4  (end is exclusive)
        print!("{i} ");
    }
    println!();

    for i in 1..=5 {         // 1, 2, 3, 4, 5  (inclusive end)
        print!("{i} ");
    }
    println!();

    for i in (1..=5).rev() { // 5, 4, 3, 2, 1
        print!("{i} ");
    }
    println!();

    for i in (0..20).step_by(5) {   // 0, 5, 10, 15
        print!("{i} ");
    }
    println!();
}
```

`a..b` is a range value like any other. If you don't need the counter, name it `_`:

```rust
fn main() {
    for _ in 0..3 {
        println!("hello");
    }
}
```

## `for` over collections

```rust
fn main() {
    let fruits = ["apple", "banana", "cherry"];

    for fruit in fruits {
        println!("{fruit}");
    }

    // With the index:
    for (i, fruit) in fruits.iter().enumerate() {
        println!("{i}: {fruit}");
    }

    // Two sequences together:
    let prices = [1.5, 0.25, 4.0];
    for (fruit, price) in fruits.iter().zip(prices.iter()) {
        println!("{fruit} costs {price}");
    }
}
```

`.iter()`, `.enumerate()`, `.zip()` and `.rev()` are **iterator adapters**; you'll go deep on iterators in Intermediate. For now, `for x in &collection` borrows each item, `for x in collection` consumes the collection (lesson 7 explains why that matters).

## `while`

```rust
fn main() {
    let mut n = 27;
    let mut steps = 0;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        steps += 1;
    }
    println!("reached 1 in {steps} steps");   // 111
}
```

Use `while` when the number of iterations isn't known in advance.

## `loop`, `break` and `continue`

`loop` runs forever; `break` leaves it and can even **return a value**:

```rust
fn main() {
    let mut attempts = 0;
    let result = loop {
        attempts += 1;
        if attempts * attempts > 200 {
            break attempts;       // the value of the whole loop expression
        }
    };
    println!("{result}");         // 15

    for i in 1..=10 {
        if i % 2 == 0 {
            continue;             // skip to the next iteration
        }
        if i > 7 {
            break;
        }
        print!("{i} ");           // 1 3 5 7
    }
    println!();
}
```

Only `loop` can break with a value, because it's the only one guaranteed to run until a `break`.

## Labels

When loops are nested, a label lets `break` and `continue` target an outer loop:

```rust
fn main() {
    let mut found = None;
    'outer: for a in 1..50 {
        for b in a..50 {
            let c_squared = a * a + b * b;
            for c in b..100 {
                if c * c == c_squared {
                    found = Some((a, b, c));
                    break 'outer;           // leave all three loops
                }
            }
        }
    }
    println!("{found:?}");                   // Some((3, 4, 5))
}
```

## Building up a result

The classic accumulator pattern:

```rust
fn main() {
    let mut total = 0;
    let mut evens = Vec::new();
    for n in 1..=10 {
        total += n;
        if n % 2 == 0 {
            evens.push(n);
        }
    }
    println!("{total} {evens:?}");   // 55 [2, 4, 6, 8, 10]
}
```

Rust programmers often write the same thing with iterators — `(1..=10).sum::<i32>()` or `.filter(...).collect()` — which you'll learn soon. Loops are never wrong, though; start here.

## Reading until the user stops

```rust
use std::io;

fn main() {
    let mut sum = 0;
    loop {
        let mut line = String::new();
        io::stdin().read_line(&mut line).expect("read failed");
        let line = line.trim();
        if line.is_empty() {
            break;                       // blank line ends the input
        }
        match line.parse::<i32>() {
            Ok(n) => sum += n,
            Err(_) => println!("'{line}' is not a number, skipping"),
        }
    }
    println!("sum = {sum}");
}
```

(`Ok`/`Err` are explained in lesson 11. Running this at the end-of-input also stops, because `read_line` then returns an empty line.)

## Common mistakes

- **Off-by-one with ranges.** `0..5` stops at 4; `0..=5` includes 5.
- **Indexing with a counter** (`while i < v.len() { v[i] ... }`) when `for x in &v` would do. The `for` form can't go out of bounds.
- **Looping on `a..b` with `a > b`.** It's simply empty — no error, no iterations. Use `.rev()` for counting down.
- **Infinite `while`** because the variable never changes. Check that your loop body updates the condition.
- **Modifying a collection while iterating over it.** The borrow checker refuses; collect changes and apply them afterwards.
- **Expecting `break value` in `while` or `for`.** Only `loop` supports it.

## Exercises

1. Print the multiplication table for 7 (`7 x 1 = 7` ... `7 x 10 = 70`).
2. Sum the numbers 1 to 100 with a `for` loop, then with `(1..=100).sum::<i32>()`.
3. Print a right triangle of `*` characters with 5 rows using nested loops.
4. Write a countdown from 10 to 1 using `.rev()`, then print "liftoff!".
5. Find the first number divisible by both 7 and 13 that is greater than 1000, using `loop` and `break value`.
6. Use a labelled `break` to find the first pair `(a, b)` with `a * b == 391` and `1 < a < b < 100`.
7. Print each character of a string with its index using `.chars().enumerate()`.
8. Write a program that reads numbers until a blank line, then prints their average.

## Recap

- `for x in range_or_collection` is the default loop; `0..n` excludes `n`, `0..=n` includes it.
- `while` for unknown iteration counts; `loop` for "until I break", and `break value` returns a value.
- `continue` skips ahead; labels (`'outer:`) target nested loops.
- `.iter()`, `.enumerate()`, `.zip()`, `.rev()`, `.step_by()` shape what you loop over.

Next: [Functions →](06-functions.md)
