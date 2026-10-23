# 10. Performance & profiling

## What & why

Rust can be as fast as C and C++ — but only if you measure and choose well. The language gives you control over memory layout, allocation and dispatch, and zero-cost abstractions mean idiomatic code is usually fast already. This lesson is a **method**, not a checklist:

1. **Make it correct** and clear first.
2. **Measure** with a release build on realistic input.
3. **Find the bottleneck** with a profiler (guessing is wrong most of the time).
4. **Fix the biggest thing** — usually the algorithm or the allocations — and measure again.

## Always benchmark release builds

`cargo run` and `cargo build` produce **debug** builds: no optimization, overflow checks, extra assertions. They are routinely 10–100× slower than release.

```bash
cargo build --release          # optimized: target/release/
cargo run --release
```

Tune the release profile in `Cargo.toml` when you need the last few percent:

```toml
[profile.release]
opt-level = 3          # default for release; "s"/"z" optimize for size
lto = "fat"            # link-time optimization across crates (slower builds, faster code)
codegen-units = 1      # fewer parallel units -> better optimization
panic = "abort"        # no unwinding tables; smaller and slightly faster (but panics can't be caught)
strip = true           # remove symbols for a smaller binary

[profile.profiling]    # a custom profile: release speed, with symbols for profilers
inherits = "release"
debug = true
strip = false
```

Build it with `cargo build --profile profiling`. Also consider `RUSTFLAGS="-C target-cpu=native"` for code that runs only on your own hardware (it enables the newest instruction sets, e.g. AVX2, but the binary may not run elsewhere).

## Measure with a simple harness first

Before reaching for a benchmark framework, `Instant` and `black_box` give a quick answer:

```rust
use std::hint::black_box;
use std::time::Instant;

fn sum_loop(data: &[u64]) -> u64 {
    let mut total = 0;
    for i in 0..data.len() {
        total += data[i];
    }
    total
}

fn sum_iter(data: &[u64]) -> u64 {
    data.iter().sum()
}

fn time<F: FnMut() -> u64>(label: &str, mut f: F) {
    let runs = 20;
    let mut best = std::time::Duration::MAX;
    for _ in 0..runs {
        let start = Instant::now();
        black_box(f());                      // black_box stops the optimizer deleting the work
        best = best.min(start.elapsed());
    }
    println!("{label:>10}: {best:?}");
}

fn main() {
    let data: Vec<u64> = (0..5_000_000).collect();
    time("index loop", || sum_loop(black_box(&data)));
    time("iterator", || sum_iter(black_box(&data)));
}
```

Run it with `--release`; in debug mode the numbers are meaningless. Good practice: warm up, repeat, take the **minimum** or median (not the mean), vary input sizes, and use `black_box` on inputs *and* outputs so the compiler can't precompute everything.

For serious work use a framework that handles statistics and noise: **`criterion`** or **`divan`**, run with `cargo bench`.

<!-- check: skip -->
```rust
// benches/sum.rs   (cargo add --dev divan;  [[bench]] name = "sum", harness = false)
fn main() {
    divan::main();
}

#[divan::bench(args = [1_000, 100_000, 10_000_000])]
fn sum_iter(n: u64) -> u64 {
    (0..n).sum()
}
```

## Choose the right algorithm and data structure

Nothing else comes close: an O(n²) → O(n log n) change beats any micro-optimization.

| You need | Use | Not |
| --- | --- | --- |
| Lookup by key | `HashMap` / `BTreeMap` | scanning a `Vec` of pairs (unless tiny) |
| "Have I seen this?" | `HashSet` | `vec.contains()` in a loop |
| FIFO queue | `VecDeque` | `vec.remove(0)` |
| Sorted order, ranges | `BTreeMap` / sorted `Vec` + binary search | sorting repeatedly |
| Many small inserts at the front | `VecDeque` | `Vec::insert(0, ..)` |
| Priority queue | `BinaryHeap` | re-sorting each time |

```rust
use std::collections::HashSet;

fn has_duplicates_slow(items: &[u32]) -> bool {       // O(n²)
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            if items[i] == items[j] {
                return true;
            }
        }
    }
    false
}

fn has_duplicates_fast(items: &[u32]) -> bool {       // O(n)
    let mut seen = HashSet::with_capacity(items.len());
    items.iter().any(|x| !seen.insert(*x))
}

fn main() {
    let data: Vec<u32> = (0..20_000).collect();
    println!("{}", has_duplicates_fast(&data));
    println!("{}", has_duplicates_slow(&data[..2_000]));
}
```

## Allocation: the usual suspect

Heap allocation is cheap but not free, and in a hot loop it dominates. Reduce it:

```rust
use std::fmt::Write;

fn main() {
    // 1. Reserve capacity when you know the size — avoids repeated regrowth.
    let mut v = Vec::with_capacity(1_000);
    for i in 0..1_000 {
        v.push(i);
    }

    // 2. Reuse buffers instead of allocating a new one per iteration.
    let mut line = String::new();
    let mut total_len = 0;
    for i in 0..5 {
        line.clear();                                   // keeps the allocation
        write!(line, "item number {i}").unwrap();        // formats in place; no temporary String
        total_len += line.len();
    }

    // 3. Borrow instead of clone.
    let names = vec![String::from("Ada"), String::from("Grace")];
    let longest = names.iter().max_by_key(|n| n.len()).unwrap();   // &String, no copy
    println!("{} {} {longest}", v.len(), total_len);

    // 4. Avoid collecting into a Vec just to iterate again.
    let evens_squared: u64 = (1..=10u64).filter(|n| n % 2 == 0).map(|n| n * n).sum();
    println!("{evens_squared}");
}
```

More ideas: `String::with_capacity`, `SmallVec`/`ArrayVec` (inline storage for small collections), `Cow<str>` for rarely-modified text, `Box<[T]>` instead of `Vec<T>` when the length is fixed, an **arena** (`bumpalo`) when many objects share a lifetime, and `mem::take` / `mem::replace` instead of clone-and-overwrite.

Find where allocations come from with a heap profiler (`dhat`, `heaptrack`, or `valgrind --tool=massif`), or temporarily count them with a custom global allocator:

```rust
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn count_allocations<R>(f: impl FnOnce() -> R) -> (R, usize) {
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let result = f();
    (result, ALLOCATIONS.load(Ordering::Relaxed) - before)
}

fn main() {
    let (_, without) = count_allocations(|| {
        let mut v = Vec::new();
        for i in 0..1_000 { v.push(i); }
        v.len()
    });
    let (_, with) = count_allocations(|| {
        let mut v = Vec::with_capacity(1_000);
        for i in 0..1_000 { v.push(i); }
        v.len()
    });
    println!("without capacity: {without} allocations; with capacity: {with}");   // many vs 1
}
```

## Memory layout and cache

CPUs are fast; memory is slow. Data that is **contiguous and accessed in order** (a `Vec<T>` of plain values) is dramatically faster than data scattered through pointers (`Vec<Box<T>>`, linked lists, `HashMap` of boxed things).

```rust
use std::mem::{align_of, size_of};

struct Loose {         // Rust is free to reorder fields to minimize padding...
    a: u8,
    b: u64,
    c: u8,
}

#[repr(C)]
struct InOrder {       // ...but repr(C) keeps declaration order, and padding appears
    a: u8,
    b: u64,
    c: u8,
}

fn main() {
    println!("{} {}", size_of::<Loose>(), align_of::<Loose>());      // 16 8
    println!("{} {}", size_of::<InOrder>(), align_of::<InOrder>());  // 24 8
    println!("{} {}", size_of::<Option<Box<i32>>>(), size_of::<Box<i32>>());   // 8 8 — niche optimization
    println!("{}", size_of::<Option<u32>>());                        // 8: needs a tag
}
```

Struct-of-arrays often beats array-of-structs when you process one field at a time:

```rust
struct ParticlesAoS {
    items: Vec<Particle>,                 // x, y, z, mass interleaved
}
struct Particle { x: f32, y: f32, z: f32, mass: f32 }

struct ParticlesSoA {                     // each field contiguous: great for "sum all masses"
    x: Vec<f32>,
    y: Vec<f32>,
    z: Vec<f32>,
    mass: Vec<f32>,
}

fn total_mass_aos(p: &ParticlesAoS) -> f32 { p.items.iter().map(|p| p.mass).sum() }
fn total_mass_soa(p: &ParticlesSoA) -> f32 { p.mass.iter().sum() }

fn main() {
    let aos = ParticlesAoS { items: (0..4).map(|i| Particle { x: 0.0, y: 0.0, z: 0.0, mass: i as f32 }).collect() };
    let soa = ParticlesSoA { x: vec![0.0; 4], y: vec![0.0; 4], z: vec![0.0; 4], mass: vec![0.0, 1.0, 2.0, 3.0] };
    println!("{} {}", total_mass_aos(&aos), total_mass_soa(&soa));
}
```

The SoA version touches a quarter of the memory and lets the compiler auto-vectorize with SIMD. Use it when profiling shows a hot loop over one field.

## Iterators, bounds checks and inlining

Idiomatic iterator code compiles to tight loops, often **faster** than indexing because the compiler can prove there are no out-of-range accesses and drop the bounds checks:

```rust
fn dot_indexed(a: &[f64], b: &[f64]) -> f64 {
    let mut sum = 0.0;
    for i in 0..a.len() {
        sum += a[i] * b[i];                  // bounds-checks a[i] and b[i] on every iteration
    }
    sum
}

fn dot_zipped(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()     // no bounds checks; can vectorize
}

fn main() {
    let a = vec![1.0; 1000];
    let b = vec![2.0; 1000];
    println!("{} {}", dot_indexed(&a, &b), dot_zipped(&a, &b));
}
```

Tricks when you must index: assert the lengths once up front (`let b = &b[..a.len()];`), use `chunks_exact` for SIMD-friendly blocks, or `get_unchecked` *only* after profiling and with a `// SAFETY:` comment.

