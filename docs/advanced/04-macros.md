# 4. Macros

## What & why

You've been calling macros since lesson 1: `println!`, `vec!`, `assert_eq!`, `format!`. A macro is code that **writes code** at compile time. Rust needs macros because functions can't do everything: they can't take a variable number of arguments, can't generate new items (structs, impls, tests), can't inspect their arguments' syntax.

There are two families:

| Family | Written as | Works on |
| --- | --- | --- |
| **Declarative** macros | `macro_rules!` | patterns of tokens |
| **Procedural** macros | a Rust function in its own crate | the full token stream |

Procedural macros come in three flavours: custom `#[derive(...)]`, attribute macros (`#[route(GET, "/")]`), and function-like macros (`sql!(...)`).

Macros are powerful and also harder to read, debug and maintain than functions. The rule: **use a function when you can; reach for a macro when you must.**

## `macro_rules!` basics

A macro is a set of rules: `(pattern) => { expansion }`. The compiler matches the tokens you pass against each pattern and substitutes the first that fits.

```rust
macro_rules! square {
    ($x:expr) => {
        $x * $x
    };
}

macro_rules! say_hello {
    () => {
        println!("hello!")
    };
    ($name:expr) => {
        println!("hello, {}!", $name)
    };
}

fn main() {
    println!("{}", square!(7));
    say_hello!();
    say_hello!("Ada");
}
```

Invoke with `()`, `[]` or `{}` — they're equivalent (`vec![..]` just conventionally uses brackets).

### Fragment specifiers

`$name:kind` captures a piece of syntax. The common kinds:

| Kind | Matches | Example |
| --- | --- | --- |
| `expr` | an expression | `1 + 2`, `foo(x)` |
| `ident` | an identifier | `my_var`, `Point` |
| `ty` | a type | `Vec<i32>` |
| `pat` | a pattern | `Some(x)` |
| `stmt` | a statement | `let a = 1` |
| `block` | a `{ ... }` block | `{ a + b }` |
| `item` | an item (fn, struct, impl) | `fn f() {}` |
| `path` | a path | `std::fmt::Display` |
| `literal` | a literal | `42`, `"hi"` |
| `tt` | a single token tree (anything) | the escape hatch |
| `vis` | a visibility | `pub`, `pub(crate)` |
| `lifetime` | a lifetime | `'a` |

### Repetition

`$( ... ) sep rep` repeats a pattern: `*` zero or more, `+` one or more, `?` zero or one. The optional separator goes before the operator.

```rust
macro_rules! my_vec {
    () => { Vec::new() };
    ( $( $x:expr ),+ $(,)? ) => {          // one or more comma-separated exprs, optional trailing comma
        {
            let mut v = Vec::new();
            $( v.push($x); )+              // repeat the push once per captured expression
            v
        }
    };
}

macro_rules! hashmap {
    ( $( $key:expr => $value:expr ),* $(,)? ) => {{
        let mut map = std::collections::HashMap::new();
        $( map.insert($key, $value); )*
        map
    }};
}

macro_rules! max {
    ($x:expr) => { $x };
    ($x:expr, $($rest:expr),+) => {
        { let a = $x; let b = max!($($rest),+); if a > b { a } else { b } }   // recursion
    };
}

fn main() {
    let v: Vec<i32> = my_vec![1, 2, 3,];
    println!("{v:?}");

    let ages = hashmap! { "Ada" => 36, "Grace" => 85 };
    println!("{}", ages["Grace"]);

    println!("{}", max!(3, 9, 4));
}
```

Note the double braces `{{ ... }}` in `hashmap!`: the outer braces delimit the rule's expansion, the inner pair makes the expansion a single block expression so the `let` statements stay local.

## Hygiene

Variables created inside a macro are in their own scope; they don't collide with the caller's:

```rust
macro_rules! double_it {
    ($e:expr) => {{
        let tmp = $e;           // this `tmp` is invisible to the caller
        tmp * 2
    }};
}

fn main() {
    let tmp = 100;
    println!("{}", double_it!(tmp + 1));    // 202 — not confused by the caller's `tmp`
}
```

Hygiene applies to local variables and labels, **not** to items (functions, types), which are global. To let the caller name something, take an `ident`:

