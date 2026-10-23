# 4. Generics

## What & why

Generics let you write code once and use it with many types — safely, and with **no runtime cost**. You've already used them: `Vec<T>`, `Option<T>`, `HashMap<K, V>`, `Result<T, E>`. This lesson shows how to write your own, and how traits constrain them.

## Generic functions

Without generics you'd write `largest_i32`, `largest_f64`, `largest_char`... With them:

```rust
fn largest<T: PartialOrd>(items: &[T]) -> &T {
    let mut best = &items[0];
    for item in items {
        if item > best {
            best = item;
        }
    }
    best
}

fn main() {
    println!("{}", largest(&[3, 9, 2]));
    println!("{}", largest(&[1.5, 0.2]));
    println!("{}", largest(&["pear", "apple", "zebra"]));
    println!("{}", largest(&['x', 'b']));
}
```

`<T: PartialOrd>` reads "for any type `T` that can be compared with `>`". Without that **bound**, the compiler refuses to let you use `>`:

<!-- check: fail -->
```rust
fn largest<T>(items: &[T]) -> &T {
    let mut best = &items[0];
    for item in items {
        if item > best {       // error[E0369]: binary operation `>` cannot be applied to type `&T`
            best = item;
        }
    }
    best
}

fn main() {
    println!("{}", largest(&[1, 2]));
}
```

The compiler checks the generic function *once*, against its bounds — not separately for each caller. That makes error messages local and trustworthy.

## Generic structs and enums

```rust
#[derive(Debug)]
struct Point<T> {
    x: T,
    y: T,
}

impl<T> Point<T> {
    fn new(x: T, y: T) -> Point<T> {
        Point { x, y }
    }
}

// Methods that only exist for particular types:
impl Point<f64> {
    fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

// Methods needing a bound:
impl<T: std::ops::Add<Output = T> + Copy> Point<T> {
    fn sum(&self) -> T {
        self.x + self.y
    }
}

fn main() {
    let a = Point::new(3, 4);
    let b = Point::new(3.0, 4.0);
    println!("{:?} {}", a, a.sum());
    println!("{}", b.distance_from_origin());      // 5
    // Point::new(1, 2.5) would not compile: x and y must be the same T.
}
```

Two type parameters when you need them:

```rust
#[derive(Debug)]
struct Pair<A, B> {
    first: A,
    second: B,
}

impl<A, B> Pair<A, B> {
    fn swap(self) -> Pair<B, A> {
        Pair { first: self.second, second: self.first }
    }
}

fn main() {
    let p = Pair { first: 1, second: "one" };
    println!("{:?}", p.swap());
}
```

## Trait bounds and `where` clauses

```rust
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;

fn count<T: Eq + Hash + Clone>(items: &[T]) -> HashMap<T, usize> {
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(item.clone()).or_insert(0) += 1;
    }
    counts
}

// A `where` clause keeps long signatures readable:
fn report<K, V>(map: &HashMap<K, V>) -> String
where
    K: Display + Ord,
    V: Debug,
{
    let mut entries: Vec<(&K, &V)> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries.iter().map(|(k, v)| format!("{k}={v:?}")).collect::<Vec<_>>().join(", ")
}

fn main() {
    let c = count(&["a", "b", "a", "c", "a"]);
    println!("{}", report(&c));        // a=3, b=1, c=1
}
```

## Monomorphization: zero-cost

When you call `largest::<i32>` and `largest::<f64>`, the compiler generates a separate, fully optimized copy of the function for each. The generic code runs exactly as fast as hand-written specific code. The trade-off: more generic instantiations means a bigger binary and longer compile times.

## Generic traits and associated types

```rust
// A trait with an associated type: each implementor picks ONE output type.
trait Container {
    type Item;
    fn items(&self) -> Vec<Self::Item>;
    fn first(&self) -> Option<Self::Item> {
        self.items().into_iter().next()
    }
}

struct Words(String);

impl Container for Words {
    type Item = String;
    fn items(&self) -> Vec<String> {
        self.0.split_whitespace().map(String::from).collect()
    }
}

fn main() {
    let w = Words("hello generic world".into());
    println!("{:?} {:?}", w.items(), w.first());
}
```

The difference from a generic trait (`trait Convert<T>`) is that a type may implement a generic trait many times (once per `T`), but an associated type is chosen once. `Iterator` uses an associated type (`Item`) for exactly this reason.

## `impl Trait` vs generics

