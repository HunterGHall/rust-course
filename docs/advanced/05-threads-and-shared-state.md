# 5. Concurrency with threads

## What & why

Rust's pitch is "fearless concurrency": the same ownership and borrowing rules that prevent memory bugs also prevent **data races** — two threads accessing the same memory with at least one writing and no synchronization. A data race is not just a bug in Rust; it's a **compile error** in safe code.

You get that guarantee from two marker traits:

- **`Send`** — a value of this type can be *moved* to another thread.
- **`Sync`** — a *shared reference* `&T` to this type can be used from several threads (equivalently, `T: Sync` iff `&T: Send`).

The compiler derives them automatically and checks them at every thread boundary. `Rc<T>` isn't `Send` (its counter isn't atomic); `RefCell<T>` isn't `Sync`; `Arc<Mutex<T>>` is both.

This lesson uses OS threads from `std::thread`. Lessons 6–7 cover `async`, the alternative for huge numbers of I/O-bound tasks.

## Spawning and joining

```rust
use std::thread;
use std::time::Duration;

fn main() {
    let handle = thread::spawn(|| {
        for i in 1..=3 {
            println!("worker: {i}");
            thread::sleep(Duration::from_millis(10));
        }
        42                              // the thread's return value
    });

    println!("main keeps going");
    let result = handle.join().unwrap();   // wait for it; Err if the thread panicked
    println!("worker returned {result}");
}
```

When `main` returns, the process exits and detached threads are killed, so `join` anything you need to finish.

### Moving data in

`thread::spawn` needs a `'static` closure: the thread might outlive the function that created it, so it can't borrow locals.

<!-- check: fail -->
```rust
use std::thread;

fn main() {
    let numbers = vec![1, 2, 3];
    let handle = thread::spawn(|| {
        println!("{numbers:?}");        // error[E0373]: closure may outlive the current function
    });
    handle.join().unwrap();
}
```

Use `move` to transfer ownership into the thread:

```rust
use std::thread;

fn main() {
    let numbers = vec![1, 2, 3];
    let handle = thread::spawn(move || {
        println!("{numbers:?}");
        numbers.iter().sum::<i32>()
    });
    // numbers is gone here: the thread owns it now.
    println!("sum = {}", handle.join().unwrap());
}
```

### Panics stay in the thread

```rust
use std::thread;

fn main() {
    let handle = thread::spawn(|| {
        if true {
            panic!("worker blew up");
        }
    });
    match handle.join() {
        Ok(_) => println!("fine"),
        Err(_) => println!("the worker panicked, but main is still running"),
    }
}
```

## Scoped threads

`thread::scope` guarantees all spawned threads finish before it returns, so they **can borrow locals** — no `move`, no `Arc`, no `'static`:

```rust
use std::thread;

fn main() {
    let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut results = Vec::new();

    thread::scope(|s| {
        let (left, right) = numbers.split_at(numbers.len() / 2);
        let a = s.spawn(|| left.iter().sum::<i32>());       // borrows `left`
        let b = s.spawn(|| right.iter().sum::<i32>());      // borrows `right`
        results.push(a.join().unwrap());
        results.push(b.join().unwrap());
    });

    println!("{results:?} total {}", results.iter().sum::<i32>());   // [10, 26] total 36
}
```

Scoped threads are the right tool for fork–join parallelism over local data; the file-indexer capstone uses them. Mutable data can be split into disjoint pieces for several threads:

```rust
use std::thread;

fn main() {
    let mut data = vec![1, 2, 3, 4, 5, 6];
    thread::scope(|s| {
        for chunk in data.chunks_mut(2) {       // each thread gets its own &mut slice
            s.spawn(move || {
                for x in chunk {
                    *x *= 10;
                }
            });
        }
    });
    println!("{data:?}");                        // [10, 20, 30, 40, 50, 60]
}
```

No locks, no atomics: the type system proves the chunks don't overlap.

## Message passing with channels

"Do not communicate by sharing memory; share memory by communicating." `std::sync::mpsc` is a **m**ulti-**p**roducer, **s**ingle-**c**onsumer queue:

```rust
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel();

    for id in 0..3 {
        let tx = tx.clone();                       // each producer gets its own sender
        thread::spawn(move || {
            tx.send(format!("hello from worker {id}")).unwrap();
        });
    }
    drop(tx);                                      // drop the original so the channel can close

    let mut messages: Vec<String> = rx.iter().collect();   // ends when ALL senders are dropped
    messages.sort();                                        // arrival order is nondeterministic
    for m in messages {
        println!("{m}");
    }
}
```

- `send` **moves** the value into the channel — after sending, the sender can no longer touch it.
- `recv()` blocks; `try_recv()` doesn't; `recv_timeout(d)` waits up to `d`; iterating a `Receiver` loops until every `Sender` is dropped.
- `mpsc::sync_channel(n)` is **bounded**: `send` blocks when `n` messages are queued, giving natural back-pressure.

A worker pool built on a channel of jobs:

