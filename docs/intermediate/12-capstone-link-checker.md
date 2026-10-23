# 12. Capstone: Markdown link checker

## Goal

Build a real tool with a proper package structure, a library/binary split, and tests. It scans Markdown files, extracts every link, checks that local files and `#anchors` exist, and prints a report with a CI-friendly exit code.

This exercises: structs and enums, `impl` blocks, `Option`/`Result`, traits (`Debug`, `PartialEq`, `Default`), iterators, string processing and slices, modules and `pub use`, `std::fs` and `Path`, and unit + integration tests.

## What it does

```
$ cargo run -q -- docs/

docs/index.md
  OK    3:./setup.md
  FAIL  4:./missng.md  (file not found)
  FAIL  7:./setup.md#instal  (no heading #instal in ./setup.md)
  SKIP  9:https://rust-lang.org  (external URL)

docs/setup.md
  OK    12:#install

Checked 5 links in 2 files: 2 OK, 2 broken, 1 skipped
$ echo $?
1
```

## Requirements

1. Recursively find `.md` files under one or more given paths (default: `.`), skipping hidden folders, `target` and `node_modules`.
2. Extract inline links `[text](target)` and images `![alt](target)` with their 1-based line numbers, ignoring anything inside fenced code blocks or `inline code`.
3. Classify each target as an external URL, a same-file `#anchor`, or a file path with an optional `#fragment`.
4. Verify local files exist; verify that anchors match a heading in the target Markdown file, using GitHub-style slugs.
5. Print a grouped report and a summary; exit `1` if anything is broken, `2` on I/O errors, `0` otherwise.
6. Do not make network requests: external URLs are reported as skipped.

## Setup

```bash
cargo new linkcheck
cd linkcheck
mkdir tests
```

The final layout:

```
linkcheck/
├── Cargo.toml
├── src/
│   ├── lib.rs        # module declarations and re-exports
│   ├── links.rs      # pure functions: classify, slugify, extract
│   ├── checker.rs    # filesystem checks and reporting
│   └── main.rs       # thin CLI
└── tests/
    ├── links.rs      # integration tests for links.rs
    └── checker.rs    # integration tests using temp directories
```

Cargo's default for a new project is a binary. Adding `src/lib.rs` next to `src/main.rs` makes the package contain **both** a library and a binary, sharing the package name. `Cargo.toml` needs no changes and has no dependencies.

## The design

- **`links.rs`** is pure: strings in, data out. `classify`, `slugify`, `heading_slugs` and `extract_links` need no files, so they're trivial to unit-test.
- **`checker.rs`** is the only module that touches the filesystem. It turns links into `Verdict`s (`Ok`, `Broken(why)`, `Skipped(why)`), then summarizes and renders them.
- **`lib.rs`** wires the modules together and re-exports the useful names so users write `linkcheck::check_file`, not `linkcheck::checker::check_file`.
- **`main.rs`** parses arguments, calls the library, prints, and chooses the exit code. It contains no logic worth testing.

Start with `links.rs`. Write the tests (further below) *first* if you like — each function takes a string and returns data, so there's nothing to mock.

## `src/links.rs`

<!-- sync: capstones/link-checker/src/links.rs -->
````rust
/// A link found in a Markdown document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub line: usize, // 1-based
    pub target: String,
}

/// What a link points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    /// `https://...`, `mailto:...`
    Url,
    /// `#heading` in the same file.
    Anchor(String),
    /// `./other.md#heading`
    File {
        path: String,
        fragment: Option<String>,
    },
}

// ---------- Classification ----------

pub fn classify(target: &str) -> LinkKind {
    if let Some(anchor) = target.strip_prefix('#') {
        return LinkKind::Anchor(anchor.to_string());
    }
    if has_scheme(target) {
        return LinkKind::Url;
    }

    let (path, fragment) = match target.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment.to_string())),
        None => (target, None),
    };
    let path = path.split('?').next().unwrap_or("");
    LinkKind::File {
        path: path.to_string(),
        fragment,
    }
}

