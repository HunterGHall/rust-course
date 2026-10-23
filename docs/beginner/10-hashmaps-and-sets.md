# 10. HashMaps & sets

## What & why

A **`HashMap<K, V>`** stores key–value pairs and finds a value by its key in (amortized) constant time — the right tool for lookups, counting, caching and grouping. A **`HashSet<T>`** stores unique items and answers "have I seen this?" instantly. Rust also has ordered versions, **`BTreeMap`** and **`BTreeSet`**, which keep keys sorted.

None of these are in the prelude, so you import them:

```rust
use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};
```

## Creating and reading a map

```rust
use std::collections::HashMap;

fn main() {
    let mut scores: HashMap<String, u32> = HashMap::new();
    scores.insert(String::from("Ada"), 95);
    scores.insert(String::from("Grace"), 88);

    // From an array of pairs:
    let capitals = HashMap::from([("France", "Paris"), ("Japan", "Tokyo")]);

    println!("{}", scores.len());                    // 2
    println!("{:?}", scores.get("Ada"));             // Some(95)
    println!("{:?}", scores.get("Linus"));           // None
    println!("{}", capitals["Japan"]);               // Tokyo — panics if the key is missing
    println!("{}", scores.contains_key("Grace"));    // true
    println!("{}", scores.get("Linus").copied().unwrap_or(0));   // 0 — default when missing
}
```

`get` returns `Option<&V>`. Keys can be looked up with a borrowed form (`&str` for `String` keys), so you don't allocate to search.

## Updating

```rust
use std::collections::HashMap;

fn main() {
    let mut stock: HashMap<&str, i32> = HashMap::new();

    stock.insert("apples", 3);
    stock.insert("apples", 5);                      // replaces: now 5
    println!("{:?}", stock);

    stock.entry("pears").or_insert(10);             // insert only if absent
    stock.entry("pears").or_insert(99);             // ignored — already there
    println!("{}", stock["pears"]);                 // 10

    *stock.entry("apples").or_insert(0) += 1;       // update in place
    println!("{}", stock["apples"]);                // 6

    if let Some(n) = stock.get_mut("pears") {
        *n -= 4;
    }
    println!("{:?}", stock.remove("pears"));        // Some(6)
    println!("{:?}", stock.remove("kiwis"));        // None
}
```

The **`entry` API** is the idiomatic way to say "find this key; if it's missing, create it". It does one lookup instead of two.

## Iterating

```rust
use std::collections::HashMap;

fn main() {
    let ages = HashMap::from([("Ada", 36), ("Grace", 85), ("Linus", 55)]);

    for (name, age) in &ages {
        println!("{name} is {age}");     // order is NOT guaranteed
    }

    // For stable output, sort first:
    let mut entries: Vec<_> = ages.iter().collect();
    entries.sort();
    println!("{entries:?}");             // [("Ada", 36), ("Grace", 85), ("Linus", 55)]

    let mut names: Vec<&&str> = ages.keys().collect();
    names.sort();
    println!("{names:?}");
    let total: i32 = ages.values().sum();
    println!("{total}");                 // 176
}
```

A `HashMap` deliberately randomizes its iteration order between runs (as protection against certain attacks). Never rely on it. If you need sorted keys, use a `BTreeMap`.

## The counting idiom

```rust
use std::collections::HashMap;

fn main() {
    let text = "the quick brown fox jumps over the lazy dog the end";
    let mut counts: HashMap<&str, usize> = HashMap::new();

    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }

    let mut ranked: Vec<(&str, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));   // by count desc, then word
    for (word, n) in ranked.iter().take(3) {
        println!("{word}: {n}");        // the: 3, then brown: 1, dog: 1
    }
}
```

## Grouping

```rust
use std::collections::HashMap;

fn main() {
    let words = ["apple", "avocado", "banana", "blueberry", "cherry"];
    let mut by_letter: HashMap<char, Vec<&str>> = HashMap::new();

    for word in words {
        let first = word.chars().next().unwrap();
        by_letter.entry(first).or_default().push(word);   // or_default creates an empty Vec
    }

    let mut keys: Vec<_> = by_letter.keys().copied().collect();
    keys.sort();
    for k in keys {
        println!("{k}: {:?}", by_letter[&k]);
    }
}
```

## Ownership and maps

Inserting moves owned keys and values *into* the map:

<!-- check: fail -->
```rust
use std::collections::HashMap;

fn main() {
    let name = String::from("Ada");
    let mut map = HashMap::new();
    map.insert(name, 1);
    println!("{name}");        // error[E0382]: borrow of moved value: `name`
}
```

Insert a clone (`name.clone()`) if you still need the original, or use `&str` keys that borrow from something that outlives the map.

## What can be a key?

