# 8. Strings & characters

## What & why

Rust has two main string types, and now that you know about ownership you can see why:

- **`String`** — an *owned*, growable, heap-allocated string. You can modify it.
- **`&str`** — a *borrowed* string slice: a view into text owned by someone else. String literals (`"hello"`) are `&'static str`, stored inside the program itself.

Both are always **valid UTF-8**. That's why you can't index a string with an integer, as this lesson explains.

## Creating strings

```rust
fn main() {
    let literal: &str = "hello";                // borrowed, fixed text
    let owned: String = String::from("hello");  // owned
    let also_owned = "hello".to_string();
    let same = "hello".to_owned();
    let formatted = format!("{literal}-{owned}-{}", 42);
    let empty = String::new();
    let repeated = "ab".repeat(3);               // ababab
    println!("{literal} {owned} {also_owned} {same} {formatted} [{empty}] {repeated}");
}
```

Escapes and multi-line literals:

```rust
fn main() {
    let quote = "She said \"hi\"\tand left.\n";
    let multi = "line one
line two";                                       // newlines are kept as written
    let joined = "no newline \
                  between these";               // trailing \ eats the newline and indentation
    let raw = r"C:\Users\ada\notes.txt";         // raw: no escapes
    let raw_quotes = r#"She said "hi""#;         // use #...# when the text contains quotes
    print!("{quote}{multi}\n{joined}\n{raw}\n{raw_quotes}\n");
}
```

## Changing a `String`

```rust
fn main() {
    let mut s = String::from("Hello");
    s.push(',');                 // append one char
    s.push_str(" world");        // append a &str
    s += "!";                    // same as push_str
    println!("{s}");             // Hello, world!

    s.insert(0, '>');
    s.insert_str(1, " ");
    println!("{s}");             // > Hello, world!

    let removed = s.pop();       // remove and return the last char
    println!("{removed:?} {s}");

    s.truncate(7);
    println!("{s}");
    s.clear();
    println!("{:?} {}", s, s.is_empty());
}
```

Concatenation with `+` **moves** the left-hand `String`:

```rust
fn main() {
    let a = String::from("Hello, ");
    let b = String::from("world");
    let c = a + &b;              // a is moved into c; b is only borrowed
    // println!("{a}");          // error: a was moved
    println!("{c} {b}");

    // For more than two pieces, format! is clearer and moves nothing.
    let d = format!("{c}/{b}");
    println!("{d}");
}
```

## Common operations

```rust
fn main() {
    let s = "Rust is fun";

    println!("{}", s.len());                  // 11 — length in BYTES, not characters
    println!("{}", s.is_empty());             // false
    println!("{}", s.to_uppercase());         // RUST IS FUN
    println!("{}", s.to_lowercase());
    println!("{}", s.starts_with("Rust"));    // true
    println!("{}", s.ends_with("fun"));       // true
    println!("{}", s.contains("is"));         // true
    println!("{:?}", s.find("is"));           // Some(5) — byte index of the first match
    println!("{}", s.replace("fun", "great"));// Rust is great
    println!("{}", "  padded \n".trim());     // padded
    println!("{:?}", " x ".trim_start());     // "x "
    println!("{}", "xxhixx".trim_matches('x'));// hi
}
```

Splitting and joining:

```rust
fn main() {
    let csv = "red,green,blue";
    let parts: Vec<&str> = csv.split(',').collect();
    println!("{parts:?}");                              // ["red", "green", "blue"]
    println!("{}", parts.join(" - "));                  // red - green - blue

    let words: Vec<&str> = "  split   on   whitespace ".split_whitespace().collect();
    println!("{words:?}");                              // ["split", "on", "whitespace"]

    for line in "one\ntwo\nthree".lines() {
        println!("> {line}");
    }

    println!("{:?}", "key=value=more".split_once('='));  // Some(("key", "value=more"))
    println!("{:?}", "a,b,c,d".splitn(2, ',').collect::<Vec<_>>()); // ["a", "b,c,d"]
}
```

`split_once` is wonderful for parsing `key=value` text.

## Why you can't write `s[0]`

<!-- check: fail -->
```rust
fn main() {
    let s = String::from("hello");
    let c = s[0];      // error[E0277]: the type `str` cannot be indexed by `{integer}`
    println!("{c}");
}
```

Strings are UTF-8: a character takes 1 to 4 bytes. An integer index would be ambiguous (byte or character?) and finding the *n*th character means walking the string, so Rust refuses and makes you say what you mean:

```rust
fn main() {
    let word = "héllo";
    println!("{} bytes, {} chars", word.len(), word.chars().count());   // 6 bytes, 5 chars

    println!("{:?}", word.chars().nth(1));        // Some('é')
    println!("{:?}", word.chars().next());        // Some('h')
    println!("{:?}", word.chars().last());        // Some('o')

    for c in word.chars() {
        print!("[{c}]");
    }
    println!();
    for b in word.bytes() {
        print!("{b} ");                           // the raw UTF-8 bytes
    }
    println!();

    let chars: Vec<char> = word.chars().collect();   // random access: chars[1]
    println!("{}", chars[1]);
}
```

## Slicing strings

You can slice by **byte range**, but the boundaries must fall on character edges or the program panics:

```rust
fn main() {
    let s = "hello world";
    println!("{}", &s[0..5]);     // hello
    println!("{}", &s[6..]);      // world
    println!("{:?}", s.get(0..3));    // Some("hel")
    println!("{:?}", s.get(0..100));  // None — no panic
}
```

<!-- check: wrap panics -->
```rust
let s = String::from("héllo");
let bad = &s[0..2];     // panics: byte index 2 is not a char boundary (é is bytes 1..3)
println!("{bad}");
```

Prefer `.get(range)` when the range comes from untrusted input.

## `char` helpers

```rust
fn main() {
    let c = 'ß';
    println!("{}", c.is_alphabetic());          // true
    println!("{}", '7'.is_ascii_digit());        // true
    println!("{:?}", '7'.to_digit(10));          // Some(7)
    println!("{:?}", std::char::from_digit(7, 10));   // Some('7')
    println!("{}", 'a'.to_ascii_uppercase());    // A
    println!("{}", 'a' as u32);                  // 97
    println!("{:?}", char::from_u32(0x1F980));   // Some('🦀')
    println!("{}", c.len_utf8());                // 2
}
```

## Converting to and from numbers

```rust
fn main() {
    let n = 42;
    let text = n.to_string();                  // "42"
    let back: i32 = text.parse().unwrap();
    println!("{text} {back}");

    println!("{:?}", "3.5".parse::<f64>());    // Ok(3.5)
    println!("{:?}", "abc".parse::<i32>().is_err());   // true
    println!("{:?}", i32::from_str_radix("ff", 16));   // Ok(255)
    println!("{}", format!("{:x} {:b} {:o}", 255, 5, 8));   // ff 101 10
}
```

## Comparing

```rust
fn main() {
    println!("{}", "apple" < "banana");              // true — bytewise lexicographic
    println!("{}", "Zebra" < "apple");               // true — uppercase sorts first
    println!("{}", "Rust".eq_ignore_ascii_case("RUST"));   // true
    println!("{}", "Rust".to_lowercase() == "rust"); // true (works for all of Unicode)
}
```

## Building strings efficiently

```rust
use std::fmt::Write;

fn main() {
    let mut out = String::with_capacity(64);   // reserve space up front
    for i in 1..=3 {
        out.push_str("Line ");
        out.push_str(&i.to_string());
        out.push('\n');
    }
    write!(out, "total: {}", 3).unwrap();      // formatted append, no temporary String
    println!("{out}");
}
```

## Common mistakes

- **Treating `len()` as a character count.** It is the byte count; use `.chars().count()`.
- **Slicing at a non-boundary**, which panics on non-ASCII text.
- **Taking `&String` or returning `String` when `&str` would do.** Accept `&str`.
- **`let s = "a" + "b";`** — you can't add two `&str`. Use `format!`, or `String::from("a") + "b"`.
- **Using `+` and then the left operand** — it was moved.
- **Comparing with `==` between `String` and `&str`** works fine (`s == "hi"`); don't convert needlessly.
- **Forgetting `trim()`** on input read with `read_line`.

## Exercises

1. Read a name and print it with only the first letter capitalised (hint: `chars()`, `next()`, `to_uppercase()`, then the rest).
2. Write `fn is_palindrome(s: &str) -> bool` that ignores case and non-letters.
3. Count the vowels in a sentence.
4. Parse `"2026-10-16"` into three integers with `split('-')`.
5. Print `len()` and `chars().count()` of `"café 🙂"` and explain both.
6. Reverse a string with `.chars().rev().collect::<String>()`.
7. Parse lines like `name=Ada` with `split_once` into `(name, value)` pairs; print `"missing ="` for lines without one.
8. Build a 5-line string with `write!` and `with_capacity`.

## Recap

- `String` is owned and growable; `&str` is a borrowed view. Both are UTF-8.
- Create with `String::from`, `.to_string()`, `format!`; grow with `push`, `push_str`, `+=`.
- `len()` is bytes; use `.chars()` for characters. No integer indexing.
- Byte-range slicing must land on character boundaries — prefer `.get()`.
- `split`, `split_once`, `lines`, `trim`, `parse` handle most text work.

Next: [Arrays, vectors & slices →](09-vectors-and-slices.md)