/// "https:" and "mailto:" have a scheme; a Windows drive letter such as "C:\" does not.
fn has_scheme(text: &str) -> bool {
    let Some((scheme, _)) = text.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    scheme.len() > 1
        && chars.next().is_some_and(|c| c.is_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_alphanumeric() || "+-.".contains(c))
}

// ---------- Headings and slugs ----------

/// GitHub-style anchor: "Hello, World!" -> "hello-world".
pub fn slugify(heading: &str) -> String {
    let mut slug = String::new();
    for c in heading.to_lowercase().chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            slug.push(c);
        } else if c == ' ' {
            slug.push('-');
        }
    }
    slug
}

/// The slugs of every ATX heading (`# Title`) in a document, ignoring fenced code.
pub fn heading_slugs(markdown: &str) -> Vec<String> {
    let mut slugs = Vec::new();
    let mut in_fence = false;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) {
            if let Some(text) = trimmed[hashes..].strip_prefix(' ') {
                slugs.push(slugify(text.trim().trim_end_matches('#').trim_end()));
            }
        }
    }
    slugs
}

// ---------- Extraction ----------

/// Finds inline links `[text](target)` and images `![alt](target)`, skipping code.
pub fn extract_links(markdown: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let mut in_fence = false;

    for (index, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        let cleaned = strip_inline_code(line);
        let bytes = cleaned.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            // Look for "](" — the join between the text and the target.
            if bytes[i] == b']' && bytes[i + 1] == b'(' {
                let start = i + 2;
                if let Some(len) = cleaned[start..].find(')') {
                    let target = link_target(&cleaned[start..start + len]);
                    if !target.is_empty() {
                        links.push(Link {
                            line: index + 1,
                            target: target.to_string(),
                        });
                    }
                    i = start + len;
                    continue;
                }
            }
            i += 1;
        }
    }
    links
}

/// `path "title"` -> `path`, and strips `<...>` wrappers.
fn link_target(raw: &str) -> &str {
    let raw = raw.trim();
    let raw = match raw.find(char::is_whitespace) {
        Some(space) => &raw[..space],
        None => raw,
    };
    raw.trim_start_matches('<').trim_end_matches('>')
}

/// Replaces `inline code` with spaces so links inside it are ignored.
fn strip_inline_code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
            out.push(' ');
        } else if in_code {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}
````
<!-- /sync -->

Things worth noticing:

- `classify` returns an enum with data (`LinkKind::File { path, fragment }`), so callers can `match` exhaustively.
- `has_scheme` rejects Windows drive letters (`C:\`) by requiring the scheme to be longer than one character.
- `extract_links` works on **bytes** to find `](` quickly, but only slices the string at positions that are guaranteed to be character boundaries (`]`, `(`, `)` are ASCII).
- `strip_inline_code` blanks out `code` spans with spaces so byte offsets remain consistent.
- `link_target` returns a `&str` borrowed from its input, with no allocation; the lifetime is inferred by elision.

## `src/checker.rs`

<!-- sync: capstones/link-checker/src/checker.rs -->
```rust
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::links::{LinkKind, classify, extract_links, heading_slugs};

/// The result of checking one link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    Broken(String),
    Skipped(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkReport {
    pub line: usize,
    pub target: String,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReport {
    pub path: PathBuf,
    pub links: Vec<LinkReport>,
}

/// Every `.md` file under `root` (or `root` itself if it is a file), sorted.
pub fn find_markdown(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    collect(root, &mut found)?;
    found.sort();
    Ok(found)
}

fn collect(path: &Path, found: &mut Vec<PathBuf>) -> io::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // Skip hidden folders and build output.
            if child.is_dir()
                && (name.starts_with('.') || name == "target" || name == "node_modules")
            {
                continue;
            }
            collect(&child, found)?;
        }
    } else if path.extension().is_some_and(|e| e == "md") {
        found.push(path.to_path_buf());
    }
    Ok(())
}