```rust
use std::fmt::Display;

fn print_all(items: &[impl Display]) {            // sugar for <T: Display>(items: &[T])
    for i in items {
        print!("{i} ");
    }
    println!();
}

fn evens(limit: u32) -> impl Iterator<Item = u32> {   // return an unnamed iterator type
    (0..limit).filter(|n| n % 2 == 0)
}

fn main() {
    print_all(&[1, 2, 3]);
    println!("{:?}", evens(10).collect::<Vec<_>>());
}
```

`-> impl Iterator<Item = T>` is the idiomatic way to return an iterator without naming its (often huge) concrete type.

## Const generics

Generic over a *value*, usually an array length:

```rust
fn sum_array<const N: usize>(values: [i32; N]) -> i32 {
    values.iter().sum()
}

struct Matrix<const R: usize, const C: usize> {
    data: [[f64; C]; R],
}

impl<const R: usize, const C: usize> Matrix<R, C> {
    fn transpose(&self) -> Matrix<C, R> {
        let mut data = [[0.0; R]; C];
        for r in 0..R {
            for c in 0..C {
                data[c][r] = self.data[r][c];
            }
        }
        Matrix { data }
    }
}

fn main() {
    println!("{}", sum_array([1, 2, 3]));
    println!("{}", sum_array([10, 20, 30, 40, 50]));

    let m = Matrix { data: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]] };   // 2x3
    let t = m.transpose();                                          // 3x2
    println!("{:?}", t.data);
}
```

## `PhantomData` and type-state

Sometimes a type parameter carries meaning but no data. `PhantomData` is a zero-size marker:

```rust
use std::marker::PhantomData;

struct Meters;
struct Feet;

struct Length<Unit> {
    value: f64,
    unit: PhantomData<Unit>,
}

impl<Unit> Length<Unit> {
    fn new(value: f64) -> Self {
        Length { value, unit: PhantomData }
    }
}

impl Length<Feet> {
    fn to_meters(&self) -> Length<Meters> {
        Length::new(self.value * 0.3048)
    }
}

fn total_meters(a: &Length<Meters>, b: &Length<Meters>) -> f64 {
    a.value + b.value
}

fn main() {
    let a = Length::<Meters>::new(5.0);
    let b = Length::<Feet>::new(10.0).to_meters();
    println!("{:.3}", total_meters(&a, &b));
    // total_meters(&a, &Length::<Feet>::new(1.0))  — would not compile: mixing units
}
```

Units, states and permissions encoded in the type system cost nothing at run time. Advanced lesson 2 builds on this.

## Common mistakes

- **Missing bounds.** If you use `+`, `>`, `.clone()` or `{}` on a `T`, add the matching bound.
- **Over-constraining** with bounds you don't need — it limits callers.
- **Expecting different `T`s to unify**: `Point::new(1, 2.5)` fails because both fields are one `T`.
- **Turbofish confusion**: `"5".parse::<i32>()` and `Vec::<u8>::new()` — the `::<>` goes after the function/type name.
- **Using generics where an enum is better** (a closed set of cases you control).
- **Slow compiles from excessive generic instantiation** — occasionally use `&dyn Trait` or a non-generic inner function to cut code bloat.

## Exercises

1. Write `fn swap_pair<A, B>(pair: (A, B)) -> (B, A)`.
2. Write a generic `Stack<T>` with `push`, `pop`, `peek` and `len`.
3. Write `fn min_max<T: PartialOrd + Copy>(items: &[T]) -> Option<(T, T)>`.
4. Add `fn map<U>(self, f: fn(T) -> U) -> Wrapper<U>` to a `Wrapper<T>` struct.
5. Implement `trait Shape { type Unit; fn area(&self) -> Self::Unit; }` for two shapes with different unit types.
6. Write `fn sum_all<T: std::iter::Sum<T> + Copy>(items: &[T]) -> T`.
7. Make a `Buffer<const N: usize>` that wraps `[u8; N]` and has `fn capacity(&self) -> usize`.
8. Add a `Length<Unit>::add` method that only accepts the same `Unit`.

## Recap

- Generics (`<T>`) abstract over types; bounds (`T: Trait`, `where`) say what the type must be able to do.
- The compiler generates specialized code for each concrete type — no runtime overhead.
- Associated types (`type Item`) are chosen once per implementor; generic traits can be implemented many times.
- `impl Trait` in argument/return position is concise; use it to return iterators.
- Const generics parameterize by values; `PhantomData` carries type-level information at zero cost.

Next: [Lifetimes →](05-lifetimes.md)
