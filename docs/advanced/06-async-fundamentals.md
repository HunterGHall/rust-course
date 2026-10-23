# 6. Async Rust

## What & why

Threads are great up to a few thousand. A server handling **100,000 mostly-idle connections** can't afford 100,000 OS threads (each reserves megabytes of stack and costs a context switch). **Async** lets one thread juggle many tasks: when a task would wait (for a socket, a timer, a file), it yields and another task runs.

Rust's async is unusual in three ways, and understanding them removes most of the mystery:

1. **Futures are lazy.** Calling an `async fn` runs *nothing*; it returns a value that does work only when polled.
2. **No built-in runtime.** The language provides `async`/`await` and the `Future` trait; an **executor** (usually `tokio`) actually runs futures.
3. **Zero-cost.** An `async fn` compiles into a plain state machine struct — no hidden allocation, no garbage collector.

## `async` and `.await`

```rust
async fn add_slowly(a: u32, b: u32) -> u32 {
    a + b
}

async fn compute() -> u32 {
    let x = add_slowly(1, 2).await;       // wait for the future, yielding if it isn't ready
    let y = add_slowly(x, 10).await;
    y * 2
}
```

An `async fn foo() -> T` is sugar for `fn foo() -> impl Future<Output = T>`. `.await` is only allowed inside `async` code; it suspends the current future until the awaited one finishes. If you never await or poll the future, nothing happens:

```rust
async fn say(msg: &str) {
    println!("{msg}");
}

fn main() {
    let _fut = say("this never prints");     // a future value; not polled, not run
    println!("main done");
}
```

(The compiler warns: "futures do nothing unless you `.await` or poll them".)

## What a `Future` really is

<!-- check: skip -->
```rust
// From std::future (simplified):
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

pub enum Poll<T> {
    Ready(T),
    Pending,
}
```

An executor calls `poll`. The future either finishes (`Ready(value)`) or says "not yet" (`Pending`) after arranging to be woken up: it stores the `Waker` from the `Context` and calls `wake()` when progress is possible. `Pin` (lesson 3) guarantees the state machine isn't moved between polls.

Let's implement everything by hand. First a future that completes after being polled a few times:

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct CountDown(u32);

impl Future for CountDown {
    type Output = &'static str;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<&'static str> {
        if self.0 == 0 {
            Poll::Ready("liftoff")
        } else {
            println!("  countdown at {}", self.0);
            self.0 -= 1;
            cx.waker().wake_by_ref();       // "poll me again soon"
            Poll::Pending
        }
    }
}

fn main() {}
```

Now a minimal executor, `block_on`, in about twenty lines of standard library:

```rust
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};

struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();                         // wake the parked thread
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);               // pin the future on our stack
    let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
    let mut cx = Context::from_waker(&waker);

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => thread::park(),     // sleep until something calls wake()
        }
    }
}

async fn add(a: u32, b: u32) -> u32 {
    a + b
}

async fn program() -> u32 {
    let x = add(1, 2).await;
    add(x, 10).await
}

fn main() {
    println!("{}", block_on(program()));         // 13
}
```

That is the entire contract. A real executor like `tokio` adds a task queue, a thread pool, timers and I/O readiness (epoll/kqueue/IOCP) — but the `poll`/`wake` protocol is the same.

### A timer future

Let's make a future that genuinely waits, by having a helper thread wake it:

```rust
use std::future::Future;
use std::pin::{pin, Pin};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

struct ThreadWaker(Thread);
impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) { self.0.unpark(); }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(v) = future.as_mut().poll(&mut cx) {
            return v;
        }
        thread::park();
    }
}

struct Shared {
    done: bool,
    waker: Option<Waker>,
}

struct Sleep(Arc<Mutex<Shared>>);

impl Sleep {
    fn new(duration: Duration) -> Sleep {
        let shared = Arc::new(Mutex::new(Shared { done: false, waker: None }));
        let background = Arc::clone(&shared);
        thread::spawn(move || {
            thread::sleep(duration);
            let mut state = background.lock().unwrap();
            state.done = true;
            if let Some(waker) = state.waker.take() {
                waker.wake();                        // tell the executor to poll again
            }
        });
        Sleep(shared)
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.0.lock().unwrap();
        if state.done {
            Poll::Ready(())
        } else {
            state.waker = Some(cx.waker().clone()); // remember how to wake us
            Poll::Pending
        }
    }
}