/// Checks every link in one Markdown file.
pub fn check_file(path: &Path) -> io::Result<FileReport> {
    let text = fs::read_to_string(path)?;
    let own_slugs = heading_slugs(&text);
    let folder = path.parent().unwrap_or(Path::new("."));

    let mut links = Vec::new();
    for link in extract_links(&text) {
        let verdict = check_link(&link.target, folder, &own_slugs);
        links.push(LinkReport {
            line: link.line,
            target: link.target,
            verdict,
        });
    }
    Ok(FileReport {
        path: path.to_path_buf(),
        links,
    })
}

fn check_link(target: &str, folder: &Path, own_slugs: &[String]) -> Verdict {
    match classify(target) {
        LinkKind::Url => Verdict::Skipped("external URL".to_string()),
        LinkKind::Anchor(anchor) => {
            if own_slugs.contains(&anchor) {
                Verdict::Ok
            } else {
                Verdict::Broken(format!("no heading #{anchor} in this file"))
            }
        }
        LinkKind::File { path, fragment } => {
            let resolved = if path.is_empty() {
                folder.to_path_buf()
            } else {
                folder.join(&path)
            };
            if !resolved.exists() {
                return Verdict::Broken("file not found".to_string());
            }
            let Some(fragment) = fragment else {
                return Verdict::Ok;
            };
            if resolved.is_dir() || resolved.extension().is_none_or(|e| e != "md") {
                return Verdict::Ok; // anchors are only checkable in Markdown
            }
            match fs::read_to_string(&resolved) {
                Ok(text) if heading_slugs(&text).contains(&fragment) => Verdict::Ok,
                Ok(_) => Verdict::Broken(format!("no heading #{fragment} in {path}")),
                Err(e) => Verdict::Broken(format!("cannot read {path}: {e}")),
            }
        }
    }
}

// ---------- Reporting ----------

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub files: usize,
    pub ok: usize,
    pub broken: usize,
    pub skipped: usize,
}

impl Summary {
    pub fn total(&self) -> usize {
        self.ok + self.broken + self.skipped
    }
}

pub fn summarize(reports: &[FileReport]) -> Summary {
    let mut summary = Summary {
        files: reports.len(),
        ..Summary::default()
    };
    for report in reports {
        for link in &report.links {
            match link.verdict {
                Verdict::Ok => summary.ok += 1,
                Verdict::Broken(_) => summary.broken += 1,
                Verdict::Skipped(_) => summary.skipped += 1,
            }
        }
    }
    summary
}

/// Renders reports in the format shown in the lesson.
pub fn render(reports: &[FileReport]) -> String {
    let mut out = String::new();
    for report in reports {
        if report.links.is_empty() {
            continue;
        }
        out.push_str(&format!("\n{}\n", report.path.display()));
        for link in &report.links {
            let (label, note) = match &link.verdict {
                Verdict::Ok => ("OK   ", String::new()),
                Verdict::Broken(why) => ("FAIL ", format!("  ({why})")),
                Verdict::Skipped(why) => ("SKIP ", format!("  ({why})")),
            };
            out.push_str(&format!("  {label} {}:{}{note}\n", link.line, link.target));
        }
    }
    let s = summarize(reports);
    out.push_str(&format!(
        "\nChecked {} links in {} files: {} OK, {} broken, {} skipped\n",
        s.total(),
        s.files,
        s.ok,
        s.broken,
        s.skipped
    ));
    out
}
```
<!-- /sync -->

Things worth noticing:

- `find_markdown` uses a private recursive helper `collect` that takes `&mut Vec<PathBuf>`, so one vector is filled instead of allocating at every level.
- `check_link` is a pure decision over a small amount of context (the folder and the file's own slugs); it returns a `Verdict` and never prints.
- `Option::is_some_and` and `is_none_or` express path-extension checks without `match`.
- `summarize` and `render` are separate: counting and formatting can change independently.
- `Summary` derives `Default`, so `Summary { files: n, ..Summary::default() }` zeroes the counters.

## `src/lib.rs`

<!-- sync: capstones/link-checker/src/lib.rs -->
```rust
//! A tiny Markdown link checker: a testable library behind a thin CLI.

pub mod checker;
pub mod links;

