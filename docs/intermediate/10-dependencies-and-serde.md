# 10. Dependencies & serde

## What & why

Rust's standard library is deliberately small: no random numbers, no JSON, no regex, no HTTP client. Those live in **crates** published on [crates.io](https://crates.io), and Cargo downloads, builds and version-locks them for you. Learning to add a dependency, read its documentation, and use it safely is as important as the language itself.

This lesson covers how dependencies work and then walks through the most important crate in the ecosystem: **serde**, the serialization framework behind nearly all Rust JSON, TOML, YAML and binary formats.

## Adding a dependency

```bash
cargo new demo && cd demo
cargo add serde --features derive
cargo add serde_json
cargo add rand
```

`cargo add` edits `Cargo.toml` for you:

```toml
[package]
name = "demo"
version = "0.1.0"
edition = "2024"

[dependencies]
rand = "0.10"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

You can write them by hand too. The next `cargo build` fetches the crates, compiles them, and writes **`Cargo.lock`**, recording the exact version of everything.

- **Commit `Cargo.lock`** for applications (reproducible builds). Libraries traditionally leave it out.
- `cargo update` moves the lock file to the newest versions your constraints allow.
- `cargo tree` shows the dependency graph; `cargo tree -d` shows duplicates.

### Version requirements

A bare `"1.2.3"` means `^1.2.3`: any version `>= 1.2.3` and `< 2.0.0`. Cargo follows **semantic versioning** — within a major version nothing should break. (For `0.x` crates, the *minor* number plays the role of the major: `"0.10"` allows `0.10.x` only.)

| Written | Allows |
| --- | --- |
| `"1.2.3"` or `"^1.2.3"` | `>=1.2.3, <2.0.0` |
| `"0.10"` | `>=0.10.0, <0.11.0` |
| `"~1.2.3"` | `>=1.2.3, <1.3.0` |
| `"=1.2.3"` | exactly that version |
| `"*"` | anything (avoid) |

Other kinds of dependency:

```toml
[dependencies]
local-helper = { path = "../local-helper" }                    # a crate on disk
tool = { git = "https://github.com/someone/tool", branch = "main" }   # straight from git
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

[dev-dependencies]          # only for tests, examples and benchmarks
pretty_assertions = "1"

[build-dependencies]        # only for build.rs scripts
cc = "1"
```

### Features

Crates expose optional functionality as **features** so you only compile what you use. `serde`'s `derive` feature turns on the `#[derive(Serialize, Deserialize)]` macros. When a method or type is "missing", check the docs for a feature you need to enable.

## Reading crate documentation

Every published crate has generated docs at `https://docs.rs/<crate>`, and you can build docs for *your own project and all its dependencies* offline:

```bash
cargo doc --open
```

Before you adopt a crate, check: recent releases, download count, open issues, license, and how many dependencies it drags in. `cargo audit` (from `cargo install cargo-audit`) checks your lock file against the RustSec database of known vulnerabilities.

## A first dependency: random numbers

<!-- check: skip -->
```rust
use rand::RngExt;

fn main() {
    let mut rng = rand::rng();
    let roll: u32 = rng.random_range(1..=6);
    let coin: bool = rng.random_bool(0.5);
    println!("rolled {roll}, coin is {coin}");

    let mut deck: Vec<u32> = (1..=5).collect();
    use rand::seq::SliceRandom;
    deck.shuffle(&mut rng);
    println!("{deck:?}");
}
```

(Older tutorials show `thread_rng()` and `gen_range`, or `use rand::Rng`; those names changed in `rand` 0.9 and 0.10. This is a good example of why you check the docs for the version you are using.)

## serde: serialization

`serde` separates **data structures** from **formats**. You derive `Serialize` and `Deserialize` once, and any format crate (`serde_json`, `toml`, `serde_yaml`, `bincode`, `csv`...) can read and write your type.

<!-- check: skip -->
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct User {
    name: String,
    age: u32,
    email: Option<String>,
    tags: Vec<String>,
}

fn main() -> Result<(), serde_json::Error> {
    let user = User {
        name: "Ada".into(),
        age: 36,
        email: None,
        tags: vec!["math".into(), "engines".into()],
    };

    let json = serde_json::to_string(&user)?;
    println!("{json}");
    // {"name":"Ada","age":36,"email":null,"tags":["math","engines"]}

    let pretty = serde_json::to_string_pretty(&user)?;
    println!("{pretty}");

    let back: User = serde_json::from_str(&json)?;
    assert_eq!(user, back);

    // Bad input is an Err, not a crash:
    let bad: Result<User, _> = serde_json::from_str(r#"{"name": "Bob", "age": "old"}"#);
    println!("{}", bad.unwrap_err());   // invalid type: string "old", expected u32 at line 1 column 28
    Ok(())
}
```

### Customizing with attributes

<!-- check: skip -->
```rust
use serde::{Deserialize, Serialize};

fn default_port() -> u16 { 8080 }

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]            // field names in JSON become camelCase
struct Settings {
    server_name: String,                      // "serverName"

    #[serde(default = "default_port")]        // use this when the field is missing
    port: u16,

    #[serde(default)]                         // Default::default() when missing
    debug_mode: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    proxy: Option<String>,                    // omitted from output when None