fn main() {
    let start = Instant::now();
    block_on(async {
        Sleep::new(Duration::from_millis(50)).await;
        println!("first sleep done");
        Sleep::new(Duration::from_millis(50)).await;
        println!("second sleep done");
    });
    println!("{}", start.elapsed() >= Duration::from_millis(100));   // true
}
```

The key rule: **a future returning `Pending` must have arranged for `wake()` to be called**, otherwise nobody will ever poll it again and the program hangs.

## Using a real runtime: `tokio`

In practice you add a runtime. `tokio` is the de-facto standard:

```bash
cargo add tokio --features full
```

<!-- check: skip -->
```rust
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]                                   // sets up a runtime and blocks on main()
async fn main() {
    println!("start");
    sleep(Duration::from_millis(100)).await;     // yields: other tasks can run meanwhile
    println!("after 100ms");

    let result = compute().await;
    println!("{result}");
}

async fn compute() -> u32 {
    sleep(Duration::from_millis(10)).await;
    42
}
```

`#[tokio::main]` expands to roughly `fn main() { Runtime::new().unwrap().block_on(async_main()) }`. Other runtimes exist (`async-std`, `smol`, embedded ones like `embassy`); libraries that want to work with all of them are written against `std::future::Future` and avoid runtime-specific APIs.

## Concurrency inside one task: `join!`

Awaiting one after another is sequential. To run futures **concurrently** on the same task, combine them:

<!-- check: skip -->
```rust
use std::time::{Duration, Instant};
use tokio::time::sleep;

async fn fetch(name: &str, millis: u64) -> String {
    sleep(Duration::from_millis(millis)).await;
    format!("{name} done")
}

#[tokio::main]
async fn main() {
    let start = Instant::now();

    // Sequential: ~300 ms
    let a = fetch("a", 100).await;
    let b = fetch("b", 200).await;
    println!("{a}, {b} after {:?}", start.elapsed());

    // Concurrent: ~200 ms (the longest one)
    let start = Instant::now();
    let (a, b) = tokio::join!(fetch("a", 100), fetch("b", 200));
    println!("{a}, {b} after {:?}", start.elapsed());
}
```

`join!` polls both futures in turn from a single task — no threads involved. For multiple tasks that may run in parallel on a thread pool, use `tokio::spawn` (next lesson).

## Why a future is cheap: it's a state machine

The compiler turns each `async fn` into an enum with one state per `.await` point, storing only the local variables that live across it:

```rust
use std::mem::size_of_val;

async fn small() -> u8 { 1 }

async fn big() -> usize {
    let buffer = [0u8; 1024];
    small().await;                      // `buffer` is live across this await, so it's stored in the future
    buffer.len()
}

fn main() {
    println!("{}", size_of_val(&small()) < 8);       // tiny
    println!("{}", size_of_val(&big()) >= 1024);     // contains the buffer
}
```

