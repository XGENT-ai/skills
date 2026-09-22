# Chapter 10 - Async Patterns with Tokio

> Not part of the original Apollo handbook. Added to cover async Rust, which the other chapters only touch in passing (see [Chapter 4](chapter_04.md) for error traits across `.await`, and [Chapter 9](chapter_09.md) for `Send`/`Sync`).

Async Rust is not a separate language, but it does change which idioms are correct. Ownership, borrowing and error handling rules from the previous chapters still apply - what is new is that your code can be *suspended* at every `.await`, and everything held across that point must survive the suspension.

## 10.1 The Execution Model

A `Future` is **lazy**: nothing runs until something polls it. That is why an un-awaited future is a bug, not a background task.

```
Future (lazy) → poll() → Ready(value) | Pending
                ↑           ↓
              Waker ← Runtime schedules
```

| Concept | Purpose |
|------------|------------------------------------------|
| `Future` | Lazy computation that may complete later |
| `async fn` | Function returning `impl Future` |
| `await` | Suspend until the future completes |
| `Task` | Spawned future running concurrently |
| `Runtime` | Executor that polls futures |

The practical consequences, each expanded later in this chapter:

* **Blocking a thread blocks every task on it.** Never call `std::thread::sleep`, blocking file I/O or CPU-bound loops inside `async fn` - see [10.9](#109-shared-state-and-resource-limits).
* **State held across `.await` must be `Send`** when the future is spawned on a multi-threaded runtime.
* **Cancellation is silent.** Dropping a future stops it wherever it was suspended, so cleanup belongs in `Drop`, not after the `.await` - see [10.6](#106-graceful-shutdown).

## 10.2 Project Setup

```toml
# Cargo.toml
[dependencies]
tokio = { version = "1", features = ["full"] }
futures = "0.3"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

> `features = ["full"]` is fine for binaries. For libraries, enable only what you use (`rt`, `macros`, `sync`, `time`, ...) so downstream crates do not pay for the rest.

```rust
use anyhow::Result;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let result = fetch_data("https://api.example.com").await?;
    println!("Got: {result}");

    Ok(())
}

async fn fetch_data(url: &str) -> Result<String> {
    sleep(Duration::from_millis(100)).await;
    Ok(format!("Data from {url}"))
}
```

## 10.3 Concurrent Task Execution

`JoinSet` owns its tasks and aborts them when dropped, which makes it the default choice over a `Vec<JoinHandle<_>>`:

```rust
use anyhow::Result;
use tokio::task::JoinSet;

async fn fetch_all_concurrent(urls: Vec<String>) -> Result<Vec<String>> {
    let mut set = JoinSet::new();

    for url in urls {
        set.spawn(async move { fetch_data(&url).await });
    }

    let mut results = Vec::new();
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(data)) => results.push(data),
            Ok(Err(e)) => tracing::error!("Task failed: {e}"),
            Err(e) => tracing::error!("Join error: {e}"), // panic or abort
        }
    }

    Ok(results)
}
```

> The nested `Result` is not noise: the outer one reports whether the *task* survived (panic/abort), the inner one whether the *work* succeeded. Collapsing them hides panics.

Unbounded spawning is a denial of service against yourself. When the input size is caller-controlled, bound the concurrency:

```rust
use futures::stream::{self, StreamExt};

async fn fetch_with_limit(urls: Vec<String>, limit: usize) -> Vec<Result<String>> {
    stream::iter(urls)
        .map(|url| async move { fetch_data(&url).await })
        .buffer_unordered(limit) // at most `limit` in flight
        .collect()
        .await
}
```

`select!` races futures and **drops the losers**:

```rust
use tokio::select;

async fn race_requests(url1: &str, url2: &str) -> Result<String> {
    select! {
        result = fetch_data(url1) => result,
        result = fetch_data(url2) => result,
    }
}
```

> ❗ Every branch of `select!` must be cancel-safe. A future dropped mid-`.await` never resumes, so a branch that has already consumed bytes from a socket loses them. When in doubt, move the work into a spawned task and select on its handle instead.

## 10.4 Channels for Communication

Prefer message passing over shared mutable state: a channel makes ownership transfer explicit and avoids lock contention entirely.

| Channel | Shape | Use for |
|---------------|-----------------------------|----------------------------------|
| `mpsc` | many producers, one consumer | work queues, actor inboxes |
| `broadcast` | many producers, many consumers | fan-out events (each gets a copy) |
| `oneshot` | one producer, one value | request/response replies |
| `watch` | one producer, many consumers | latest-value state (config, health) |

```rust
use tokio::sync::{broadcast, mpsc, oneshot, watch};

