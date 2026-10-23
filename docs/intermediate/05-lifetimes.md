# 5. Lifetimes

## What & why

Every reference in Rust has a **lifetime**: the stretch of the program during which it's valid. Usually the compiler works them out on its own. Occasionally, when a function receives several references and returns one, it needs your help to know *which input the output borrows from*. Lifetime annotations are that help.

They don't change how long anything lives. They are **labels that describe relationships** so the borrow checker can verify your code — and so it can prove references never dangle.

## The problem lifetimes solve

<!-- check: fail -->
```rust
fn main() {
    let outer;
    {
        let inner = 5;
        outer = &inner;       // error[E0597]: `inner` does not live long enough
    }
    println!("{outer}");      // outer would point at freed memory
}
```

The borrow checker compares the lifetime of `inner` (the inner block) with where `outer` is used (after it) and rejects the program. Nothing to annotate here — it's inferred.

## When you must annotate

A function that returns a reference must say where it came from:

<!-- check: fail -->
```rust
fn longest(a: &str, b: &str) -> &str {      // error[E0106]: missing lifetime specifier
    if a.len() >= b.len() { a } else { b }
}

fn main() {
    println!("{}", longest("hello", "hi"));
}
```

"This function's return type contains a borrowed value, but the signature does not say whether it is borrowed from `a` or `b`." The fix is a named lifetime parameter:

```rust
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

fn main() {
    let first = String::from("hello");
    let result;
    {
        let second = String::from("hi");
        result = longest(&first, &second);
        println!("{result}");        // fine: both inputs are alive here
    }
}
```

Read `<'a>` as: "there is some lifetime `'a`; both inputs live at least that long, and the output is valid for (at most) that long." In practice `'a` becomes the *shorter* of the two inputs' lifetimes. That's why this fails:

<!-- check: fail -->
```rust
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

fn main() {
    let first = String::from("hello");
    let result;
    {
        let second = String::from("hi");
        result = longest(&first, &second);   // error[E0597]: `second` does not live long enough
    }
    println!("{result}");                    // result might point at `second`
}
```

## Lifetime annotations describe, they don't extend

A common misconception: `'a` can't make data live longer. It only lets the compiler check that your claims are consistent. If the claim is impossible, you get an error — never a runtime surprise.

If the output only depends on one input, say so; the other doesn't need to match:

```rust
fn first_word<'a>(text: &'a str, _separator: &str) -> &'a str {
    text.split(' ').next().unwrap_or("")
}

fn main() {
    println!("{}", first_word("hello world", " "));
}
```

## Lifetime elision: why you rarely write them

The compiler applies three rules before asking you:

1. Each reference parameter gets its own lifetime.
2. If there is exactly **one** input lifetime, the output gets it.
3. If one of the inputs is `&self` or `&mut self`, the output gets *self's* lifetime.

```rust
fn first_word(text: &str) -> &str {           // rule 2: output borrows from text
    text.split(' ').next().unwrap_or("")
}

struct Parser {
    input: String,
}

impl Parser {
    fn head(&self, other: &str) -> &str {     // rule 3: output borrows from self
        &self.input[..1]
    }
}

fn main() {
    println!("{}", first_word("a b"));
    let p = Parser { input: "xyz".into() };
    println!("{}", p.head("ignored"));
}
```

`longest` needed annotations because it has two inputs and no `self`: rules 2 and 3 don't apply.

## Structs that hold references

A struct holding a reference needs a lifetime parameter: the struct can't outlive the data it borrows.

```rust
#[derive(Debug)]
struct Excerpt<'a> {
    text: &'a str,
}

impl<'a> Excerpt<'a> {
    fn first_sentence(&self) -> &str {
        self.text.split('.').next().unwrap_or("")
    }

    fn new(text: &'a str) -> Excerpt<'a> {
        Excerpt { text }
    }
}

fn main() {
    let novel = String::from("Call me Ishmael. Some years ago...");
    let excerpt = Excerpt::new(&novel);
    println!("{}", excerpt.first_sentence());
    println!("{excerpt:?}");
}
```

