# 11. Publishing, API design & distribution

## What & why

Writing code that works is the beginning. Code other people depend on needs three more things: an **API** that is hard to misuse and can evolve, **documentation** that stays correct, and a **release process** that gets it to users reliably — as a library on crates.io, or as a binary on their machines.

## Designing a good public API

The official [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) are a checklist worth reading in full. The highlights:

### Naming conventions

| Pattern | Meaning | Cost | Example |
| --- | --- | --- | --- |
| `as_*` | borrowed → borrowed view | free | `str::as_bytes`, `Vec::as_slice` |
| `to_*` | borrowed → owned (or expensive conversion) | allocates | `str::to_string`, `to_vec` |
| `into_*` | owned → owned, consumes `self` | usually free | `String::into_bytes`, `into_boxed_slice` |
| `is_*` / `has_*` | boolean query | free | `is_empty`, `has_headers` |
| `iter` / `iter_mut` / `into_iter` | the three iterator flavours | free | |
| `with_*` | builder-style setter or constructor variant | | `with_capacity`, `with_timeout` |
| `try_*` | fallible version returning `Result`/`Option` | | `try_from`, `try_borrow` |
| getters | **no `get_` prefix** | | `fn name(&self) -> &str` |

### Accept general types, return specific ones

```rust
use std::path::Path;

pub struct Greeter {
    greeting: String,
}

impl Greeter {
    /// Accepts &str, String, Cow<str>, char...
    pub fn new(greeting: impl Into<String>) -> Greeter {
        Greeter { greeting: greeting.into() }
    }

    /// Accepts any iterator of anything string-like.
    pub fn greet_all<I, S>(&self, names: I) -> Vec<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        names.into_iter().map(|n| format!("{}, {}!", self.greeting, n.as_ref())).collect()
    }

    /// Accepts &str, String, &Path, PathBuf...
    pub fn extension_of(&self, path: impl AsRef<Path>) -> Option<String> {
        path.as_ref().extension().map(|e| e.to_string_lossy().into_owned())
    }
}

fn main() {
    let g = Greeter::new("Hello");
    println!("{:?}", g.greet_all(["Ada", "Grace"]));
    println!("{:?}", g.greet_all(vec![String::from("Linus")]));
    println!("{:?}", g.extension_of("notes.md"));
}
```

Take the *most general* type that does the job (`&[T]` over `&Vec<T>`, `impl AsRef<Path>` over `&str`), but return the *concrete* type you have (callers can always generalize; they can't recover lost information).

### Make invalid states unrepresentable

Use types to enforce rules: newtypes for validated values, enums for alternatives, ownership for lifecycle (Advanced lesson 2's typestate).

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidEmail;

impl fmt::Display for InvalidEmail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid email address")
    }
}
impl std::error::Error for InvalidEmail {}

impl std::str::FromStr for Email {
    type Err = InvalidEmail;

    fn from_str(s: &str) -> Result<Email, InvalidEmail> {
        match s.split_once('@') {
            Some((user, domain)) if !user.is_empty() && domain.contains('.') => Ok(Email(s.to_string())),
            _ => Err(InvalidEmail),
        }
    }
}

impl Email {
    pub fn as_str(&self) -> &str { &self.0 }
}

fn send(to: &Email) -> String {          // can't be called with an unvalidated string
    format!("sending to {}", to.as_str())
}

fn main() {
    let ok: Email = "ada@example.com".parse().unwrap();
    println!("{}", send(&ok));
    println!("{:?}", "nope".parse::<Email>());
}
```

Implementing the standard traits (`FromStr`, `Display`, `Debug`, `Default`, `From`, `Clone`, `PartialEq`, `Hash`, `Ord` where meaningful) makes your types interoperate with the whole ecosystem.

### Builders for many options

```rust
#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
    timeout_secs: u64,
    retries: u32,
    user_agent: Option<String>,
}

#[derive(Debug, Default)]
pub struct ClientBuilder {
    base_url: Option<String>,
    timeout_secs: Option<u64>,
    retries: Option<u32>,
    user_agent: Option<String>,
}

