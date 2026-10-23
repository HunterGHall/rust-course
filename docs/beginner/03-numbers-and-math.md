# 3. Numbers & math

## What & why

Numbers are where Rust's strictness shows first. There are no implicit conversions between numeric types, and overflow is something you decide how to handle rather than something that quietly corrupts your data. Once you know the rules, they are short.

## Arithmetic

```rust
fn main() {
    println!("{}", 7 + 2);     // 9
    println!("{}", 7 - 2);     // 5
    println!("{}", 7 * 2);     // 14
    println!("{}", 7 / 2);     // 3   — integer division truncates toward zero
    println!("{}", 7 % 2);     // 1   — remainder
    println!("{}", 7.0 / 2.0); // 3.5
    println!("{}", -7 / 2);    // -3  (truncates toward zero, not floor)
    println!("{}", -7 % 2);    // -1  (remainder takes the sign of the left side)
    println!("{}", (-7_i32).rem_euclid(2)); // 1  — a "true" modulo, never negative
}
```

Compound assignment works as you'd expect: `+=`, `-=`, `*=`, `/=`, `%=`. There is no `++` or `--`; write `x += 1`.

## No implicit conversion

<!-- check: fail -->
```rust
fn main() {
    let a: i32 = 10;
    let b: i64 = 20;
    println!("{}", a + b);   // error[E0308]: mismatched types
}
```

You convert explicitly. There are three tools, from safest to loosest:

```rust
fn main() {
    let a: i32 = 10;
    let b: i64 = 20;

    // 1. From/Into — only when no information can be lost.
    let wide = i64::from(a);
    println!("{}", wide + b);                 // 30

    // 2. TryFrom/TryInto — checked; returns a Result.
    let big: i64 = 300;
    let small: Result<u8, _> = u8::try_from(big);
    println!("{:?}", small.is_err());         // true — 300 doesn't fit in a u8
    let ok: u8 = u8::try_from(200_i64).unwrap();
    println!("{ok}");

    // 3. `as` — always succeeds, may truncate or wrap.
    println!("{}", 300_i32 as u8);            // 44  (300 mod 256)
    println!("{}", -1_i32 as u32);            // 4294967295
    println!("{}", 3.99_f64 as i32);          // 3   (truncates)
    println!("{}", -5.0_f64 as u32);          // 0   (floats saturate)
    println!("{}", 10_i32 as f64 / 4.0);      // 2.5
}
```

Prefer `From`/`TryFrom`; reach for `as` only when you really want C-style truncation.

## Overflow

What happens when `255_u8 + 1`? In a **debug** build the program panics; in a **release** build it wraps around silently. Don't depend on either — pick the behaviour you want:

```rust
fn main() {
    let x: u8 = 250;

    println!("{:?}", x.checked_add(10));    // None        — overflow detected
    println!("{:?}", x.checked_add(3));     // Some(253)
    println!("{}",   x.wrapping_add(10));   // 4           — wraps around
    println!("{}",   x.saturating_add(10)); // 255         — clamps at the max
    println!("{:?}", x.overflowing_add(10));// (4, true)   — value and an overflow flag
}
```

`checked_*` is the right default for anything involving user input, money, or sizes.

<!-- check: wrap panics -->
```rust
let x: u8 = 255;
let y = std::hint::black_box(x) + 1;   // panics in debug builds: "attempt to add with overflow"
println!("{y}");
```

Integer division by zero always panics, in every build. Check first, or use `checked_div`.

## Useful integer methods

```rust
fn main() {
    println!("{}", (-5_i32).abs());            // 5
    println!("{}", 10_i32.pow(3));             // 1000
    println!("{}", 7_i32.min(3));              // 3
    println!("{}", 7_i32.max(30));             // 30
    println!("{}", 15_i32.clamp(0, 10));       // 10
    println!("{}", (-9_i32).signum());         // -1
    println!("{}", 17_u32.is_power_of_two());  // false
    println!("{}", 12_u32.count_ones());       // 2  (binary 1100)
    println!("{}", 7_u32.abs_diff(10));        // 3  — safe difference of unsigned numbers
    println!("{}", u32::MAX);                   // 4294967295
}
```