pub use checker::{
    FileReport, LinkReport, Summary, Verdict, check_file, find_markdown, render, summarize,
};
pub use links::{Link, LinkKind, classify, extract_links, heading_slugs, slugify};
```
<!-- /sync -->

## `src/main.rs`

<!-- sync: capstones/link-checker/src/main.rs -->
```rust
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use linkcheck::{check_file, find_markdown, render, summarize};

fn main() -> ExitCode {
    let mut roots: Vec<PathBuf> = env::args().skip(1).map(PathBuf::from).collect();
    if roots.is_empty() {
        roots.push(PathBuf::from("."));
    }

    let mut reports = Vec::new();
    for root in &roots {
        let files = match find_markdown(root) {
            Ok(files) => files,
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", root.display());
                return ExitCode::from(2);
            }
        };
        for file in files {
            match check_file(&file) {
                Ok(report) => reports.push(report),
                Err(e) => {
                    eprintln!("error: cannot read {}: {e}", file.display());
                    return ExitCode::from(2);
                }
            }
        }
    }

    print!("{}", render(&reports));
    if summarize(&reports).broken > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
```
<!-- /sync -->

The binary imports the library by the **package name** (`linkcheck`), exactly as an outside user would.

## Tests

### `tests/links.rs`

<!-- sync: capstones/link-checker/tests/links.rs -->
````rust
use linkcheck::{LinkKind, classify, extract_links, heading_slugs, slugify};

#[test]
fn slugify_matches_github_style() {
    assert_eq!(slugify("Hello, World!"), "hello-world");
    assert_eq!(slugify("Install & Setup"), "install--setup");
    assert_eq!(slugify("snake_case-name"), "snake_case-name");
}

#[test]
fn classifies_targets() {
    assert_eq!(classify("https://rust-lang.org"), LinkKind::Url);
    assert_eq!(classify("mailto:a@b.c"), LinkKind::Url);
    assert_eq!(classify("#setup"), LinkKind::Anchor("setup".into()));
    assert_eq!(
        classify("./guide.md#install"),
        LinkKind::File {
            path: "./guide.md".into(),
            fragment: Some("install".into())
        }
    );
    assert_eq!(
        classify("img.png?raw=1"),
        LinkKind::File {
            path: "img.png".into(),
            fragment: None
        }
    );
}

#[test]
fn windows_drive_letter_is_not_a_scheme() {
    assert!(matches!(classify(r"C:\notes.md"), LinkKind::File { .. }));
}

#[test]
fn finds_links_with_line_numbers() {
    let doc = "# Title\n\nSee [one](a.md) and [two](b.md#x).\n![pic](img.png)\n";
    let links = extract_links(doc);
    let found: Vec<(usize, &str)> = links.iter().map(|l| (l.line, l.target.as_str())).collect();
    assert_eq!(found, vec![(3, "a.md"), (3, "b.md#x"), (4, "img.png")]);
}

#[test]
fn ignores_code() {
    let doc = "Use `[x](nope.md)` inline.\n\n```\n[y](nope2.md)\n```\n[real](ok.md)\n";
    let links = extract_links(doc);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "ok.md");
}

#[test]
fn strips_titles_and_angle_brackets() {
    let links = extract_links("[a](page.md \"A title\") [b](<other.md>)");
    let targets: Vec<&str> = links.iter().map(|l| l.target.as_str()).collect();
    assert_eq!(targets, vec!["page.md", "other.md"]);
}

#[test]
fn headings_skip_fenced_code() {
    let doc = "# One\n\n```\n# not a heading\n```\n\n## Two ##\n#hashtag\n";
    assert_eq!(heading_slugs(doc), vec!["one", "two"]);
}
````
<!-- /sync -->

### `tests/checker.rs`