impl ClientBuilder {
    pub fn base_url(mut self, url: impl Into<String>) -> Self { self.base_url = Some(url.into()); self }
    pub fn timeout_secs(mut self, secs: u64) -> Self { self.timeout_secs = Some(secs); self }
    pub fn retries(mut self, n: u32) -> Self { self.retries = Some(n); self }
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self { self.user_agent = Some(ua.into()); self }

    pub fn build(self) -> Result<Client, String> {
        Ok(Client {
            base_url: self.base_url.ok_or("base_url is required")?,
            timeout_secs: self.timeout_secs.unwrap_or(30),
            retries: self.retries.unwrap_or(3),
            user_agent: self.user_agent,
        })
    }
}

impl Client {
    pub fn builder() -> ClientBuilder { ClientBuilder::default() }
}

fn main() {
    let client = Client::builder().base_url("https://api.example.com").retries(5).build();
    println!("{client:?}");
    println!("{:?}", Client::builder().build());           // Err("base_url is required")
}
```

Rust has no named or default arguments, so builders (and `Default` + struct update syntax for plain config structs) are the idiom.

### Plan for evolution: `#[non_exhaustive]` and private fields

Adding a variant to a public enum, or a field to a public struct, is a **breaking change** — unless you reserved the right to:

```rust
#[derive(Debug)]
#[non_exhaustive]                      // downstream `match` must include a wildcard arm
pub enum Error {
    NotFound,
    PermissionDenied,
    // adding `Timeout` later is not a breaking change
}

#[derive(Debug, Default)]
#[non_exhaustive]                      // downstream code can't build it with a struct literal
pub struct Options {
    pub verbose: bool,
    pub jobs: usize,
}

fn describe(e: &Error) -> &'static str {
    match e {
        Error::NotFound => "not found",
        Error::PermissionDenied => "denied",
        _ => "unknown",                // required for #[non_exhaustive] types from OTHER crates
    }
}

fn main() {
    println!("{}", describe(&Error::NotFound));
    println!("{:?}", Options::default());
}
```

(Within the defining crate the attribute has no effect; the wildcard is only required in other crates.)

Other evolution tools: keep fields private and expose methods; seal traits (Advanced lesson 1); return `impl Trait` instead of a concrete type; take `impl Into<..>`.

## Documentation

Doc comments (`///` for items, `//!` for the enclosing crate/module) use Markdown, and code blocks in them are **tests**:

```rust
/// A rectangle with integer sides.
///
/// # Examples
///
/// ```
/// use geometry::Rect;
///
/// let r = Rect::new(3, 4)?;
/// assert_eq!(r.area(), 12);
/// # Ok::<(), geometry::NegativeSide>(())
/// ```
///
/// # Errors
///
/// [`Rect::new`] returns [`NegativeSide`] if either side is negative.
///
/// # Panics
///
/// [`Rect::area`] panics on overflow in debug builds.
pub struct Rect { w: i32, h: i32 }

#[derive(Debug)]
pub struct NegativeSide;

impl Rect {
    pub fn new(w: i32, h: i32) -> Result<Rect, NegativeSide> {
        if w < 0 || h < 0 { Err(NegativeSide) } else { Ok(Rect { w, h }) }
    }
    pub fn area(&self) -> i32 { self.w * self.h }
}

fn main() {}
```

Conventions: a one-line summary, then details; sections named `# Examples`, `# Errors`, `# Panics`, `# Safety`. Link items with `[`Name`]`. Hide setup lines with a leading `# `. Enforce coverage in `lib.rs`:

<!-- check: skip -->
```rust
//! # geometry
//!
//! Tiny shape helpers. Start with [`Rect`].

#![warn(missing_docs)]                     // warn on any undocumented public item
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(unsafe_op_in_unsafe_fn)]
```

Build and read the docs locally with `cargo doc --open`; run the examples with `cargo test --doc`. When you publish, docs.rs builds and hosts them automatically.