`Excerpt` can't outlive `novel`. This is how you build zero-copy parsers: the tokens are `&str` slices of the original input instead of freshly allocated `String`s.

```rust
struct Tokens<'a> {
    rest: &'a str,
}

impl<'a> Iterator for Tokens<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        let trimmed = self.rest.trim_start();
        if trimmed.is_empty() {
            return None;
        }
        let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
        let (token, rest) = trimmed.split_at(end);
        self.rest = rest;
        Some(token)
    }
}

fn main() {
    let text = String::from("  zero copy   tokens ");
    let tokens: Vec<&str> = Tokens { rest: &text }.collect();
    println!("{tokens:?}");       // ["zero", "copy", "tokens"] — all slices of `text`
}
```

## `'static`

`'static` means "valid for the entire program". String literals are `&'static str`, because they live in the executable:

```rust
fn greeting() -> &'static str {
    "hello"
}

fn main() {
    println!("{}", greeting());
}
```

Beware the common (mis)advice "just add `'static`" to silence an error. It usually means "this must own its data" — and the real fix is returning a `String` or `Box` instead of a reference. A bound like `T: 'static` means "`T` contains no non-static borrows" (an owned `String` satisfies it), which is what thread-spawning APIs require.

## Returning references vs owned values

Often the easiest fix to a lifetime puzzle is to return an owned value:

```rust
fn make_greeting(name: &str) -> String {
    format!("Hello, {name}!")          // new data -> owned String, no lifetime needed
}

fn main() {
    println!("{}", make_greeting("Ada"));
}
```

You can't return a reference to something created inside the function — it would dangle:

<!-- check: fail -->
```rust
fn make_greeting(name: &str) -> &str {
    let s = format!("Hello, {name}!");
    &s                                    // error[E0515]: cannot return reference to local variable `s`
}

fn main() {
    println!("{}", make_greeting("Ada"));
}
```

## Reading lifetime errors

The most common messages and what they mean:

| Message | Usual meaning | Typical fix |
| --- | --- | --- |
| `missing lifetime specifier` | a returned reference has several possible sources | add `<'a>` and tie the output to the right input |
| `does not live long enough` | a reference is used after its owner is gone | extend the owner's scope, or clone/own the data |
| `cannot return reference to local variable` | returning a borrow of a local | return an owned value |
| `lifetime may not live long enough` | your annotations promise more than the code guarantees | fix the annotation or the code |

## Common mistakes

- **Thinking annotations extend lifetimes.** They never do.
- **Using one `'a` for everything** when inputs are independent; it needlessly ties them together.
- **Slapping `'static` on to quiet the compiler.** Own the data instead.
- **Storing references in structs too early.** Start with owned `String`/`Vec`; add lifetimes when you've measured a need for zero-copy.
- **Returning references to locals.**
- **Fighting self-referential structs** (a struct holding a reference into its own field). Rust can't express that safely in simple code; use indexes, `Rc`, or restructure.

## Exercises

1. Write `fn pick<'a>(a: &'a str, b: &'a str, first: bool) -> &'a str`.
2. Write `fn longest_line(text: &str) -> &str`. Which elision rule applies?
3. Make a struct `Highlight<'a> { word: &'a str, line: usize }` and a function that returns the highlights of one word in a text.
4. Fix the "does not live long enough" example by moving `second` to a wider scope.
5. Write `fn split_pair(s: &str) -> Option<(&str, &str)>` that splits on the first `=`.
6. Convert `Tokens` to also yield the byte offset of each token as `(usize, &'a str)`.
7. Take `fn first_char_upper(s: &str) -> &str` — explain why you can't write it to return an uppercase `&str`, and change the return type.

## Recap

- Lifetimes describe how long references are valid; annotations describe relationships, they don't extend anything.
- Elision fills them in for most functions; you annotate when several inputs could be the source of an output.
- A struct holding a reference needs `<'a>` and cannot outlive its source.
- `'static` means "lives for the whole program" — string literals, or owned data bounded by `T: 'static`.
- When in doubt, return owned data.

Next: [Closures & iterators →](06-closures-and-iterators.md)