```rust
macro_rules! make_getter {
    ($field:ident : $ty:ty) => {
        fn $field(&self) -> &$ty {
            &self.$field
        }
    };
}

struct User {
    name: String,
    age: u32,
}

impl User {
    make_getter!(name: String);
    make_getter!(age: u32);
}

fn main() {
    let u = User { name: "Ada".into(), age: 36 };
    println!("{} {}", u.name(), u.age());
}
```

## Generating items

Macros shine at removing boilerplate that functions can't:

```rust
macro_rules! newtype {
    ($(#[$meta:meta])* $vis:vis struct $name:ident($inner:ty);) => {
        $(#[$meta])*
        $vis struct $name($inner);

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self { $name(value) }
        }

        impl std::ops::Deref for $name {
            type Target = $inner;
            fn deref(&self) -> &$inner { &self.0 }
        }
    };
}

newtype! {
    #[derive(Debug, Clone)]
    pub struct UserName(String);
}

macro_rules! unit_tests {
    ($( $name:ident : $input:expr => $expected:expr ),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(super::slug($input), $expected);
            }
        )*
    };
}

fn slug(s: &str) -> String {
    s.to_lowercase().replace(' ', "-")
}

#[cfg(test)]
mod tests {
    unit_tests! {
        simple: "Hello World" => "hello-world",
        already: "done" => "done",
    }
}

fn main() {
    let name: UserName = String::from("ada").into();
    println!("{:?} {}", name, name.len());     // Deref gives us String's methods
    println!("{}", slug("A B"));
}
```

Another common one: an enum together with its string conversions, from a single list of variants.

```rust
macro_rules! string_enum {
    ($name:ident { $( $variant:ident => $text:expr ),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum $name {
            $( $variant ),+
        }

        impl $name {
            const ALL: &'static [$name] = &[ $( $name::$variant ),+ ];

            fn as_str(&self) -> &'static str {
                match self {
                    $( $name::$variant => $text ),+
                }
            }

            fn parse(s: &str) -> Option<$name> {
                match s {
                    $( $text => Some($name::$variant), )+
                    _ => None,
                }
            }
        }
    };
}

string_enum!(Color { Red => "red", Green => "green", Blue => "blue" });

fn main() {
    println!("{}", Color::Green.as_str());
    println!("{:?}", Color::parse("blue"));
    println!("{}", Color::ALL.len());
}
```

Adding a variant is one line, and the list, the `match`es and the parser can never get out of sync.

## `tt` munchers and internal rules

The `tt` fragment matches any single token tree, which lets a macro consume input one piece at a time ("tt-muncher") and recurse. Prefix helper rules with `@name` so they can't be called by accident:

```rust
macro_rules! calculate {
    (eval $e:expr) => {{
        let value: i64 = $e;
        println!("{} = {}", stringify!($e), value);
    }};
    (eval $e:expr, $(eval $es:expr),+) => {{
        calculate! { eval $e }
        calculate! { $(eval $es),+ }
    }};
}

macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}

macro_rules! sum_pairs {
    (@acc $total:expr ;) => { $total };
    (@acc $total:expr ; $a:expr, $b:expr $(, $rest:expr)*) => {
        sum_pairs!(@acc $total + $a * $b ; $($rest),*)
    };
    ($($x:expr),*) => { sum_pairs!(@acc 0 ; $($x),*) };
}

fn main() {
    calculate! { eval 1 + 2, eval (3 + 4) * 5 }
    println!("{}", count!(a b c d));
    println!("{}", sum_pairs!(1, 2, 3, 4));     // 1*2 + 3*4 = 14
}
```

`stringify!` turns tokens into a string; `concat!`, `line!`, `file!`, `env!`, `include_str!` and `cfg!` are other built-in macros you'll use.

## Scoping and exporting

`macro_rules!` macros are textually scoped: usable only *after* their definition in the file. To share a macro across modules or crates:

```rust
mod macros {
    #[macro_export]                 // makes it available as `crate::shout!` (at the crate root)
    macro_rules! shout {
        ($s:expr) => { println!("{}!", $s.to_uppercase()) };
    }
}

fn main() {
    shout!("hello");
}
```

Alternatively `#[macro_use]` on a module, or (preferred in modern code) `pub(crate) use my_macro;` after the definition.

## Debugging macros

- `cargo expand` (install `cargo-expand`) prints your code after macro expansion.
- `rustc -Zunpretty=expanded` does the same on nightly.
- `trace_macros!` and `log_syntax!` are nightly aids.
- Read the error that points at the **macro invocation**, then at the "in this macro invocation" note.
- Prefer simple, small macros; call a helper function from the expansion for anything non-trivial.