## Semantic versioning

Cargo assumes [SemVer](https://semver.org): `MAJOR.MINOR.PATCH`. Consumers write `"1.2"` and expect every `1.x` release to keep working.

| Change | Version bump |
| --- | --- |
| Bug fix, internal change | patch |
| New public item, new method, new optional feature | minor |
| Remove or rename a public item; change a signature; add a required trait method; add a variant to a non-`#[non_exhaustive]` enum; raise MSRV (by convention); change behaviour incompatibly | **major** |

Less obvious breakages: adding a blanket trait impl, changing an auto-trait (`Send`/`Sync`) of a public type, making a function `const` is fine but adding a generic parameter without a default is not, tightening a generic bound. Catch them mechanically:

```bash
cargo install cargo-semver-checks
cargo semver-checks            # compares your crate with the last published version
```

For `0.x` crates, the minor number acts as the major: `0.3 → 0.4` may break.

## Cargo features

Features are optional parts of your crate. They must be **additive**: enabling a feature may only add functionality, never remove it or change behaviour incompatibly, because Cargo unifies features across the whole dependency graph.

```toml
[features]
default = ["std"]
std = []
json = ["dep:serde", "dep:serde_json"]     # optional dependencies, enabled by the feature
full = ["json", "std"]

[dependencies]
serde = { version = "1", optional = true, features = ["derive"] }
serde_json = { version = "1", optional = true }
```

<!-- check: skip -->
```rust
#[cfg(feature = "json")]
pub fn to_json(value: &impl serde::Serialize) -> serde_json::Result<String> {
    serde_json::to_string(value)
}
```

Test the combinations you promise: `cargo test --no-default-features`, `cargo test --all-features`, and (with `cargo-hack`) `cargo hack --feature-powerset check`.

## Preparing `Cargo.toml` for release

```toml
[package]
name = "geometry"                  # must be unique on crates.io
version = "0.1.0"
edition = "2024"
rust-version = "1.85"              # minimum supported Rust version (MSRV)
description = "Tiny shape helpers"
license = "MIT OR Apache-2.0"      # SPDX expression; or license-file = "LICENSE"
repository = "https://github.com/you/geometry"
readme = "README.md"
keywords = ["geometry", "shapes"]  # up to 5
categories = ["mathematics"]
exclude = ["/.github", "/benches/data"]   # keep the published package small
```

Check the package before publishing:

```bash
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps
cargo package --list      # exactly which files will be uploaded
cargo package             # builds the .crate and verifies it compiles from scratch
```

## Publishing a library

```bash
cargo login                 # paste a token from crates.io/settings/tokens
cargo publish --dry-run
cargo publish
```

Facts to know:

- Publishing is **permanent**: a version can never be changed or deleted. If a release is broken, **yank** it (`cargo yank --vers 0.1.1`) so new projects don't pick it, then publish a fixed version.
- All dependencies must also come from crates.io (no `path`/`git`-only dependencies; a `path` dependency needs a `version` too).
- Use Git tags matching versions (`v0.1.0`) and keep a `CHANGELOG.md` ("Keep a Changelog" format).
- Consider `cargo release` or `release-plz` to automate version bumps, tags and publishing.

## Distributing a binary

**Via crates.io:** `cargo install your-tool` compiles from source on the user's machine — fine for developers. `cargo binstall` fetches prebuilt binaries if you publish them.

**Prebuilt releases:** build for each platform in CI and attach the files to a GitHub Release. The `cargo-dist` tool generates this workflow (installers, archives, checksums) for you. Hand-written, it looks like:

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - { os: ubuntu-latest,  target: x86_64-unknown-linux-gnu }
          - { os: ubuntu-latest,  target: x86_64-unknown-linux-musl }
          - { os: macos-latest,   target: aarch64-apple-darwin }
          - { os: windows-latest, target: x86_64-pc-windows-msvc }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }}
      - uses: softprops/action-gh-release@v2
        with:
          files: target/${{ matrix.target }}/release/mytool*