`abs_diff` avoids the classic bug where `small - big` on unsigned integers underflows.

## Floating point

```rust
fn main() {
    let x = 0.1 + 0.2;
    println!("{x}");                     // 0.30000000000000004
    println!("{}", x == 0.3);            // false — never compare floats with ==
    println!("{}", (x - 0.3_f64).abs() < 1e-9);   // true — compare with a tolerance

    println!("{}", 2.0_f64.sqrt());      // 1.4142135623730951
    println!("{}", 2.0_f64.powi(10));    // 1024
    println!("{}", 2.0_f64.powf(0.5));   // 1.4142135623730951
    println!("{}", 3.7_f64.floor());     // 3
    println!("{}", 3.2_f64.ceil());      // 4
    println!("{}", 3.5_f64.round());     // 4
    println!("{}", 3.789_f64.trunc());   // 3
    println!("{}", std::f64::consts::PI);

    let nan = f64::NAN;
    println!("{}", nan == nan);          // false — NaN is not equal to itself
    println!("{}", nan.is_nan());        // true
    println!("{}", 1.0_f64 / 0.0);       // inf
}
```

Floats have two quirks worth knowing now: `NaN != NaN`, and because of that, `f64` can't be sorted with the normal `.sort()` — you'll use `sort_by` and `total_cmp` later.

## Rounding to a number of decimals

Formatting rounds for display; it doesn't change the value:

```rust
fn main() {
    let price = 19.987_f64;
    println!("{price:.2}");                 // 19.99
    println!("{:8.3}|", price);             // "  19.987|"
    let cents = (price * 100.0).round() / 100.0;
    println!("{cents}");                    // 19.99
}
```

For money, store integers (cents) instead of floats.

## Random numbers

The standard library has no random number generator. In a real project you'd add the `rand` crate (lesson Intermediate 10). For now, a tiny deterministic generator is enough to experiment:

```rust
fn main() {
    // A linear congruential generator — fine for toys, NOT for anything secure.
    let mut seed: u64 = 12345;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as u32
    };
    for _ in 0..3 {
        println!("{}", next() % 100);
    }
}
```

(That `|| { ... }` is a closure — you'll learn those in Intermediate.)

## Common mistakes

- **Expecting `7 / 2` to be 3.5.** Both operands are integers, so the result is `3`. Use `7.0 / 2.0` or convert first.
- **Comparing floats with `==`.** Use a tolerance.
- **Using `as` to "fix" a type error** and silently truncating. Prefer `try_from`.
- **`unsigned - unsigned` going below zero.** Panics in debug, wraps in release. Use `checked_sub`, `saturating_sub` or `abs_diff`.
- **Relying on overflow behaviour** that differs between debug and release.
- **Forgetting that `%` can be negative** for negative inputs; use `rem_euclid`.

## Exercises

1. Compute the average of `3`, `4` and `8` as a floating-point number without changing the three integers' types.
2. Convert `-1_i32`, `300_i32` and `3.9_f64` with `as u8`. Explain each result, then try `u8::try_from` on the integers.
3. Use `checked_add` to write `fn safe_sum(a: u32, b: u32) -> Option<u32>`.
4. Print `u8::MAX`, `i16::MIN` and `u64::MAX`.
5. Write a program that reads Celsius and prints Fahrenheit to 1 decimal place.
6. Show that `0.1 + 0.2 != 0.3`, then write an `approx_eq(a, b)` helper.
7. Find the remainder of `-17 % 5` and `(-17_i32).rem_euclid(5)` and explain the difference.

## Recap

- Integer division truncates; `%` is a remainder, `rem_euclid` is a modulo.
- No implicit numeric conversion: use `From`, `TryFrom`, or (carefully) `as`.
- Overflow panics in debug and wraps in release — choose `checked_`, `wrapping_`, `saturating_` or `overflowing_` explicitly.
- Floats are inexact; compare with a tolerance and remember NaN.
- Format with `{:.N}`; store money as integers.

Next: [Booleans & conditionals →](04-booleans-and-conditionals.md)