// mpsc: bounded, so a slow consumer applies backpressure to producers
async fn mpsc_example() {
    let (tx, mut rx) = mpsc::channel::<String>(100);

    let tx2 = tx.clone();
    tokio::spawn(async move {
        let _ = tx2.send("Hello".to_string()).await;
    });
    drop(tx); // the loop below ends only once every sender is gone

    while let Some(msg) = rx.recv().await {
        println!("Got: {msg}");
    }
}

// broadcast: every live receiver gets every message; slow ones get Lagged
async fn broadcast_example() {
    let (tx, _) = broadcast::channel::<String>(100);

    let mut rx1 = tx.subscribe();
    let mut rx2 = tx.subscribe();

    tx.send("Event".to_string()).unwrap();

    let _ = rx1.recv().await;
    let _ = rx2.recv().await;
}

// oneshot: a single value, consumed by awaiting the receiver
async fn oneshot_example() -> String {
    let (tx, rx) = oneshot::channel::<String>();

    tokio::spawn(async move {
        let _ = tx.send("Result".to_string());
    });

    rx.await.expect("sender dropped without sending")
}

// watch: receivers see only the latest value, never a backlog
async fn watch_example() {
    let (tx, mut rx) = watch::channel("initial".to_string());

    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            println!("New value: {}", *rx.borrow());
        }
    });

    tx.send("updated".to_string()).unwrap();
}
```

> `mpsc::unbounded_channel` removes backpressure along with the bound - memory grows until the process dies. Reach for it only when the producer is provably bounded.

## 10.5 Async Error Handling

The rules from [Chapter 4](chapter_04.md) hold: `thiserror` for libraries, `anyhow` for binaries, `?` over match chains. Async adds one requirement - errors that cross a spawn or an `.await` boundary generally need `Send + Sync + 'static`.

```rust
use anyhow::{Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),
}

// Binary/application code: anyhow plus context at each layer
async fn process_request(id: &str) -> Result<Response> {
    let data = fetch_data(id).await.context("Failed to fetch data")?;
    let parsed = parse_response(&data).context("Failed to parse response")?;
    Ok(parsed)
}

// Library code: a typed error the caller can match on
async fn get_user(db: &Db, id: &str) -> Result<User, ServiceError> {
    match db.query(id).await? {
        Some(user) => Ok(user),
        None => Err(ServiceError::NotFound(id.to_string())),
    }
}
```

Every outbound call needs a deadline. Without one, a hung peer holds a task forever:

```rust
use std::future::Future;
use std::time::Duration;
use tokio::time::timeout;

async fn with_timeout<T, F>(duration: Duration, future: F) -> Result<T, ServiceError>
where
    F: Future<Output = Result<T, ServiceError>>,
{
    timeout(duration, future)
        .await
        .map_err(|_| ServiceError::Timeout(duration))?
}
```

> `timeout` cancels by dropping the future. Anything that was half-done is abandoned, so for non-idempotent work (a payment, a write) record intent *before* the call rather than assuming a timeout means "did not happen".

## 10.6 Graceful Shutdown

A task that ignores shutdown gets killed mid-write when the runtime drops. Give every long-lived task a cancellation branch.

```rust
use tokio::signal;
use tokio_util::sync::CancellationToken;

async fn run_server() -> Result<()> {
    let token = CancellationToken::new();
    let worker_token = token.clone();

    let worker = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = worker_token.cancelled() => {
                    tracing::info!("Task shutting down");
                    break;
                }
                _ = do_work() => {}
            }
        }
    });

    signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received");

    token.cancel();

    // Bound the wait: await the task, but do not hang forever on a stuck one
    match tokio::time::timeout(Duration::from_secs(5), worker).await {
        Ok(_) => tracing::info!("Clean shutdown"),
        Err(_) => tracing::warn!("Worker did not stop in time"),
    }

    Ok(())
}
```