A key type must implement `Eq` and `Hash`. Integers, `bool`, `char`, `String`, `&str`, tuples and `Vec`s of those all qualify. **`f32`/`f64` do not** (NaN breaks equality). For your own types:

```rust
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let mut visited: HashMap<Point, &str> = HashMap::new();
    visited.insert(Point { x: 0, y: 0 }, "start");
    println!("{:?}", visited.get(&Point { x: 0, y: 0 }));   // Some("start")

    let mut grid: HashMap<(i32, i32), char> = HashMap::new();   // tuples work too
    grid.insert((1, 2), '#');
    println!("{:?}", grid.get(&(1, 2)));
}
```

(`#[derive(...)]` asks the compiler to generate the trait implementations — Intermediate lesson 3.)

## Sets

```rust
use std::collections::HashSet;

fn main() {
    let mut seen = HashSet::new();
    println!("{}", seen.insert(3));      // true — newly added
    println!("{}", seen.insert(3));      // false — already present
    seen.insert(4);
    println!("{} {}", seen.contains(&4), seen.len());   // true 2

    let a: HashSet<i32> = [1, 2, 3, 4].into_iter().collect();
    let b: HashSet<i32> = HashSet::from([3, 4, 5]);

    let mut both: Vec<_> = a.intersection(&b).copied().collect();
    let mut either: Vec<_> = a.union(&b).copied().collect();
    let mut only_a: Vec<_> = a.difference(&b).copied().collect();
    both.sort();
    either.sort();
    only_a.sort();
    println!("{both:?} {either:?} {only_a:?}");   // [3, 4] [1, 2, 3, 4, 5] [1, 2]
    println!("{}", a.is_subset(&b));              // false
}
```

A set is the fastest way to deduplicate or to test membership:

```rust
use std::collections::HashSet;

fn main() {
    let items = [3, 1, 3, 2, 1];
    let unique: HashSet<_> = items.iter().collect();
    println!("{}", unique.len());        // 3

    let mut seen = HashSet::new();
    let first_dup = items.iter().find(|&&x| !seen.insert(x));
    println!("{first_dup:?}");           // Some(3)
}
```

## Ordered versions

```rust
use std::collections::{BTreeMap, BTreeSet};

fn main() {
    let mut tree = BTreeMap::new();
    tree.insert(30, "thirty");
    tree.insert(10, "ten");
    tree.insert(20, "twenty");

    println!("{tree:?}");                                    // {10: "ten", 20: "twenty", 30: "thirty"}
    println!("{:?}", tree.first_key_value());                // Some((10, "ten"))
    for (k, v) in tree.range(15..=30) {                      // iterate a key range
        print!("{k}={v} ");
    }
    println!();

    let set: BTreeSet<char> = "hello world".chars().filter(|c| c.is_alphabetic()).collect();
    println!("{set:?}");                                     // {'d', 'e', 'h', 'l', 'o', 'r', 'w'}
}
```

`HashMap` is faster for plain lookups; `BTreeMap` gives sorted iteration, range queries and deterministic output. Choose by need.

## Common mistakes

- **Relying on `HashMap` iteration order.** It's random; sort or use `BTreeMap`.
- **Using `map[key]` for keys that might be absent** — it panics. Use `get`.
- **Doing `get` then `insert`** instead of the `entry` API.
- **Using `f64` as a key.** It doesn't implement `Hash`/`Eq`; use an integer representation or a wrapper.
- **Forgetting that inserting moves the key and value.**
- **Using a map when a sorted `Vec` or a small array would do.** For a handful of items, a `Vec<(K, V)>` is often faster.

## Exercises

1. Count how many times each letter appears in a sentence and print them alphabetically.
2. Given a list of words, group them by length into a `HashMap<usize, Vec<&str>>`.
3. Write `fn has_duplicates(values: &[i32]) -> bool` using a `HashSet`.
4. Given two lists of numbers, print the numbers in both, and the numbers in only the first.
5. Build a phone book `HashMap<String, String>`; write `lookup(&book, name)` that returns the number or `"unknown"`.
6. Find the most frequent word in a text (break ties alphabetically).
7. Do exercise 1 again with a `BTreeMap` and notice you no longer need to sort.
8. Use a `HashSet<(i32, i32)>` to track the squares a robot visits as it follows the moves `"NNESWW"`.

## Recap

- `HashMap<K, V>` for lookups; `get` returns `Option`, `entry().or_insert()` updates or creates.
- Iteration order is unspecified — sort, or use `BTreeMap`/`BTreeSet`.
- Inserting moves owned values into the collection.
- Keys must be `Eq + Hash`; floats are not.
- `HashSet` deduplicates and offers `union`, `intersection`, `difference`.

Next: [Option, Result & files →](11-option-result-and-files.md)