    #[serde(rename = "type")]                 // 'type' is a Rust keyword
    kind: String,

    #[serde(skip)]                            // never (de)serialized
    cache: Vec<u8>,
}

fn main() {
    let s: Settings = serde_json::from_str(r#"{"serverName": "alpha", "type": "web"}"#).unwrap();
    println!("{s:?}");                        // port = 8080, debug_mode = false
    println!("{}", serde_json::to_string(&s).unwrap());
}
```

Add `#[serde(deny_unknown_fields)]` to reject JSON with fields you didn't expect.

### Enums

Enums map to JSON in several configurable ways:

<!-- check: skip -->
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]     // internally tagged
enum Event {
    Click { x: i32, y: i32 },
    Key { code: String },
    Quit,
}

fn main() {
    let events = vec![Event::Click { x: 1, y: 2 }, Event::Key { code: "A".into() }, Event::Quit];
    let json = serde_json::to_string(&events).unwrap();
    println!("{json}");
    // [{"type":"click","x":1,"y":2},{"type":"key","code":"A"},{"type":"quit"}]

    let parsed: Vec<Event> = serde_json::from_str(&json).unwrap();
    println!("{parsed:?}");
}
```

### Untyped JSON

When the shape isn't known in advance, use `serde_json::Value`:

<!-- check: skip -->
```rust
use serde_json::{json, Value};

fn main() {
    let data: Value = serde_json::from_str(r#"{"user": {"name": "Ada", "langs": ["en", "fr"]}}"#).unwrap();

    println!("{}", data["user"]["name"]);                 // "Ada"
    println!("{:?}", data["user"]["langs"][1].as_str());  // Some("fr")
    println!("{}", data["missing"]["deeper"].is_null());  // true — indexing never panics

    let built = json!({ "ok": true, "items": [1, 2, 3], "name": "demo" });
    println!("{built}");
}
```

### Reading and writing files

<!-- check: skip -->
```rust
use std::fs::File;
use std::io::{BufReader, BufWriter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Library {
    books: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lib = Library { books: vec!["SICP".into(), "TRPL".into()] };

    let writer = BufWriter::new(File::create("library.json")?);
    serde_json::to_writer_pretty(writer, &lib)?;

    let reader = BufReader::new(File::open("library.json")?);
    let loaded: Library = serde_json::from_reader(reader)?;
    println!("{loaded:?}");
    Ok(())
}
```

Streaming to and from files with `to_writer`/`from_reader` avoids building giant intermediate strings.

## Other crates worth knowing

| Crate | Use |
| --- | --- |
| `clap` | command-line argument parsing (`#[derive(Parser)]`) |
| `anyhow` / `thiserror` | error handling (lesson 8) |
| `regex` | regular expressions |
| `chrono` / `time` | dates and times |
| `tokio` | async runtime (Advanced 6–7) |
| `reqwest` | HTTP client |
| `rayon` | data-parallel iterators (`.par_iter()`) |
| `itertools` | extra iterator adapters |
| `tracing` / `log` | logging |
| `toml` | config files (what Cargo itself uses) |

## Dependency hygiene

- Prefer a few well-maintained crates over many obscure ones — every dependency is code you're trusting.
- Turn off default features you don't need: `serde_json = { version = "1", default-features = false }` (when the crate supports it).
- Keep the lock file committed (applications), and run `cargo update` + your tests periodically.
- `cargo tree -i <crate>` explains why a crate is in your build.

## Common mistakes

- **Forgetting the `derive` feature**: "cannot find derive macro `Serialize`" — add `features = ["derive"]` to serde.
- **Copying old tutorial code** that uses renamed APIs (check the crate's changelog and docs.rs for your version).
- **Using `unwrap()` on deserialization** of external data. Handle the `Err`.
- **Deserializing numbers into too-small types**, or expecting `f64` to round-trip exactly.
- **Wildcard versions** (`"*"`) that make builds non-reproducible.
- **Not enabling a needed feature**, then searching for a "missing" function that is gated behind it.

## Exercises

1. Create a new project, `cargo add rand`, and write a number-guessing game where the computer picks 1..=100.
2. Define a `Recipe { name, ingredients: Vec<String>, minutes: u32 }`, serialize it to pretty JSON, parse it back, and `assert_eq!`.
3. Make `minutes` optional with `#[serde(default)]` and show that JSON without it still loads.
4. Parse a JSON array of recipes from a file and print the names of the ones under 30 minutes.
5. Use `serde_json::Value` to read an arbitrary JSON file and print every top-level key.
6. Add the `toml` crate and load a `Config` struct from a `.toml` string with the same derives.
7. Run `cargo tree` on your project and find which crates serde pulled in.

## Recap

- Crates come from crates.io; `cargo add` edits `Cargo.toml`, `Cargo.lock` pins exact versions.
- Versions follow semver; `"1.2"` means "compatible with 1.2"; features switch optional functionality on.
- `docs.rs` and `cargo doc --open` are your reference manuals.
- `serde` derives `Serialize`/`Deserialize` once and works with every format; attributes tune names, defaults and enum layouts.
- `serde_json::Value` handles untyped JSON; `from_reader`/`to_writer` stream to files.

Next: [Testing →](11-testing.md)