> Prefer joining the tasks over `sleep`-ing for a fixed grace period: a sleep is both too long on a fast shutdown and too short on a slow one. `CancellationToken` also has `child_token()`, which propagates cancellation down a task tree.

A `broadcast` channel works when you have no `tokio-util` dependency - every subscriber is woken by a single send:

```rust
use tokio::sync::broadcast;

async fn run_with_broadcast() -> Result<()> {
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    let mut rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        tokio::select! {
            _ = rx.recv() => tracing::info!("Received shutdown"),
            _ = async { loop { do_work().await } } => {}
        }
    });

    signal::ctrl_c().await?;
    let _ = shutdown_tx.send(());

    Ok(())
}
```

## 10.7 Async Traits

Since Rust 1.75, `async fn` works directly in traits. Use it - it allocates nothing and needs no dependency:

```rust
pub trait Repository {
    async fn get(&self, id: &str) -> Result<Entity>;
    async fn save(&self, entity: &Entity) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
}
```

The catch is that such a trait is **not dyn-compatible** and its returned futures carry no `Send` bound, so a generic caller that spawns will not compile. Pick by how the trait is consumed:

| Need | Use |
|------------------------------------------|----------------------------------------------|
| Static dispatch, `impl Repository` bounds | native `async fn` in trait |
| `Send` bound for spawning | `fn get(..) -> impl Future<Output = ..> + Send` |
| `dyn Repository` / heterogeneous storage | `#[async_trait]` (boxes each future) |

This follows the rule from [Chapter 6](chapter_06.md): static where you can, dynamic where you must. `#[async_trait]` is not deprecated - it is the tool for the `dyn` case, at the cost of one `Box` allocation per call.

```rust
use async_trait::async_trait;

#[async_trait]
pub trait Repository: Send + Sync {
    async fn get(&self, id: &str) -> Result<Entity>;
    async fn save(&self, entity: &Entity) -> Result<()>;
}

pub struct PostgresRepository {
    pool: sqlx::PgPool,
}

#[async_trait]
impl Repository for PostgresRepository {
    async fn get(&self, id: &str) -> Result<Entity> {
        sqlx::query_as!(Entity, "SELECT * FROM entities WHERE id = $1", id)
            .fetch_one(&self.pool)
            .await
            .map_err(Into::into)
    }

    async fn save(&self, entity: &Entity) -> Result<()> {
        sqlx::query!(
            "INSERT INTO entities (id, data) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET data = $2",
            entity.id,
            entity.data
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

// Only possible because the trait is boxed by #[async_trait]
async fn process(repo: &dyn Repository, id: &str) -> Result<()> {
    let entity = repo.get(id).await?;
    repo.save(&entity).await
}
```

## 10.8 Streams and Async Iteration

A `Stream` is the async counterpart of `Iterator`: many values over time, each awaited. Use one when results arrive incrementally and you do not want to buffer them all.

```rust
use async_stream::stream;
use futures::stream::{self, Stream, StreamExt};

fn numbers_stream() -> impl Stream<Item = i32> {
    stream! {
        for i in 0..10 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            yield i;
        }
    }
}

async fn process_stream() {
    let processed: Vec<_> = numbers_stream()
        .filter(|n| futures::future::ready(*n % 2 == 0))
        .map(|n| n * 2)
        .collect()
        .await;

    println!("{processed:?}");
}

// Amortize per-item cost (one DB round trip per chunk instead of per item)
async fn process_in_chunks() {
    let mut chunks = numbers_stream().chunks(3);

    while let Some(chunk) = chunks.next().await {
        println!("Processing chunk: {chunk:?}");
    }
}

// Interleave two sources, yielding whichever is ready first
async fn merge_streams() {
    stream::select(numbers_stream(), numbers_stream())
        .for_each(|n| async move { println!("Got: {n}") })
        .await;
}
```

> The same advice as [Chapter 1](chapter_01.md) applies: avoid an intermediate `.collect()` when the next stage can consume the stream directly.

## 10.9 Shared State and Resource Limits

Use `tokio::sync::Mutex`/`RwLock` **only** when the guard is held across an `.await`. Otherwise `std::sync` is faster and lets you lock from non-async code such as `Drop`.