<!-- sync: capstones/link-checker/tests/checker.rs -->
```rust
use std::fs;
use std::path::PathBuf;

use linkcheck::{Verdict, check_file, find_markdown, summarize};

/// A throwaway directory that cleans up after itself.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!("linkcheck-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn verdicts_for_each_kind_of_link() {
    let dir = TempDir::new("verdicts");
    dir.write("other.md", "# Install\n");
    let index = dir.write(
        "index.md",
        "# Top\n[ok](other.md) [missing](nope.md) [anchor](other.md#install) \
         [bad](other.md#instal) [self](#top) [ext](https://rust-lang.org)\n",
    );

    let report = check_file(&index).unwrap();
    let verdicts: Vec<&Verdict> = report.links.iter().map(|l| &l.verdict).collect();

    assert_eq!(verdicts[0], &Verdict::Ok);
    assert!(matches!(verdicts[1], Verdict::Broken(why) if why == "file not found"));
    assert_eq!(verdicts[2], &Verdict::Ok);
    assert!(matches!(verdicts[3], Verdict::Broken(_)));
    assert_eq!(verdicts[4], &Verdict::Ok);
    assert!(matches!(verdicts[5], Verdict::Skipped(_)));

    let summary = summarize(&[report]);
    assert_eq!((summary.ok, summary.broken, summary.skipped), (3, 2, 1));
}

#[test]
fn finds_markdown_recursively_and_sorted() {
    let dir = TempDir::new("walk");
    fs::create_dir_all(dir.0.join("sub")).unwrap();
    fs::create_dir_all(dir.0.join(".hidden")).unwrap();
    dir.write("b.md", "");
    dir.write("a.md", "");
    dir.write("notes.txt", "");
    fs::write(dir.0.join("sub").join("c.md"), "").unwrap();
    fs::write(dir.0.join(".hidden").join("d.md"), "").unwrap();

    let names: Vec<String> = find_markdown(&dir.0)
        .unwrap()
        .iter()
        .map(|p| {
            p.strip_prefix(&dir.0)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    assert_eq!(names, vec!["a.md", "b.md", "sub/c.md"]);
}
```
<!-- /sync -->

The `TempDir` helper creates a unique directory per test (the process id keeps parallel `cargo test` runs apart) and removes it in `Drop`, so even a failing test cleans up.

## Try it

```bash
cargo test
cargo run -q -- docs/
cargo run -q -- README.md docs/index.md
cargo build --release
./target/release/linkcheck .
```

Point it at this course's own `docs/` folder — or any project's — and fix what it finds.

The complete, buildable project lives in [`capstones/link-checker`](https://github.com/HunterGHall/rust-course/tree/main/capstones/link-checker), and CI builds and tests it on Linux, macOS and Windows.

## Self-check

- [ ] `cargo test` passes and `cargo clippy --all-targets` is quiet.
- [ ] You can explain why the logic lives in a library crate and why `main.rs` is tiny.
- [ ] You can say which functions are pure and which touch the filesystem.
- [ ] Adding a new `Verdict` variant makes the compiler point at every `match` you must update.
- [ ] You can explain why `link_target` returns `&str` without needing an explicit lifetime annotation.
- [ ] No `unwrap()` appears in `src/` (tests may use it).

## Stretch goals

1. **Reference-style links** (`[text][ref]` with `[ref]: target` definitions).
2. **Check images and HTML** `<a href="...">` / `<img src="...">` too.
3. **Duplicate headings:** GitHub appends `-1`, `-2` to repeated slugs — implement it and test it.
4. **`--quiet` and `--format json`** flags; add `serde` and `serde_json` and print machine-readable output.
5. **Parallelism:** check files on several threads (after Advanced lesson 5, or with `rayon`).
6. **External URLs:** with `--check-urls`, make HEAD requests (e.g. with `ureq`) and report status codes, with a timeout and a cache so each URL is fetched once.
7. **A `--fix` mode** that suggests the closest existing heading for a broken anchor, using edit distance.
8. **Use `clap`** to replace the hand-written argument loop and generate `--help`.
9. **Property tests** with `proptest`: `slugify` is idempotent; `extract_links` never panics on arbitrary text.

## When you're done

You've built a real, tested command-line tool with a clean architecture, and you understand why Rust code is easy to refactor: the compiler tells you everything you broke. Then go to [Advanced](../advanced/index.md).