## Procedural macros

Procedural macros are ordinary Rust functions that take a `TokenStream` and return a `TokenStream`. They must live in a crate with `proc-macro = true`, and almost always use two helper crates: **`syn`** (parses tokens into a syntax tree) and **`quote`** (turns a template back into tokens).

```
describe/                 # workspace
├── Cargo.toml
├── describe_derive/      # the proc-macro crate
│   ├── Cargo.toml
│   └── src/lib.rs
└── app/                  # uses it
    ├── Cargo.toml
    └── src/main.rs
```

`describe_derive/Cargo.toml`:

```toml
[package]
name = "describe_derive"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true

[dependencies]
syn = "2"
quote = "1"
proc-macro2 = "1"
```

`describe_derive/src/lib.rs` — a derive macro that adds a method listing a struct's field names:

<!-- check: skip -->
```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Describe)]
pub fn derive_describe(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let field_names: Vec<String> = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .map(|f| f.ident.as_ref().unwrap().to_string())
                .collect(),
            _ => Vec::new(),
        },
        _ => {
            return syn::Error::new_spanned(&ast.ident, "Describe only supports structs")
                .to_compile_error()
                .into();
        }
    };

    let type_name = name.to_string();
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn describe() -> String {
                format!("{} {{ {} }}", #type_name, [#(#field_names),*].join(", "))
            }
        }
    }
    .into()
}
```

`app/src/main.rs`:

<!-- check: skip -->
```rust
use describe_derive::Describe;

#[derive(Describe)]
struct User {
    name: String,
    age: u32,
}

fn main() {
    println!("{}", User::describe());      // User { name, age }
}
```

The three kinds:

| Kind | Declaration | Used as |
| --- | --- | --- |
| Derive | `#[proc_macro_derive(Name)]` | `#[derive(Name)]` |
| Attribute | `#[proc_macro_attribute]` | `#[name(args)] fn ...` |
| Function-like | `#[proc_macro]` | `name!(...)` |

Practical advice:

- Produce **good error messages** with `syn::Error::new_spanned(...)` instead of panicking; the compiler underlines the user's code.
- Test macros with the `trybuild` crate (compile-fail tests) and plain unit tests of the generated code.
- Compile times grow with `syn`'s `full` feature — enable only what you need.
- Most of the time you'll *use* proc-macros written by others: `serde`, `clap`, `thiserror`, `tokio::main`.

## Common mistakes

- **Using a macro where a generic function works.** Functions give better errors, type checking and IDE support.
- **Evaluating an `$expr` argument twice** inside the expansion (`$x * $x` runs `x` twice; bind it to a `let` first when it may have side effects).
- **Forgetting a separator or `$(,)?`** and then failing on a trailing comma.
- **Missing hygiene for items:** two expansions defining the same function name collide.
- **Using the macro before its definition** (`macro_rules!` is textual).
- **Writing recursive macros that hit the recursion limit** (default 128): `#![recursion_limit = "256"]`.
- **Giving proc-macro crates normal dependencies on the crate that uses them** (cyclic). Keep the runtime types in a separate crate.

## Exercises

1. Write `min!` like `max!` above, accepting one or more expressions.
2. Write `square_all!(a, b, c)` expanding to `[a * a, b * b, c * c]`.
3. Write `impl_area!` that implements an `Area` trait for several shape structs from one list of `(Type, |self| expression)` pairs.
4. Write `bitflags_lite!` that generates `const` values `1 << 0`, `1 << 1`, ... for a list of names.
5. Write a `check!(cond, "message {}", args...)` macro that panics with file and line info (`file!()`, `line!()`).
6. Install `cargo-expand` and expand `string_enum!` to see what it generates.
7. Build the `Describe` derive in a workspace, then extend it to also list field types with `stringify!`/`quote!`.

## Recap

- Macros generate code at compile time: variadic arguments, new items, repetitive impls.
- `macro_rules!` matches token patterns with fragment specifiers (`expr`, `ident`, `ty`, `tt`...) and repeats with `$( ... ),*`.
- Hygiene protects local variables but not items; `@internal` rules and `tt` munchers enable advanced patterns.
- Procedural macros (derive, attribute, function-like) use `syn` + `quote` in a dedicated `proc-macro` crate.
- Prefer functions; use macros for what functions cannot do, and debug them with `cargo expand`.

Next: [Concurrency with threads →](05-threads-and-shared-state.md)
