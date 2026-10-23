# 7. Async patterns with Tokio

## What & why

Lesson 6 showed what a future is. Real async programs are built from a few recurring patterns: spawning concurrent **tasks**, waiting for **several things at once** (`select!`), imposing **timeouts**, passing messages between tasks, limiting concurrency, shutting down cleanly, and talking over the network. This lesson works through each with `tokio`.

Set up a project:

```bash
cargo new async-demo && cd async-demo
cargo add tokio --features full
cargo add futures
```

All examples below use `#[tokio::main]` unless noted.

## Tasks: `tokio::spawn`

`spawn` hands a future to the runtime, which runs it independently — on any worker thread. It returns a `JoinHandle` you can await for the result:

<!-- check: skip -->
```rust
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let mut handles = Vec::new();
    for id in 0..3u64 {
        handles.push(tokio::spawn(async move {
            sleep(Duration::from_millis(30 * (3 - id))).await;   // later ids finish first
            id * 10
        }));
    }

    for h in handles {
        println!("{}", h.await.unwrap());        // 0, 10, 20 — awaited in spawn order
    }
}
```

Requirements: the future must be `Send + 'static`, so use `async move` and own (or `Arc`) what it needs. `h.await` returns `Err(JoinError)` if the task panicked or was aborted. A spawned task keeps running even if you drop its `JoinHandle` (it's detached); call `handle.abort()` to cancel it.

### `JoinSet`: a managed group of tasks

<!-- check: skip -->
```rust
use tokio::task::JoinSet;

#[tokio::main]
async fn main() {
    let mut set = JoinSet::new();
    for n in 1..=5u32 {
        set.spawn(async move { n * n });
    }

    let mut results = Vec::new();
    while let Some(done) = set.join_next().await {     // results arrive in completion order
        results.push(done.unwrap());
    }
    results.sort();
    println!("{results:?}");                           // [1, 4, 9, 16, 25]
}
```

Dropping a `JoinSet` aborts all its tasks — convenient for scoped concurrency. `futures::future::join_all(vec_of_futures)` is the lighter option when you don't need `spawn`.

## `select!`: race futures

`tokio::select!` waits on several futures and runs the branch of whichever completes first; **the others are dropped** (cancelled):

<!-- check: skip -->
```rust
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    tokio::select! {
        _ = sleep(Duration::from_millis(50)) => println!("timer won"),
        _ = sleep(Duration::from_millis(500)) => println!("slow timer won"),
    }
}
```

## Timeouts

<!-- check: skip -->
```rust
use std::time::Duration;
use tokio::time::{sleep, timeout};

async fn slow_operation() -> &'static str {
    sleep(Duration::from_millis(500)).await;
    "finished"
}

#[tokio::main]
async fn main() {
    match timeout(Duration::from_millis(100), slow_operation()).await {
        Ok(value) => println!("got {value}"),
        Err(_elapsed) => println!("timed out"),      // the inner future was dropped
    }
}
```

## Cancellation is dropping

Async cancellation has no special mechanism: **dropping a future stops it** at its current `.await`, running destructors of everything it held. That's elegant, and it has consequences:

- Code after an `.await` may *never run*. Don't rely on "cleanup at the end of an async fn"; put cleanup in `Drop` impls.
- `select!` branches must be **cancel-safe**: losing the race shouldn't lose data. `mpsc::Receiver::recv()` and `sleep()` are cancel-safe; reading into a buffer with `read_exact` is not (some bytes may be consumed).

<!-- check: skip -->
```rust
use std::time::Duration;
use tokio::time::sleep;

struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) { println!("cleanup {}", self.0); }
}

#[tokio::main]
async fn main() {
    let task = tokio::spawn(async {
        let _guard = Guard("worker");
        println!("working...");
        sleep(Duration::from_secs(10)).await;
        println!("never printed");
    });

    sleep(Duration::from_millis(50)).await;
    task.abort();                                   // cancels at the await; Drop still runs
    println!("{}", task.await.unwrap_err().is_cancelled());   // true
}
```

## Channels between tasks

`tokio::sync` has async-aware channels (their `send`/`recv` await instead of blocking the thread):

| Channel | Shape | Use |
| --- | --- | --- |
| `mpsc` | many producers → one consumer, bounded queue | work queues, event streams |
| `oneshot` | one value, one time | reply to a request |
| `broadcast` | every receiver gets every message | pub/sub, shutdown signals |
| `watch` | many receivers, only the latest value | configuration, state |

<!-- check: skip -->
```rust
use tokio::sync::{mpsc, oneshot};

enum Command {
    Get { key: String, reply: oneshot::Sender<Option<u32>> },
    Set { key: String, value: u32 },
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<Command>(32);      // bounded: senders wait when full

    // An "actor": one task owns the state, everyone else sends it messages. No locks.
    let actor = tokio::spawn(async move {
        let mut store = std::collections::HashMap::new();
        while let Some(cmd) = rx.recv().await {           // None once every sender is dropped
            match cmd {
                Command::Set { key, value } => { store.insert(key, value); }
                Command::Get { key, reply } => { let _ = reply.send(store.get(&key).copied()); }
            }
        }
    });

    tx.send(Command::Set { key: "answer".into(), value: 42 }).await.unwrap();

    let (reply_tx, reply_rx) = oneshot::channel();
    tx.send(Command::Get { key: "answer".into(), reply: reply_tx }).await.unwrap();
    println!("{:?}", reply_rx.await.unwrap());            // Some(42)

    drop(tx);                                             // close the channel
    actor.await.unwrap();                                 // the actor exits cleanly
}
```

The **actor pattern** — a task that owns its state and processes messages one at a time — is often simpler and faster than `Arc<Mutex<..>>`.

### Broadcasting a shutdown signal

<!-- check: skip -->
```rust
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let mut workers = Vec::new();

    for id in 0..3 {
        let mut shutdown = shutdown_tx.subscribe();
        workers.push(tokio::spawn(async move {
            let mut ticks = 0;
            loop {
                tokio::select! {
                    _ = shutdown.recv() => break,                          // told to stop
                    _ = sleep(Duration::from_millis(10)) => ticks += 1,    // normal work
                }
            }
            println!("worker {id} stopped after some ticks: {}", ticks > 0);
        }));
    }

    sleep(Duration::from_millis(50)).await;
    shutdown_tx.send(()).unwrap();                       // every worker sees it
    for w in workers {
        w.await.unwrap();
    }
}
```

On a server you'd trigger this from `tokio::signal::ctrl_c().await`.

## Shared state

Prefer message passing, but when you do share state:

<!-- check: skip -->
```rust
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let counter = Arc::new(Mutex::new(0u32));
    let mut tasks = Vec::new();

    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        tasks.push(tokio::spawn(async move {
            let mut guard = counter.lock().await;     // .await, not .unwrap(): async-aware lock
            *guard += 1;
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
    println!("{}", *counter.lock().await);           // 10
}
```

Which mutex? Use **`std::sync::Mutex`** when the critical section is short and **never contains an `.await`** (it's faster, and the guard being `!Send` is a compile-time reminder). Use **`tokio::sync::Mutex`** only when you must hold the lock *across* an `.await`.

## Limiting concurrency with a semaphore

Spawning 10,000 tasks that each open a connection is a good way to hit the file-descriptor limit. A `Semaphore` hands out a fixed number of permits:

<!-- check: skip -->
```rust
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let limit = Arc::new(Semaphore::new(3));            // at most 3 at once
    let mut tasks = Vec::new();

    for i in 0..9 {
        let permit = Arc::clone(&limit).acquire_owned().await.unwrap();   // waits for a free slot
        tasks.push(tokio::spawn(async move {
            sleep(Duration::from_millis(20)).await;      // the "request"
            drop(permit);                                // release the slot
            i
        }));
    }
    let mut done = Vec::new();
    for t in tasks {
        done.push(t.await.unwrap());
    }
    println!("{done:?}");
}
```

## Streams

A `Stream` is the async version of `Iterator`: a sequence whose next item may need awaiting. The `futures` and `tokio-stream` crates provide combinators:

<!-- check: skip -->
```rust
use futures::stream::{self, StreamExt};
use std::time::Duration;
use tokio::time::sleep;

async fn fetch(n: u32) -> u32 {
    sleep(Duration::from_millis(20)).await;
    n * 2
}

#[tokio::main]
async fn main() {
    let results: Vec<u32> = stream::iter(1..=8)
        .map(fetch)                       // each item becomes a future
        .buffered(4)                      // run up to 4 at once, keep the original order
        .collect()
        .await;
    println!("{results:?}");             // [2, 4, 6, 8, 10, 12, 14, 16]

    let mut numbers = stream::iter(vec![1, 2, 3]);
    while let Some(n) = numbers.next().await {
        print!("{n} ");
    }
    println!();
}
```

`buffered(n)` / `buffer_unordered(n)` are the idiomatic way to process a list with bounded concurrency (also see `for_each_concurrent`).

## A TCP echo server

Async really shines with network I/O. This server handles any number of clients on a few threads:

<!-- check: skip -->
```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn handle(mut socket: TcpStream) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    loop {
        let n = socket.read(&mut buf).await?;       // yields while waiting for data
        if n == 0 {
            return Ok(());                          // client closed the connection
        }
        socket.write_all(&buf[..n]).await?;         // echo it back
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878").await?;
    println!("listening on 7878");

    loop {
        let (socket, addr) = listener.accept().await?;
        tokio::spawn(async move {                    // one lightweight task per connection
            if let Err(e) = handle(socket).await {
                eprintln!("{addr}: {e}");
            }
        });
    }
}
```

Try it with `nc 127.0.0.1 7878` (or `telnet`) from several terminals. The same structure underlies HTTP servers built with `axum`, `hyper` or `actix-web`.

## Testing async code

`#[tokio::test]` runs each test inside a runtime. With the `test-util` feature you can **pause time** so timers complete instantly:

<!-- check: skip -->
```rust
use std::time::Duration;

async fn retry_after_delay() -> u32 {
    tokio::time::sleep(Duration::from_secs(3600)).await;   // an hour!
    7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]            // virtual time: sleeps auto-advance
    async fn finishes_without_waiting_an_hour() {
        assert_eq!(retry_after_delay().await, 7);
    }
}

fn main() {}
```

(Enable with `cargo add tokio --features full,test-util` — `start_paused` needs `test-util`.)

## Common mistakes

- **Forgetting `move`** on spawned `async` blocks, so they borrow locals that don't live long enough.
- **Spawning unbounded numbers of tasks** — add a semaphore, `buffer_unordered`, or a bounded channel.
- **Using unbounded channels** (`unbounded_channel`) without a reason: a slow consumer means unbounded memory growth.
- **Not cancel-safe `select!` branches**, losing data when another branch wins.
- **Holding a `std::sync::Mutex` guard across `.await`** (`!Send` error or deadlock).
- **Doing CPU-heavy work in a task** instead of `spawn_blocking` / `rayon`.
- **Ignoring `JoinHandle` errors**: a panicking task's panic only surfaces when you await its handle.
- **Dropping the last `Sender`** and then wondering why the receiving loop ended (that's how channels close).

## Exercises

1. Spawn 5 tasks that each sleep a random-looking duration and return their id; print the ids in completion order using `JoinSet`.
2. Wrap a slow function in `timeout` and return a default value on timeout.
3. Build a `mpsc`-based logger task that prints messages sent from several producers.
4. Implement a counter actor with `Increment` and `Get` (with a `oneshot` reply) commands.
5. Use `buffer_unordered(3)` to "download" 10 fake URLs (sleeps) and print the total time; compare with `buffered(1)`.
6. Add graceful shutdown to the echo server using `tokio::signal::ctrl_c()` and `select!`.
7. Write a `#[tokio::test(start_paused = true)]` test for a function that retries three times with 1-second delays.
8. Make the echo server count total bytes served with an `AtomicUsize` shared through an `Arc`.

## Recap

- `tokio::spawn` runs independent tasks (`Send + 'static`); `JoinSet` manages groups; `join!`/`join_all` run futures concurrently within a task.
- `select!` races futures; `timeout` bounds one; dropping a future cancels it, so branches must be cancel-safe.
- `mpsc`, `oneshot`, `broadcast`, `watch` connect tasks; the actor pattern avoids locks.
- Bound concurrency with `Semaphore`, bounded channels or `buffer_unordered`.
- Never block the runtime; use `spawn_blocking` for synchronous or CPU-heavy work; test with `#[tokio::test]` and paused time.

Next: [Unsafe Rust →](08-unsafe-rust.md)