```

### Cross-compiling

```bash
rustup target list --installed
rustup target add x86_64-unknown-linux-musl     # fully static Linux binaries
cargo build --release --target x86_64-unknown-linux-musl
```

- A `-musl` Linux target produces a binary with no libc dependency — it runs on any distribution, perfect for containers (`FROM scratch`).
- Cross-compiling pure-Rust code is easy; anything that links C code needs a cross C toolchain. The **`cross`** tool (Docker-based) and **`cargo-zigbuild`** (uses Zig as the linker) remove most of the pain.

### Containers

```dockerfile
# Build stage
FROM rust:1 AS build
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage: tiny image with just the binary
FROM debian:bookworm-slim
COPY --from=build /app/target/release/mytool /usr/local/bin/mytool
ENTRYPOINT ["mytool"]
```

Cache dependencies by copying `Cargo.toml`/`Cargo.lock` and building a dummy `main.rs` first (or use `cargo-chef`).

## Supply-chain hygiene

```bash
cargo install cargo-audit cargo-deny
cargo audit                 # known vulnerabilities in Cargo.lock (RustSec)
cargo deny check            # licenses, banned crates, duplicate versions, advisories
cargo tree -d               # duplicate dependency versions
```

Pin tools in CI, commit `Cargo.lock` for binaries, review new dependencies, and prefer crates with active maintenance and few transitive dependencies.

## A release checklist

1. Update the version in `Cargo.toml`; update `CHANGELOG.md`.
2. `cargo semver-checks` (libraries); `cargo test --all-features`; `cargo clippy`; `cargo doc`.
3. `cargo publish --dry-run`.
4. Commit, tag `vX.Y.Z`, push the tag.
5. `cargo publish`; CI builds binaries and attaches them to the GitHub Release.
6. Announce; watch for issues; yank and patch quickly if something is wrong.

## Common mistakes

- **Making everything `pub`.** Every public item is a promise. Start private; expose deliberately.
- **Breaking SemVer by accident** (a new required trait method, a changed signature, a new enum variant).
- **Non-additive features** (a feature that swaps behaviour), which break when two dependents want different settings.
- **Forgetting `license`, `description`, `repository`** (publish fails or users can't find you).
- **Publishing with `path` dependencies** or test data you didn't mean to ship (use `exclude` and `cargo package --list`).
- **Docs that don't compile** — run `cargo test --doc` in CI.
- **Not testing with the MSRV** you claim. Add a CI job with that toolchain.
- **Treating a panic as an API** — document panics, prefer `Result`.

## Exercises

1. Take the `link-checker` capstone's library and review its public API against the checklists above. What would you make private or `#[non_exhaustive]`?
2. Add `///` docs with `# Examples` to three functions and run `cargo test --doc`.
3. Add `#![warn(missing_docs)]` and fix every warning.
4. Create a `Cargo.toml` with an optional `json` feature gating a `to_json` function; test with and without it.
5. Build the file-indexer capstone for `x86_64-unknown-linux-musl` (add the target, build, and note the binary size).
6. Write a GitHub Actions workflow that runs `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test` on Linux, macOS and Windows.
7. Run `cargo package --list` on a capstone and decide what to `exclude`.
8. Design a builder for a `Server` config with `host`, `port`, and `workers`; make `build()` validate and return a `Result`.

## Recap

- Good APIs: clear naming, general inputs/specific outputs, validated newtypes, builders, and reserved room to grow (`#[non_exhaustive]`, private fields, sealed traits).
- Document with `///`, `# Examples` (compiled and tested), `# Errors`, `# Panics`, `# Safety`; enforce with `missing_docs`.
- Follow SemVer, check it with `cargo-semver-checks`, keep features additive, and declare an MSRV.
- `cargo package` then `cargo publish`; releases are permanent, so `yank` and re-release to fix.
- Ship binaries with CI release builds (`cargo-dist`), static `musl` targets, containers; audit your supply chain.

Next: [Capstone: parallel file indexer →](12-capstone-parallel-indexer.md)
