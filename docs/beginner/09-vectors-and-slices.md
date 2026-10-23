# 9. Arrays, vectors & slices

## What & why

Three related types hold sequences:

| Type | Size | Where | Use it for |
| --- | --- | --- | --- |
| `[T; N]` array | fixed at compile time | stack | small, known-size data |
| `Vec<T>` vector | grows and shrinks | heap | the default list |
| `&[T]` slice | a view of either | borrowed | function parameters |

`Vec<T>` is the one you'll use most. It is the Rust equivalent of Python's `list` or Swift's `Array`, with ownership rules you now understand.

## Arrays

```rust
fn main() {
    let primes = [2, 3, 5, 7, 11];
    let zeros = [0u8; 4];                 // [0, 0, 0, 0]
    let names: [&str; 3] = ["a", "b", "c"];

    println!("{} {} {}", primes[0], primes.len(), zeros.len());
    println!("{:?} {:?}", primes, names);
    println!("{}", primes.contains(&7));  // true
}
```

The length is part of the type: `[i32; 3]` and `[i32; 4]` are different types. Out-of-range indexing **panics** (it never reads garbage like C):

<!-- check: wrap panics -->
```rust
let a = [1, 2, 3];
let i = std::hint::black_box(7);
println!("{}", a[i]);     // panics: index out of bounds: the len is 3 but the index is 7
```

## Vectors

```rust
fn main() {
    let mut v: Vec<i32> = Vec::new();
    v.push(10);
    v.push(20);
    v.push(30);

    let w = vec![1, 2, 3];              // the vec! macro
    let zeros = vec![0; 5];             // [0, 0, 0, 0, 0]
    let from_range: Vec<u32> = (1..=5).collect();

    println!("{v:?} {w:?} {zeros:?} {from_range:?}");
    println!("{} {}", v.len(), v.is_empty());
}
```

### Reading elements

```rust
fn main() {
    let v = vec![10, 20, 30];

    println!("{}", v[1]);              // 20 — panics if out of range
    println!("{:?}", v.get(1));        // Some(20)
    println!("{:?}", v.get(99));       // None — safe
    println!("{:?} {:?}", v.first(), v.last());   // Some(10) Some(30)

    if let Some(x) = v.get(5) {
        println!("{x}");
    } else {
        println!("no element 5");
    }
}
```

Use `v[i]` when an out-of-range index would be a bug in your program; use `get(i)` when the index comes from outside.

### Changing elements

```rust
fn main() {
    let mut v = vec![1, 2, 3, 4, 5];

    v[0] = 100;
    v.push(6);
    println!("{:?}", v.pop());         // Some(6)
    v.insert(1, 50);                   // [100, 50, 2, 3, 4, 5]
    let removed = v.remove(0);         // 100 — shifts everything left
    v.swap(0, 2);
    v.extend([7, 8]);
    v.retain(|&x| x != 3);             // keep only items matching the closure
    println!("{removed} {v:?}");

    v.truncate(3);
    v.reverse();
    println!("{v:?}");
    v.clear();
    println!("{}", v.is_empty());
}
```

## Iterating

```rust
fn main() {
    let mut v = vec![1, 2, 3];

    for x in &v {                      // borrow each element
        print!("{x} ");
    }
    println!();

    for x in &mut v {                  // borrow mutably, edit in place
        *x *= 10;                      // * dereferences the reference
    }
    println!("{v:?}");                 // [10, 20, 30]

    for (i, x) in v.iter().enumerate() {
        print!("{i}:{x} ");
    }
    println!();

    for x in v {                       // consume: v can't be used afterwards
        print!("{x} ");
    }
    println!();
}
```

Which form? `&v` to read, `&mut v` to modify, plain `v` to consume. This is ownership showing up in everyday code.

## Ownership meets vectors

<!-- check: fail -->
```rust
fn main() {
    let mut v = vec![1, 2, 3];
    let first = &v[0];         // borrow
    v.push(4);                 // error[E0502]: might reallocate and invalidate `first`
    println!("{first}");
}
```

`push` may need to allocate a bigger buffer and move all the elements; `first` would then point at freed memory. The borrow checker catches the exact bug that causes crashes in C++. Copy the value instead (`let first = v[0];`) or finish using the reference before pushing.

Elements that aren't `Copy` can't be moved out by indexing:

<!-- check: fail -->
```rust
fn main() {
    let names = vec![String::from("a"), String::from("b")];
    let first = names[0];      // error[E0507]: cannot move out of index
    println!("{first}");
}
```

Borrow (`&names[0]`) or clone (`names[0].clone()`), or take it out of the vector with `remove`/`swap_remove`/`pop`.

## Slices

A slice borrows a range of an array or vector. It is how functions should accept sequences:

```rust
fn sum(values: &[i32]) -> i32 {
    let mut total = 0;
    for v in values {
        total += v;
    }
    total
}

fn main() {
    let array = [1, 2, 3, 4];
    let vector = vec![10, 20, 30];

    println!("{}", sum(&array));          // &[i32; 4] coerces to &[i32]
    println!("{}", sum(&vector));         // &Vec<i32> coerces to &[i32]
    println!("{}", sum(&vector[1..]));    // a sub-slice
}
```