**Inlining**: small functions are inlined automatically within a crate. Across crates, generic functions are inlined; non-generic ones need `#[inline]`. Don't sprinkle it everywhere; measure. Static dispatch (generics) permits inlining, dynamic dispatch (`dyn Trait`) doesn't, which matters in the hottest loops.

## I/O performance

Unbuffered I/O makes a system call per `write`. Wrap files and stdout:

```rust
use std::io::{self, BufWriter, Write};

fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());     // lock once, buffer many writes

    for i in 0..5 {
        writeln!(out, "line {i}")?;                  // no syscall per line
    }
    out.flush()?;                                    // BufWriter also flushes on drop, but errors would be lost
    Ok(())
}
```

For reading, use `BufReader` and `read_line`/`lines()`; reuse one `String` with `read_line` to avoid per-line allocation. For large files consider memory mapping (`memmap2`) or reading in big chunks.

## Parallelism

If the work is CPU-bound and independent, spread it across cores (Advanced 5): `rayon` makes `iter()` → `par_iter()` a one-word change, and scoped threads work for custom splits. Check scaling: speedup is limited by the sequential part (Amdahl's law), memory bandwidth, lock contention and false sharing.

## Profiling: find the real bottleneck

A profiler samples the program thousands of times a second and tells you where time goes.

| Platform | Tool |
| --- | --- |
| Linux | `perf record --call-graph dwarf ./target/profiling/app` then `perf report`; `cargo flamegraph` |
| macOS | Instruments (Time Profiler), `samply` |
| Windows | Visual Studio profiler, Windows Performance Analyzer, `samply` |
| Anywhere | `samply record ./app`, `cargo flamegraph`, `valgrind --tool=callgrind` (slow but precise) |

Workflow:

```bash
cargo install flamegraph
cargo flamegraph --profile profiling -- <program args>     # writes flamegraph.svg
```

In a **flame graph** each box is a function; width is the share of time (including callees); read from the bottom (callers) up. Look for wide boxes at the top — those are your hot spots. Also check:

- `cargo bloat --release` — what takes space in the binary.
- `cargo llvm-lines` — which generic functions generate the most code (compile-time and size).
- `cargo asm` / `cargo show-asm` — see the actual assembly for a function.
- `RUSTFLAGS="-Cdebuginfo=2"` — better symbols for profiles.

## Compile-time performance

Slow builds waste more developer time than slow programs. Quick wins: use `cargo check` while editing; keep dependencies lean and disable unneeded features; split large crates into a workspace so only changed crates rebuild; use `lld`/`mold` as the linker; try `sccache` in CI; and avoid enormous generic instantiations. `cargo build --timings` shows where time goes.

## A performance checklist

1. Release build? Representative input?
2. Is the algorithm / data structure right for the data size?
3. Profile: what is actually hot?
4. Allocations in the hot path? (`with_capacity`, reuse, borrow)
5. Contiguous data and sequential access?
6. Buffered I/O?
7. Can it run in parallel?
8. Measured again after each change; kept only what helped?

## Common mistakes

- **Benchmarking debug builds.**
- **Optimizing before measuring** — or optimizing code that takes 1% of the runtime.
- **Forgetting `black_box`**, so the compiler removes the benchmarked work and you "measure" zero.
- **Micro-benchmarking in isolation** with unrealistic data that fits in cache.
- **Cloning to appease the borrow checker** inside hot loops.
- **Using `unsafe` (`get_unchecked`) for "speed"** without evidence; bounds checks are often free.
- **Printing in a hot loop** (stdout is slow and line-buffered).
- **Comparing runs on a noisy machine** (laptop on battery, other programs running).

## Exercises

1. Time summing 10 million numbers with a `for i in 0..n` index loop, `iter().sum()`, and a manual 4-way unrolled loop (`--release`).
2. Make `has_duplicates_slow` and `_fast` run on 5,000, 20,000, and 80,000 elements; plot (or just tabulate) time versus size and confirm the growth rates.
3. Write a program that builds a 1,000,000-character string with repeated `+=` and `push_str`, then with `String::with_capacity`; count allocations using the counting allocator.
4. Compare iterating a `Vec<Box<u64>>` with a `Vec<u64>` summing 5 million values.
5. Print `size_of` for a struct with fields ordered `u8, u64, u8, u32` with and without `#[repr(C)]`, then reorder fields by hand.
6. Install `cargo flamegraph` and profile a program that sorts a big `Vec` of strings; identify the hottest function.
7. Wrap stdout in `BufWriter` for a program printing 1,000,000 lines and measure the difference (redirect output to `/dev/null` or `NUL`).
8. Use `cargo build --timings` on one of the capstones and read the report.

## Recap

- Measure first, in release mode, with realistic data; use `black_box` and repeat runs.
- Algorithms and data structures dominate; then allocations (capacity, reuse, borrowing); then memory layout and cache behaviour.
- Iterators are zero-cost and often remove bounds checks; generics inline, `dyn` doesn't.
- Buffer I/O; use `rayon`/threads for CPU-bound parallel work.
- Profile with `perf`, `samply` or flamegraphs; verify every optimization with another measurement.

Next: [Publishing, API design & distribution →](11-publishing-and-api-design.md)
