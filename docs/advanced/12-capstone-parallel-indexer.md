# 12. Capstone: parallel file indexer

## Goal

Build a concurrent full-text indexer: walk a directory tree, read and tokenize every matching file on several threads, merge the results into an inverted index, and answer multi-word queries ranked by relevance. The design goal is **a concurrent program the compiler proves race-free**, with output that is *identical* regardless of how many threads you use.

This exercises: ownership and borrowing across threads, scoped threads, atomics, channels, `Result` collection, trait-free but well-layered modules, the library + binary split, deterministic design, tests (including a property that must hold for any thread count), and measuring a speedup.

## What it does

```
$ cargo run --release -q -- --stats docs concurrency threads
   21  docs/advanced/05-threads-and-shared-state.md
   21  docs/advanced/12-capstone-parallel-indexer.md
   12  docs/advanced/06-async-fundamentals.md
    6  docs/advanced/07-async-patterns-with-tokio.md
    6  docs/advanced/index.md
    4  docs/index.md
    3  docs/advanced/04-macros.md
indexed 40 files, 58810 words (4538 distinct) in 4.2ms with 16 job(s)

$ cargo run --release -q -- --jobs 1 --stats docs concurrency threads
   21  docs/advanced/05-threads-and-shared-state.md
   ...
indexed 40 files, 58810 words (4538 distinct) in 10.9ms with 1 job(s)
```