Useful slice methods:

```rust
fn main() {
    let v = [5, 3, 8, 1, 9, 2];

    println!("{:?}", &v[..2]);                 // [5, 3]
    println!("{:?}", v.iter().max());          // Some(9)
    println!("{:?}", v.iter().min());          // Some(1)
    println!("{}", v.iter().sum::<i32>());     // 28
    println!("{:?}", v.iter().position(|&x| x == 8));   // Some(2)
    println!("{}", v.contains(&9));            // true
    println!("{:?}", v.first_chunk::<2>());    // Some([5, 3])
    println!("{:?}", v.split_at(2));           // ([5, 3], [8, 1, 9, 2])

    for pair in v.chunks(2) {
        print!("{pair:?} ");                   // [5, 3] [8, 1] [9, 2]
    }
    println!();
    for window in v.windows(3).take(2) {
        print!("{window:?} ");                 // [5, 3, 8] [3, 8, 1]
    }
    println!();
}
```

## Sorting and searching

```rust
fn main() {
    let mut nums = vec![5, 3, 8, 1];
    nums.sort();
    println!("{nums:?}");                       // [1, 3, 5, 8]
    println!("{:?}", nums.binary_search(&5));   // Ok(2) — only valid on sorted data
    println!("{:?}", nums.binary_search(&4));   // Err(2) — where it would be inserted

    nums.sort_by(|a, b| b.cmp(a));              // descending
    println!("{nums:?}");

    let mut words = vec!["banana", "fig", "apple", "kiwi"];
    words.sort_by_key(|w| w.len());             // stable sort by length
    println!("{words:?}");                      // ["fig", "kiwi", "apple", "banana"]
    words.dedup();                              // remove consecutive duplicates

    let mut floats = vec![2.5, -1.0, 9.75];
    floats.sort_by(|a, b| a.partial_cmp(b).unwrap());   // f64 isn't Ord, so use partial_cmp
    println!("{floats:?}");
}
```

## A first taste of iterator chains

```rust
fn main() {
    let nums = vec![1, 2, 3, 4, 5, 6];

    let doubled: Vec<i32> = nums.iter().map(|x| x * 2).collect();
    let evens: Vec<i32> = nums.iter().filter(|&&x| x % 2 == 0).cloned().collect();
    let total: i32 = nums.iter().sum();

    println!("{doubled:?} {evens:?} {total}");
}
```

`map`, `filter` and `collect` turn sequences into other sequences without manual loops. They are compiled to the same machine code as the loop you'd write by hand. Intermediate lesson 6 covers them fully.

## 2-D data

```rust
fn main() {
    let mut grid = vec![vec![0; 3]; 2];    // 2 rows of 3 zeros
    grid[1][2] = 7;
    for row in &grid {
        println!("{row:?}");
    }

    let fixed = [[1, 2, 3], [4, 5, 6]];     // [[i32; 3]; 2]
    println!("{}", fixed[1][0]);
}
```

## Common mistakes

- **`v[i]` with an untrusted `i`.** It panics; use `get`.
- **Holding a reference into a vector while pushing** — the borrow checker error above.
- **Using `i32` as an index.** Indexes are `usize`; convert deliberately.
- **Moving out of a vector by indexing** non-`Copy` elements.
- **Taking `&Vec<T>` in function signatures.** Take `&[T]`.
- **`binary_search` on unsorted data** gives meaningless answers.
- **Calling `sort()` on floats** — `f64` has no total order; use `sort_by` with `partial_cmp` or `total_cmp`.
- **`remove(0)` in a hot loop** is O(n); use `VecDeque` for queues.

## Exercises

1. Make a vector of 1..=10, remove the even numbers with `retain`, and print the rest.
2. Write `fn largest(values: &[i32]) -> Option<i32>` that returns `None` for an empty slice.
3. Reverse a vector in place two ways: `reverse()`, and with a loop using `swap`.
4. Compute the mean and the median of a `Vec<f64>`.
5. Write `fn rotate_left(v: &mut [i32])` that moves the first element to the end.
6. Make a 3×3 grid, fill the diagonal with 1s, and print it.
7. Sort a vector of `(name, age)` tuples by age, then by name.
8. Use `windows(2)` to check whether a slice is sorted in ascending order.

## Recap

- Arrays are fixed-size; `Vec<T>` grows; `&[T]` borrows a view of either.
- `v[i]` panics when out of range, `v.get(i)` returns an `Option`.
- Iterate with `&v` (read), `&mut v` (edit), `v` (consume).
- Don't hold references into a `Vec` across a `push` — the borrow checker prevents it.
- Prefer `&[T]` in parameters; `sort`, `sort_by`, `sort_by_key`, `retain`, `dedup` are the daily tools.

Next: [HashMaps & sets →](10-hashmaps-and-sets.md)
