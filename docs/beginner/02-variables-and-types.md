# 2. Variables, mutability & types

## What & why

In Rust, variables are **immutable by default**. That's not a restriction for its own sake: when you read code and see `let x = 5`, you know `x` is 5 for its whole life. Mutation is allowed, but you must ask for it with `mut`, so every place that changes is visible.

Rust is also **statically typed**: every value has a type known at compile time. You rarely have to write them because the compiler infers them.

## `let`, `mut` and shadowing

```rust
fn main() {
    let x = 5;          // immutable
    // x = 6;           // error: cannot assign twice to immutable variable

    let mut count = 0;  // mutable
    count += 1;
    count = count * 10;
    println!("{x} {count}");   // 5 10
}
```

**Shadowing** declares a *new* variable with the same name. Unlike `mut`, it can change the type:

```rust
fn main() {
    let spaces = "   ";          // &str
    let spaces = spaces.len();   // now a usize — a brand-new variable
    println!("{spaces}");        // 3

    let x = 5;
    let x = x + 1;               // 6
    {
        let x = x * 2;           // 12, only inside this block
        println!("inner {x}");
    }
    println!("outer {x}");       // 6
}
```

Use `mut` when a value genuinely changes over time (a counter). Use shadowing to transform a value step by step (parse, then validate).

## Constants

```rust
const SECONDS_PER_DAY: u32 = 24 * 60 * 60;
static GREETING: &str = "hello";

fn main() {
    println!("{SECONDS_PER_DAY} {GREETING}");
}
```

`const` values are inlined at compile time, must have an explicit type, are `UPPER_SNAKE_CASE`, and can be declared anywhere (even outside functions). `static` is a single fixed memory location for the whole program. Prefer `const` unless you need an address.

## Scalar types

| Kind | Types |
| --- | --- |
| Signed integers | `i8` `i16` `i32` `i64` `i128` `isize` |
| Unsigned integers | `u8` `u16` `u32` `u64` `u128` `usize` |
| Floating point | `f32` `f64` |
| Boolean | `bool` (`true`, `false`) |
| Character | `char` (a Unicode scalar value, 4 bytes) |

The number is the bit width. `isize`/`usize` match the machine's pointer size and are what you use for **indexes and lengths**.

```rust
fn main() {
    let a = 42;            // i32 by default
    let b: u8 = 255;       // explicit type
    let c = 3.14;          // f64 by default
    let d = 1_000_000;     // underscores are just for reading
    let e = 0xff;          // hex
    let f = 0b1010_1010;   // binary
    let g = 0o77;          // octal
    let h = b'A';          // a u8 byte literal: 65
    let big = 10_u64;      // suffix sets the type
    let flag = true;
    let letter = 'z';
    let heart = '❤';       // char holds any Unicode scalar value
    println!("{a} {b} {c} {d} {e} {f} {g} {h} {big} {flag} {letter} {heart}");
}
```

`char` uses single quotes; strings use double quotes. They are different types.

## Type annotations and inference

```rust
fn main() {
    let guess: u32 = "42".parse().expect("not a number");
    println!("{guess}");
}
```

Without `: u32`, the compiler can't know what to parse into. When it can't infer, you tell it — either with an annotation on the variable, or with the "turbofish":

```rust
fn main() {
    let guess = "42".parse::<u32>().expect("not a number");
    println!("{guess}");
}
```

## Tuples

A fixed-size group of values that may have different types:

```rust
fn main() {
    let person = ("Ada", 36, 1.7);
    println!("{} is {}", person.0, person.1);   // access by position

    let (name, age, height) = person;           // destructure
    println!("{name} {age} {height}");

    let unit = ();                              // the empty tuple, "unit" — means "nothing"
    println!("{unit:?}");                        // ()
}
```

Functions that return nothing actually return `()`.

## Arrays (preview)

```rust
fn main() {
    let days = ["Mon", "Tue", "Wed"];
    let zeros = [0; 5];                 // [0, 0, 0, 0, 0]
    println!("{} {:?} {}", days[0], zeros, days.len());
}
```

Arrays have a fixed length known at compile time. The growable version, `Vec`, comes in lesson 9.

## Type aliases

```rust
type Meters = f64;

fn main() {
    let height: Meters = 1.82;
    println!("{height}");
}
```

An alias is just another name — `Meters` and `f64` are interchangeable.

## Common mistakes

- **Mutating without `mut`.** The error tells you to add it. Do so only if you really need to.
- **Mixing integer types.** `let a: i32 = 1; let b: i64 = 2; a + b` does not compile; Rust never converts numbers silently (lesson 3).
- **Using `i32` for indexes.** Indexes and lengths are `usize`.
- **Confusing `'a'` and `"a"`.** One is a `char`, the other a `&str`.
- **Relying on shadowing to hide bugs.** If you accidentally reuse a name, the old value vanishes quietly. Give meaningful names.
- **Forgetting that `const` needs a type**: `const MAX = 5;` is an error.

## Exercises

1. Declare an immutable `age`, try to change it, read the error, then fix it two ways (with `mut`, and with shadowing).
2. Read a number from the user as text, then shadow it into an `i32` with `parse`.
3. Create a tuple `(name, year, rating)` and print each field by position, then by destructuring.
4. Print the largest value of `u8`, `i8`, `u16` and `i32` using `u8::MAX`, `i8::MIN`, etc.
5. Write a `const` for the number of minutes in a week and print it.
6. Predict the output, then run it:
   ```rust
   let x = 2;
   let x = x * 3;
   { let x = x + 1; println!("{x}"); }
   println!("{x}");
   ```
7. Make an array of your five favourite numbers and print its length and its last element.

## Recap

- `let` is immutable, `let mut` is mutable; shadowing makes a new variable and may change the type.
- `const` needs a type and a compile-time value; use `UPPER_SNAKE_CASE`.
- Scalars: integers (`i32` default), floats (`f64` default), `bool`, `char`.
- Tuples group mixed types; arrays group same-typed values with a fixed length.
- Inference usually works; use annotations or the turbofish `::<T>` when it can't.

Next: [Numbers & math →](03-numbers-and-math.md)