```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

struct Cache {
    data: RwLock<HashMap<String, String>>,
}

impl Cache {
    async fn get(&self, key: &str) -> Option<String> {
        self.data.read().await.get(key).cloned()
    }

    async fn set(&self, key: String, value: String) {
        self.data.write().await.insert(key, value);
    }
}
```

> ❗ Never hold a lock across an `.await` that can block on the lock holder's own progress - that is the classic async deadlock. Narrow the critical section with a block, or `clone()` the value out and drop the guard before awaiting.

A `Semaphore` caps how much of a scarce resource is in flight at once. Note that the `Drop` below is **synchronous** - it cannot `.await`, so the free list uses `std::sync::Mutex`:

```rust
use std::sync::Mutex;
use tokio::sync::Semaphore;

struct Pool {
    semaphore: Semaphore,
    connections: Mutex<Vec<Connection>>, // std, not tokio: Drop cannot await
}

impl Pool {
    fn new(size: usize) -> Self {
        Self {
            semaphore: Semaphore::new(size),
            connections: Mutex::new((0..size).map(|_| Connection::new()).collect()),
        }
    }

    async fn acquire(&self) -> PooledConnection<'_> {
        // The permit guarantees a connection is available
        let permit = self.semaphore.acquire().await.expect("semaphore never closed");
        let conn = self.connections.lock().unwrap().pop().expect("permit implies a free conn");
        PooledConnection { pool: self, conn: Some(conn), _permit: permit }
    }
}

struct PooledConnection<'a> {
    pool: &'a Pool,
    conn: Option<Connection>,
    _permit: tokio::sync::SemaphorePermit<'a>, // released on drop, after the push below
}

impl Drop for PooledConnection<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool.connections.lock().unwrap().push(conn);
        }
    }
}
```

> Spawning from `Drop` to run async cleanup does not work here: `tokio::spawn` requires `'static`, and `self.pool` is a borrow. Either make the cleanup synchronous as above, or hold an `Arc<Pool>` instead of a reference.

CPU-bound or blocking work must leave the async threads:

```rust
// Blocking I/O or long CPU work: move it off the runtime threads
let result = tokio::task::spawn_blocking(move || expensive_sync_work(input)).await?;
```

## 10.10 Debugging and Instrumentation

Stack traces are near-useless across `.await` points; spans are how you regain the causal chain.

```rust
use tracing::{instrument, Instrument};

#[instrument(skip(pool))] // skip args that are large or secret
async fn fetch_user(pool: &PgPool, id: &str) -> Result<User> {
    tracing::debug!("Fetching user");
    // ...
}

// Attach a span to a spawned task - it is entered on every poll
let span = tracing::info_span!("worker", id = %worker_id);
tokio::spawn(
    async move {
        // ...
    }
    .instrument(span),
);
```

For stuck tasks and starved workers, `tokio-console` shows every task's state and poll times live:

```toml
tokio = { version = "1", features = ["full", "tracing"] }
console-subscriber = "0.4"
```

```sh
RUSTFLAGS="--cfg tokio_unstable" cargo run
tokio-console
```

## 10.11 Checklist

### ✅ Do

* **Bound concurrency** - `buffer_unordered`, `Semaphore` or a worker pool, never an unbounded `spawn` loop.
* **Use `JoinSet`** to own and abort groups of tasks instead of loose `JoinHandle`s.
* **Prefer channels over shared state**; reach for a lock only when a channel does not fit.
* **Put a timeout on every outbound call**, then decide what a timeout means for that operation.
* **Handle cancellation explicitly** with `CancellationToken` or a shutdown channel.
* **Instrument with `tracing`** - `#[instrument]` on async fns, `.instrument(span)` on spawned tasks.

### ❌ Don't

* **Don't block the runtime** - no `std::thread::sleep`, blocking I/O or heavy CPU work in `async fn`; use `spawn_blocking`.
* **Don't hold a lock across `.await`** - narrow the critical section or clone the value out first.
* **Don't use `tokio::sync::Mutex` by default** - `std::sync::Mutex` is cheaper unless the guard crosses an `.await`.
* **Don't ignore `JoinError`** - it is how a panicked task reports itself.
* **Don't forget `Send`** on futures you spawn, or on errors that cross task boundaries ([Chapter 9](chapter_09.md)).
* **Don't create a future you never await** - it does nothing at all.