(That output is from running it on this course's own `docs/` folder; your numbers and timings will differ.) The first number is the **score**: the total count of the query words in the file. Stats go to standard error, so standard output is byte-for-byte the same for any `--jobs` value.

## Requirements

1. `indexer [--jobs N] [--ext EXT] [--top N] [--stats] <dir> <word>...`
2. Find every file under `<dir>` with the extension (default `md`), skipping hidden directories and `target`.
3. Tokenize: lowercase alphanumeric words (underscores allowed), everything else is a separator.
4. Build an inverted index `word → [(file, count)]`.
5. A query matches files containing **all** the words; rank by total count, ties broken by path.
6. Use up to `--jobs` worker threads (default: available parallelism). Output must not depend on it.
7. Unreadable files are reported on stderr and skipped, not fatal. Invalid UTF-8 is replaced, not rejected.
8. Exit `0` if there were matches, `1` for none, `2` for usage or I/O errors.

## Setup

```bash
cargo new indexer
cd indexer
mkdir tests
```

Layout:

```
indexer/
├── Cargo.toml
├── src/
│   ├── lib.rs        # modules + re-exports
│   ├── model.rs      # tokenize, FileIndex, SearchIndex (pure, single-threaded)
│   ├── builder.rs    # discover, index_file, build_index (the concurrency)
│   └── main.rs       # CLI
└── tests/
    └── index.rs
```

No dependencies: everything is the standard library (`std::thread::scope`, `std::sync::mpsc`, `std::sync::atomic`).

## The design

The core idea is **separating the parallel part from the stateful part**:

- **Workers** only *read files and tokenize* — pure work that touches no shared state. Each produces an independent `FileIndex`.
- **One thread** (the caller) owns the `SearchIndex` and is the only one that mutates it. No `Mutex` is needed because there is no sharing: workers send results over a channel and the caller merges them.
- **Work distribution** is dynamic: each worker repeatedly takes the next unclaimed path via an `AtomicUsize` counter (`fetch_add`). That balances load automatically when file sizes vary — a worker stuck on a huge file doesn't hold up the others.
- **Determinism**: results arrive in whatever order threads finish, so each is tagged with its path's index and stored in a slot. The caller then adds files to the index **in sorted path order**. The final index is therefore identical for 1 or 100 threads.

`thread::scope` lets workers borrow the `paths` slice directly — no `Arc`, no cloning of the file list — and guarantees every worker has finished before `build_index` returns.

## `src/model.rs`

<!-- sync: capstones/file-indexer/src/model.rs -->
```rust
use std::collections::HashMap;
use std::path::PathBuf;

/// Lowercased alphanumeric words. Everything else separates words.
pub fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
}

/// Word counts for a single file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIndex {
    pub path: PathBuf,
    pub counts: HashMap<String, u32>,
    pub total_words: usize,
}

impl FileIndex {
    pub fn from_text(path: PathBuf, text: &str) -> FileIndex {
        let mut counts = HashMap::new();
        let mut total_words = 0;
        for word in tokenize(text) {
            *counts.entry(word).or_insert(0) += 1;
            total_words += 1;
        }
        FileIndex {
            path,
            counts,
            total_words,
        }
    }
}

/// One search result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: PathBuf,
    pub score: u32,
}

/// An inverted index: word -> which files contain it, and how often.
#[derive(Debug, Default)]
pub struct SearchIndex {
    files: Vec<FileIndex>,
    postings: HashMap<String, Vec<(usize, u32)>>,
}

impl SearchIndex {
    pub fn new() -> SearchIndex {
        SearchIndex::default()
    }

    pub fn add(&mut self, file: FileIndex) {
        let id = self.files.len();
        for (word, &count) in &file.counts {
            self.postings
                .entry(word.clone())
                .or_default()
                .push((id, count));
        }
        self.files.push(file);
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn word_count(&self) -> usize {
        self.postings.len()
    }

    pub fn total_words(&self) -> usize {
        self.files.iter().map(|f| f.total_words).sum()
    }

    /// Files containing *every* query word, best first (ties broken by path).
    pub fn search(&self, query: &[&str]) -> Vec<Hit> {
        let words: Vec<String> = query.iter().flat_map(|q| tokenize(q)).collect();
        if words.is_empty() {
            return Vec::new();
        }

        // Start from the first word's postings, then keep only files that also match the rest.
        let mut scores: HashMap<usize, u32> = HashMap::new();
        for (position, word) in words.iter().enumerate() {
            let Some(postings) = self.postings.get(word) else {
                return Vec::new();
            };
            if position == 0 {
                scores.extend(postings.iter().copied());
            } else {
                let here: HashMap<usize, u32> = postings.iter().copied().collect();
                scores.retain(|id, _| here.contains_key(id));
                for (id, score) in scores.iter_mut() {
                    *score += here[id];
                }
            }
        }

        let mut hits: Vec<Hit> = scores
            .into_iter()
            .map(|(id, score)| Hit {
                path: self.files[id].path.clone(),
                score,
            })
            .collect();
        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
        hits
    }
}
```
<!-- /sync -->

Things worth noticing:

- `tokenize` returns `impl Iterator<Item = String> + '_`: lazily produced words that borrow the input text — no intermediate `Vec`.
- `SearchIndex::add` takes a `FileIndex` **by value** (it keeps it), so ownership moves into the index and the type system documents that the caller can't keep using it.
- The `search` method intersects postings lists: it starts from the first word's map and `retain`s only the files that also contain each following word, summing scores.
- Sorting uses `then_with` so ties break by path: another source of determinism (iterating a `HashMap` has *no* defined order, so anything that escapes into output must be sorted).

## `src/builder.rs`

<!-- sync: capstones/file-indexer/src/builder.rs -->
```rust
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use crate::model::{FileIndex, SearchIndex};

/// Every file under `root` with the given extension (no dot), sorted by path.
pub fn discover(root: &Path, extension: &str) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    walk(root, extension, &mut found)?;
    found.sort();
    Ok(found)
}

fn walk(path: &Path, extension: &str, found: &mut Vec<PathBuf>) -> io::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if child.is_dir() && (name.starts_with('.') || name == "target") {
                continue;
            }
            walk(&child, extension, found)?;
        }
    } else if path.extension().is_some_and(|e| e == extension) {
        found.push(path.to_path_buf());
    }
    Ok(())
}

/// Reads and tokenizes one file. Invalid UTF-8 is replaced, not rejected.
pub fn index_file(path: &Path) -> io::Result<FileIndex> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(FileIndex::from_text(path.to_path_buf(), &text))
}

pub struct BuildResult {
    pub index: SearchIndex,
    pub errors: Vec<(PathBuf, io::Error)>,
}

/// Indexes `paths` using up to `jobs` worker threads.
///
/// Workers only read files and send results back over a channel; the calling
/// thread is the only one that touches the `SearchIndex`, and it adds files in
/// path order, so the result is identical for any `jobs`.
pub fn build_index(paths: &[PathBuf], jobs: usize) -> BuildResult {
    let jobs = jobs.clamp(1, paths.len().max(1));
    let next = AtomicUsize::new(0);
    let (sender, receiver) = mpsc::channel::<(usize, io::Result<FileIndex>)>();

    let mut slots: Vec<Option<io::Result<FileIndex>>> = Vec::new();
    slots.resize_with(paths.len(), || None);

    thread::scope(|scope| {
        for _ in 0..jobs {
            let sender = sender.clone();
            let next = &next;
            scope.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = paths.get(i) else { break };
                    if sender.send((i, index_file(path))).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender); // so the receiver stops when the workers finish

        for (i, result) in receiver {
            slots[i] = Some(result);
        }
    });

    let mut index = SearchIndex::new();
    let mut errors = Vec::new();
    for (path, slot) in paths.iter().zip(slots) {
        match slot.expect("every path produces exactly one result") {
            Ok(file) => index.add(file),
            Err(e) => errors.push((path.clone(), e)),
        }
    }
    BuildResult { index, errors }
}
```
<!-- /sync -->

Study `build_index` closely, because it is the point of the lesson:

- `let jobs = jobs.clamp(1, paths.len().max(1))` — never more workers than files, never zero.
- `thread::scope(|scope| { ... })` — workers may borrow `paths` and `next` from the enclosing function because the scope cannot end until they all do.
- Each worker loops: `next.fetch_add(1, Relaxed)` returns a unique index, so **no two workers ever process the same file**. `Relaxed` is enough because the counter is only used to hand out distinct numbers; the results travel through the channel, which provides the synchronization needed to publish them.
- The original `sender` is explicitly `drop`ped. A channel closes only when **every** sender is gone; if you forget this, `for (i, result) in receiver` would never end and the program would hang.
- The receive loop runs on the calling thread *inside* the scope while workers run, so results are collected as they arrive rather than at the end.
- After the scope, `slots` holds one result per path. `slot.expect(...)` can't fail: every index in `0..paths.len()` was claimed by exactly one worker and each claim sends exactly one message.
- Errors (`io::Error`) are collected next to successes and returned, rather than printed or panicked on from a worker thread.

Why not a `Mutex<SearchIndex>` that every worker updates? It would work, but all workers would contend for the lock for every file, the merge order would depend on timing, and output determinism would be gone. Message passing plus a single owner is simpler and scales better here.

## `src/lib.rs`

<!-- sync: capstones/file-indexer/src/lib.rs -->
```rust
//! A parallel full-text indexer: scoped threads read and tokenize files,
//! one thread owns the index.

pub mod builder;
pub mod model;

pub use builder::{BuildResult, build_index, discover, index_file};
pub use model::{FileIndex, Hit, SearchIndex, tokenize};
```
<!-- /sync -->

## `src/main.rs`

<!-- sync: capstones/file-indexer/src/main.rs -->
```rust
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::Instant;

use indexer::{build_index, discover};

const USAGE: &str = "usage: indexer [--jobs N] [--ext EXT] [--top N] [--stats] <dir> <word>...";

struct Options {
    root: PathBuf,
    words: Vec<String>,
    jobs: usize,
    extension: String,
    top: usize,
    stats: bool,
}

fn parse_args() -> Result<Options, String> {
    let default_jobs = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let mut jobs = default_jobs;
    let mut extension = "md".to_string();
    let mut top = 10;
    let mut stats = false;
    let mut positional = Vec::new();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--jobs" => jobs = number(&arg, args.next())?,
            "--top" => top = number(&arg, args.next())?,
            "--ext" => extension = args.next().ok_or("--ext needs a value")?,
            "--stats" => stats = true,
            _ if arg.starts_with("--") => return Err(format!("unknown option {arg}")),
            _ => positional.push(arg),
        }
    }

    if positional.len() < 2 {
        return Err(USAGE.to_string());
    }
    let root = PathBuf::from(positional.remove(0));
    Ok(Options {
        root,
        words: positional,
        jobs,
        extension,
        top,
        stats,
    })
}

fn number(flag: &str, value: Option<String>) -> Result<usize, String> {
    let value = value.ok_or(format!("{flag} needs a value"))?;
    match value.parse::<usize>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(format!("{flag} needs a positive number, got '{value}'")),
    }
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(2);
        }
    };

    let paths = match discover(&options.root, &options.extension) {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", options.root.display());
            return ExitCode::from(2);
        }
    };

    let started = Instant::now();
    let result = build_index(&paths, options.jobs);
    let elapsed = started.elapsed();

    for (path, error) in &result.errors {
        eprintln!("warning: skipped {}: {error}", path.display());
    }

    let query: Vec<&str> = options.words.iter().map(String::as_str).collect();
    let hits = result.index.search(&query);
    for hit in hits.iter().take(options.top) {
        println!("{:>5}  {}", hit.score, hit.path.display());
    }
    if hits.is_empty() {
        println!("no matches");
    }

    if options.stats {
        // Stats go to stderr so stdout stays identical for any --jobs.
        eprintln!(
            "indexed {} files, {} words ({} distinct) in {:.1?} with {} job(s)",
            result.index.file_count(),
            result.index.total_words(),
            result.index.word_count(),
            elapsed,
            options.jobs
        );
    }
    if hits.is_empty() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
```
<!-- /sync -->

The CLI parses arguments by hand (see stretch goal 8 for `clap`), uses the exit-code convention `0`/`1`/`2`, and sends stats and warnings to stderr so stdout is reproducible.

## Tests

<!-- sync: capstones/file-indexer/tests/index.rs -->
```rust
use std::fs;
use std::path::{Path, PathBuf};

use indexer::{FileIndex, SearchIndex, build_index, discover, tokenize};

/// A throwaway directory that cleans up after itself.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!("indexer-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    fn write(&self, name: &str, text: &str) {
        fs::write(self.0.join(name), text).unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn tokenizer_lowercases_and_splits() {
    let words: Vec<String> = tokenize("Hello, World! snake_case 42x").collect();
    assert_eq!(words, vec!["hello", "world", "snake_case", "42x"]);
}

#[test]
fn counts_words() {
    let file = FileIndex::from_text(PathBuf::from("a.md"), "to be or not to be");
    assert_eq!(file.total_words, 6);
    assert_eq!(file.counts["to"], 2);
    assert_eq!(file.counts["or"], 1);
}

#[test]
fn search_requires_all_words_and_ranks_by_count() {
    let mut index = SearchIndex::new();
    index.add(FileIndex::from_text(
        "a.md".into(),
        "rust rust rust ownership",
    ));
    index.add(FileIndex::from_text(
        "b.md".into(),
        "rust ownership ownership",
    ));
    index.add(FileIndex::from_text("c.md".into(), "rust only"));

    let hits = index.search(&["rust", "ownership"]);
    let found: Vec<(&Path, u32)> = hits.iter().map(|h| (h.path.as_path(), h.score)).collect();
    assert_eq!(found, vec![(Path::new("a.md"), 4), (Path::new("b.md"), 3)]);

    assert!(index.search(&["missing"]).is_empty());
    assert!(index.search(&[]).is_empty());
}

#[test]
fn same_results_for_any_job_count() {
    let dir = TempDir::new("jobs");
    for i in 0..40 {
        let text = format!("common word{} {}{}", i % 7, "rust ".repeat(i % 5), i);
        dir.write(&format!("f{i:02}.md"), &text);
    }
    dir.write("ignored.txt", "common");

    let paths = discover(&dir.0, "md").unwrap();
    assert_eq!(paths.len(), 40);

    let one = build_index(&paths, 1);
    let eight = build_index(&paths, 8);
    assert!(one.errors.is_empty() && eight.errors.is_empty());
    assert_eq!(one.index.file_count(), 40);
    assert_eq!(one.index.total_words(), eight.index.total_words());
    assert_eq!(one.index.search(&["rust"]), eight.index.search(&["rust"]));
    assert_eq!(
        one.index.search(&["common", "word3"]),
        eight.index.search(&["common", "word3"])
    );
}

#[test]
fn unreadable_files_are_reported_not_fatal() {
    let dir = TempDir::new("errors");
    dir.write("good.md", "hello");
    let mut paths = discover(&dir.0, "md").unwrap();
    paths.push(dir.0.join("vanished.md"));

    let result = build_index(&paths, 4);
    assert_eq!(result.index.file_count(), 1);
    assert_eq!(result.errors.len(), 1);
}

#[test]
fn empty_input_is_fine() {
    let result = build_index(&[], 4);
    assert_eq!(result.index.file_count(), 0);
}
```
<!-- /sync -->

The most important test is `same_results_for_any_job_count`: it builds the index with 1 and with 8 workers over 40 files and asserts the search results are equal. It's the executable form of the design claim "output does not depend on thread count".

## Try it

```bash
cargo test
cargo build --release

# Index this course's Markdown (or any project):
./target/release/indexer --stats docs concurrency actors
./target/release/indexer --jobs 1 --stats docs concurrency actors     # compare timings
./target/release/indexer --ext rs --top 5 src fn

# Prove the output doesn't depend on --jobs:
./target/release/indexer --jobs 1 docs rust > one.txt
./target/release/indexer --jobs 16 docs rust > many.txt
diff one.txt many.txt && echo identical
```

The complete, buildable project lives in [`capstones/file-indexer`](https://github.com/HunterGHall/rust-course/tree/main/capstones/file-indexer), and CI builds and tests it on Linux, macOS and Windows.

## Self-check

- [ ] `cargo build` and `cargo clippy --all-targets` show no warnings, and there is no `unsafe` and no `Mutex` in the program.
- [ ] You can explain why `index.add(file)` needs no lock (only the calling thread ever touches the index).
- [ ] You can explain why the original `sender` must be dropped, and what happens if it isn't.
- [ ] `--jobs 1` and `--jobs 8` produce byte-identical stdout on a large directory.
- [ ] You measured a speedup on a directory with thousands of files and can say what limits it (disk cache, the sequential merge, the number of cores, tokenization cost vs I/O).
- [ ] You can say what the compiler would say if a worker tried to push into the `SearchIndex` directly (it isn't `Sync`-shared and `&mut` can't be duplicated).
- [ ] You can say why `Relaxed` is sufficient for the work counter.

## Stretch goals

1. **Cancellation:** share an `AtomicBool` that workers check between files; make Ctrl-C (`ctrlc` crate) set it and print the partial results.
2. **Compare designs:** implement `Arc<Mutex<SearchIndex>>` where workers call `add` directly. Benchmark both with 20,000 small files and explain the difference.
3. **Chunking:** claim paths in batches of 32 with `fetch_add(32)` and measure whether it helps with many tiny files.
4. **`rayon`:** reimplement `build_index` with `paths.par_iter().map(index_file).collect::<Vec<_>>()` and compare code size and speed.
5. **Phrase search** (`"scoped threads"` as consecutive words) by storing token positions.
6. **Persistence:** serialize the index with `serde` + `bincode`/`serde_json` and reload it, with a version number and file-modification-time checks to re-index only changed files.
7. **Ranking:** replace raw counts with TF-IDF and test ranking quality on a small hand-labelled corpus.
8. **Use `clap`** for arguments and generate `--help`.
9. **Streaming progress:** make workers send a "file done" message so the caller can draw a progress bar on stderr.
10. **Async version:** use `tokio` with `tokio::fs` and `buffer_unordered`, and benchmark against threads. (Hint: for local disk, threads usually win.)
11. **Memory-map big files** with `memmap2` and compare.
12. **Run under Miri-friendly tests** or Loom for the work-claiming logic, and think about what a data race would look like if `next` were a plain `usize`.

## When you're done

You can design a concurrent Rust program where the compiler proves the absence of data races, test it deterministically, measure it, and explain the trade-offs. That is the summit of this course: you now know the language from `println!("Hello, world!")` to ownership, traits, lifetimes, macros, unsafe code and async.

Stop studying and go build something real. Contribute to an open-source Rust project, port a tool you rely on, or ship the one you've been putting off.