Consequences: futures can be large (don't hold big arrays across awaits; box recursion), they're only as expensive as what they store, and they never allocate by themselves.

### Recursion needs boxing

An `async fn` can't call itself directly (infinite size). Box the recursive call:

```rust
use std::future::Future;
use std::pin::Pin;

fn countdown(n: u32) -> Pin<Box<dyn Future<Output = u32>>> {
    Box::pin(async move {
        if n == 0 { 0 } else { n + countdown(n - 1).await }
    })
}

fn main() {
    let _ = countdown(3);       // sum(1..=3) = 6 when run by an executor
}
```

## Blocking is the cardinal sin

An executor thread runs many tasks cooperatively. If a task blocks the thread (`std::thread::sleep`, a heavy loop, a synchronous file read, `Mutex::lock` held long), **every other task on that thread stalls**.

<!-- check: skip -->
```rust
#[tokio::main]
async fn main() {
    // BAD: freezes this worker thread for a second.
    // std::thread::sleep(std::time::Duration::from_secs(1));

    // GOOD: yields to the runtime while waiting.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // CPU-heavy or blocking-API work belongs on the blocking pool:
    let digest = tokio::task::spawn_blocking(|| {
        (0..50_000_000u64).fold(0u64, |a, b| a.wrapping_add(b))
    })
    .await
    .unwrap();
    println!("{digest}");
}
```

Rule: between two `.await`s a task should run for microseconds, not milliseconds.

## `Send` and futures

On a multi-threaded runtime, a spawned future may move between threads, so it must be `Send`. A future is `Send` only if everything it holds **across an `.await`** is `Send` — an `Rc`, or a `std::sync::MutexGuard`, kept across an await makes it `!Send`:

<!-- check: skip -->
```rust
use std::rc::Rc;

async fn not_send() {
    let local = Rc::new(5);
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;   // `local` lives across this await
    println!("{local}");
}

#[tokio::main]
async fn main() {
    // tokio::spawn(not_send());   // error: future cannot be sent between threads safely
    not_send().await;              // fine: awaiting directly doesn't need Send
}
```

Fixes: drop the value before awaiting (use a block scope), use `Arc` and an async-aware mutex (`tokio::sync::Mutex`), or use a single-threaded runtime with `spawn_local`.

## Async closures and traits

Since Rust 1.85 closures can be async, and since 1.75 traits can contain `async fn`:

```rust
trait Storage {
    async fn load(&self, key: &str) -> Option<String>;
}

struct Memory;

impl Storage for Memory {
    async fn load(&self, key: &str) -> Option<String> {
        if key == "answer" { Some("42".to_string()) } else { None }
    }
}

async fn show(storage: &impl Storage, key: &str) {
    match storage.load(key).await {
        Some(v) => println!("{key} = {v}"),
        None => println!("{key} missing"),
    }
}

fn main() {
    let _ = show(&Memory, "answer");       // needs an executor to actually run
    let add = async |a: u32, b: u32| a + b;    // an async closure
    let _ = add(1, 2);
}
```

Such traits aren't `dyn`-compatible; for trait objects, return a boxed future manually or use the `async-trait` crate.

## When to use async (and when not)

| Use async when | Use threads when |
| --- | --- |
| Thousands of concurrent connections or tasks | A handful of long-running workers |
| Work is mostly waiting on I/O | Work is CPU-bound (use `rayon` for parallel loops) |
| You're already in an async ecosystem (web servers, `reqwest`, databases) | Libraries you need are blocking |

Async adds real complexity (colored functions, `Send` puzzles, runtime dependence). If a thread pool does the job, use it. A CLI tool that reads a few files does not need async.

## Common mistakes

- **Forgetting `.await`**: the future is created and silently dropped. Heed the "unused `Future`" warning.
- **Blocking inside async code** with `std::thread::sleep`, `std::fs`, or long loops.
- **Holding a `std::sync::MutexGuard` across `.await`** — makes the future `!Send` and can deadlock; use `tokio::sync::Mutex` or restructure.
- **Returning `Pending` without arranging a wake-up** in a hand-written future.
- **Expecting `async` to mean parallel.** Concurrency (interleaving) is not parallelism; `join!` runs on one task, `spawn` can use many threads.
- **Mixing runtimes** in one program (tokio timers on an async-std executor).
- **Large futures on the stack** from big locals held across awaits; box them.

## Exercises

1. Run `block_on` on an `async` block that awaits two `CountDown(3)` futures in sequence.
2. Write `async fn double_after(ms: u64, n: u32) -> u32` using the hand-written `Sleep`, and drive it with `block_on`.
3. Add `tokio` and print the time taken by three sequential vs `join!`ed 100 ms sleeps.
4. Write a `Yield` future that returns `Pending` once (waking itself) and `Ready` the second time, and use it to cooperatively split a long loop.
5. Convert a blocking function that hashes a file into an async one with `spawn_blocking`.
6. Reproduce the `!Send` error with an `Rc` held across an await, then fix it two ways.
7. Implement a simple `join2(a, b)` future combinator that polls both futures until each is `Ready` and returns a tuple.

## Recap

- `async fn` returns a lazy `Future`; `.await` suspends until it's ready; nothing runs until polled.
- A `Future` is `poll(Pin<&mut Self>, &mut Context) -> Poll<T>`; returning `Pending` requires arranging a `wake()`.
- An executor (`tokio`) polls tasks; the compiler turns async code into state machines — no hidden allocation.
- `join!` gives concurrency within a task; never block the executor thread; mind `Send` across awaits.
- Choose async for massive I/O concurrency, threads or `rayon` otherwise.

Next: [Async patterns with Tokio →](07-async-patterns-with-tokio.md)