```rust
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let (job_tx, job_rx) = mpsc::channel::<u32>();
    let (result_tx, result_rx) = mpsc::channel::<(u32, u32)>();
    let job_rx = Arc::new(Mutex::new(job_rx));             // share the single receiver

    let mut workers = Vec::new();
    for _ in 0..3 {
        let job_rx = Arc::clone(&job_rx);
        let result_tx = result_tx.clone();
        workers.push(thread::spawn(move || {
            loop {
                let job = job_rx.lock().unwrap().recv();   // lock only while waiting for a job
                match job {
                    Ok(n) => result_tx.send((n, n * n)).unwrap(),
                    Err(_) => break,                       // channel closed: no more work
                }
            }
        }));
    }
    drop(result_tx);

    for n in 1..=6 {
        job_tx.send(n).unwrap();
    }
    drop(job_tx);                                          // tell workers to finish

    let mut results: Vec<(u32, u32)> = result_rx.iter().collect();
    results.sort();
    println!("{results:?}");                               // [(1, 1), (2, 4), (3, 9), ...]
    for w in workers {
        w.join().unwrap();
    }
}
```

## Shared state: `Arc<Mutex<T>>`

Sometimes shared memory is the right model. `Arc` (atomic reference count) gives multiple ownership across threads; `Mutex` gives exclusive access:

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let counter = Arc::new(Mutex::new(0));
    let mut handles = Vec::new();

    for _ in 0..8 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                let mut guard = counter.lock().unwrap();   // blocks until we own the lock
                *guard += 1;
            }                                              // guard dropped: lock released
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    println!("{}", *counter.lock().unwrap());              // 8000, every time
}
```

Rust's `Mutex<T>` **wraps the data it protects**: you can't reach the `T` without locking, which makes "forgot to take the lock" impossible. The `MutexGuard` unlocks when dropped — keep its scope as small as you can.

Remove the `Mutex` and the compiler refuses: a plain `Arc<i32>` is read-only, and `Rc` is rejected outright because it isn't `Send`:

<!-- check: fail -->
```rust
use std::rc::Rc;
use std::thread;

fn main() {
    let shared = Rc::new(5);
    let handle = thread::spawn(move || {
        println!("{shared}");       // error[E0277]: `Rc<i32>` cannot be sent between threads safely
    });
    handle.join().unwrap();
}
```

### Poisoning

If a thread panics while holding a lock, the mutex is **poisoned**; later `lock()` calls return `Err` so you know the data may be half-updated:

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let data = Arc::new(Mutex::new(vec![1, 2, 3]));

    let d = Arc::clone(&data);
    let _ = thread::spawn(move || {
        let mut guard = d.lock().unwrap();
        guard.push(4);
        panic!("oops while holding the lock");
    })
    .join();

    match data.lock() {
        Ok(guard) => println!("fine: {guard:?}"),
        Err(poisoned) => {
            println!("poisoned!");
            let guard = poisoned.into_inner();      // recover the data anyway if you can tolerate it
            println!("{guard:?}");
        }
    }
}
```

### Deadlocks

Rust prevents data races, **not deadlocks**. If thread A holds lock 1 and waits for lock 2 while thread B holds lock 2 and waits for lock 1, both wait forever. Defences: always acquire multiple locks in a consistent order, hold locks briefly, never call unknown code (callbacks) while locked, prefer channels or a single owner thread.

<!-- check: wrap panics -->
```rust
// Re-locking a non-reentrant Mutex on the same thread is a deadlock or panic.
let m = std::sync::Mutex::new(0);
let _a = m.lock().unwrap();
let _b = m.try_lock().expect("already locked by this very thread");   // try_lock avoids hanging
```

## `RwLock`: many readers or one writer

```rust
use std::sync::{Arc, RwLock};
use std::thread;

fn main() {
    let config = Arc::new(RwLock::new(String::from("v1")));

    let readers: Vec<_> = (0..3)
        .map(|i| {
            let config = Arc::clone(&config);
            thread::spawn(move || {
                let value = config.read().unwrap();        // many readers at once
                format!("reader {i} sees {}", *value)
            })
        })
        .collect();
    for r in readers {
        println!("{}", r.join().unwrap());   // reader 0 sees v1, reader 1 sees v1, ...
    }

    *config.write().unwrap() = String::from("v2");         // exclusive writer
    println!("{}", config.read().unwrap());
}
```

Use `RwLock` when reads vastly outnumber writes and the critical section is long enough that a `Mutex` is a bottleneck; otherwise a plain `Mutex` is usually faster.

## Atomics

For single counters and flags, atomics avoid locking:

```rust
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    let hits = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));

    let workers: Vec<_> = (0..4)
        .map(|_| {
            let hits = Arc::clone(&hits);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    if hits.fetch_add(1, Ordering::Relaxed) >= 10_000 {
                        stop.store(true, Ordering::Release);
                    }
                }
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }
    println!("{}", hits.load(Ordering::Relaxed) >= 10_000);   // true
}
```

**Memory orderings** control what other memory operations become visible with the atomic one:

| Ordering | Use |
| --- | --- |
| `Relaxed` | just the atomic value itself (counters, statistics) |
| `Acquire` / `Release` | publish data: a `Release` store makes everything written before it visible to a thread whose `Acquire` load sees it |
| `AcqRel` | read-modify-write that does both |
| `SeqCst` | a single global order of all `SeqCst` operations; the simplest to reason about |

If you're unsure, use `SeqCst`; use weaker orderings only when you can explain why they're sufficient. Lock-free algorithms are hard — prefer a `Mutex` until profiling says otherwise.

## Waiting for a condition: `Condvar`

```rust
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    let worker = thread::spawn(move || {
        let (lock, cvar) = &*pair2;
        *lock.lock().unwrap() = true;       // set the condition
        cvar.notify_one();                  // wake the waiter
    });

    let (lock, cvar) = &*pair;
    let mut ready = lock.lock().unwrap();
    while !*ready {                         // loop: guards against spurious wakeups
        ready = cvar.wait(ready).unwrap();  // atomically unlocks while waiting
    }
    println!("ready: {}", *ready);
    drop(ready);
    worker.join().unwrap();
}
```

## One-time initialization and thread-locals

```rust
use std::cell::RefCell;
use std::sync::{LazyLock, OnceLock};
use std::thread;

static CONFIG: OnceLock<String> = OnceLock::new();               // set once, then read-only
static TABLE: LazyLock<Vec<u32>> = LazyLock::new(|| (0..5).map(|n| n * n).collect());   // built on first use

thread_local! {
    static COUNTER: RefCell<u32> = RefCell::new(0);              // one copy per thread
}

fn main() {
    CONFIG.get_or_init(|| String::from("loaded"));
    println!("{} {:?}", CONFIG.get().unwrap(), *TABLE);

    COUNTER.with(|c| *c.borrow_mut() += 1);
    thread::spawn(|| {
        COUNTER.with(|c| *c.borrow_mut() += 100);
        COUNTER.with(|c| println!("worker's counter = {}", c.borrow()));   // 100
    })
    .join()
    .unwrap();
    COUNTER.with(|c| println!("main's counter = {}", c.borrow()));         // 1
}
```

## Data parallelism with `rayon`

When the job is "do this to every element, using all cores", the `rayon` crate turns iterators parallel with one word:

<!-- check: skip -->
```rust
use rayon::prelude::*;

fn main() {
    let numbers: Vec<u64> = (1..=1_000_000).collect();
    let sum_of_squares: u64 = numbers.par_iter().map(|n| n * n).sum();   // iter() -> par_iter()
    println!("{sum_of_squares}");
}
```

It manages a thread pool and work stealing for you. Reach for it before hand-rolling thread pools.

## Choosing a tool

| Situation | Reach for |
| --- | --- |
| Split work over borrowed local data | `thread::scope` |
| Pipeline of stages / producer-consumer | channels |
| Same data updated by several threads | `Arc<Mutex<T>>` (or `RwLock`) |
| A counter or flag | atomics |
| Data-parallel loops | `rayon` |
| Thousands of concurrent I/O tasks | `async` (next lessons) |

## Common mistakes

- **Spawning threads without joining** and losing results or output.
- **Holding a `MutexGuard` while doing slow work** (I/O, sleeping) — it serializes everything.
- **Locking in inconsistent order** — deadlock.
- **Forgetting to drop the original `Sender`**, so `rx.iter()` never ends.
- **Using `Rc`/`RefCell` in threaded code** — use `Arc`/`Mutex`.
- **Using `Ordering::Relaxed` to publish data** — you need `Release`/`Acquire`.
- **Spawning one OS thread per tiny task.** Use a pool, `rayon`, or `async`.
- **Treating `unwrap()` on `join()` as free** — a worker panic propagates; decide how to handle it.

## Exercises

1. Compute the sum of a large `Vec<u64>` using `thread::scope` and four chunks; compare with a single-threaded sum.
2. Use `chunks_mut` and scoped threads to double every element of a vector in parallel.
3. Build a producer–consumer: three producers send numbers, one consumer prints their sum.
4. Count words in several strings concurrently, merging per-thread `HashMap`s at the end (no shared lock).
5. Make a thread-safe `Counter` struct with `increment(&self)` and `get(&self)` using `Mutex`, then again with `AtomicU64`.
6. Reproduce a deadlock with two mutexes and two threads; then fix it by ordering the locks.
7. Use `mpsc::sync_channel(2)` with a slow consumer and watch the producer block.
8. Use `Condvar` to build a blocking queue with `push` and `pop`.

## Recap

- `Send` and `Sync` make data races a compile error; `Rc` and `RefCell` are excluded from threads, `Arc` and `Mutex` are the thread-safe equivalents.
- `thread::spawn` needs `'static` (use `move`); `thread::scope` lets threads borrow locals.
- Channels move values between threads; `Arc<Mutex<T>>`/`RwLock` share state; atomics handle counters and flags.
- Mutexes can be poisoned, and deadlocks are still possible.
- Prefer scoped threads, channels, or `rayon` over hand-rolled shared-state designs.

Next: [Async Rust →](06-async-fundamentals.md)
