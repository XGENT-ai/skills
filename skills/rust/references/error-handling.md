# Errors Handling

Rust enforces a strict error handling approach, but *how* you handle them defines where your code feels ergonomic, consistent and safe - as opposing cryptic and painful. This chapter dives into best practices for modeling and managing fallible operations across libraries and binaries.

> Even if you decide to crash you application with `unwrap` or `expect`, Rust forces you to declare that intentionally.

## 4.1 Prefer `Result`, avoid panic 🫨

Rust has a powerful type that wraps fallible data, [`Result<T, E>`](https://doc.rust-lang.org/std/result/), this allows us to handle Error cases according to our needs and manage the state of the application based on that.

* If your function can fail, prefer to return a `Result`:
```rust
fn divide(x: f64, y: f64) -> Result<f64, DivisionError> {
    if y == 0.0 {
        Err(DivisionError::DividedByZero)
    } else {
        Ok(x / y)
    }
}
```

* Use `panic!` only in unrecoverable conditions - typically tests, assertions, bugs or a need to crash the application for some explicit reason.
* There are 3 relevant macros that can replace `panic!` in appropriate conditions:
    * `todo!`, similar to panic, but alerts the compiler that you are aware that there is code missing.
    * `unreachable!`, you have reasoned about the code block and are sure that condition `xyz` is not possible and if ever becomes possible you want to be alerted.
    * `unimplemented!`, specially useful for alerting that a block is not yet implement with a reason.

## 4.2 Avoid `unwrap`/`expect` in Production

This skill reserves `unwrap()` and `expect()` for tests and test helpers. Outside tests,
propagate failures with `?`, pattern-match explicitly or encode the invariant in a type.
An assumed-impossible failure is not an exception to this rule. Do not replace an unwrap
with a panic on ordinary invalid input, I/O failures or lock poisoning.

### 🚨 Alternative ways of handling `unwrap`/`expect`:

* If your `Result` (or `Option`) can have a predefined early return value in case of `Result::Err`, that doesn't need to know the `Err` value, use `let Ok(..) = else { return ... }` pattern, as it helps with flatten functions:
```rust
let Ok(json) = serde_json::from_str(&input) else {
    return Err(MyError::InvalidJson);
}; // `let ... else` is a statement: the `;` is required
```
* If your `Result` (or `Option`) needs error recovery in case of `Result::Err`, and the recovery path produces a value, use plain `if let ... else`:
```rust
// ❗ Note the shape: `if let PATTERN = EXPR { .. } else { .. }` - there is no
// `else` block attached to the pattern itself, that form only exists in `let ... else`.
if let Ok(json) = serde_json::from_str(&input) {
    handle(json)
} else {
    recover_from(&input)
}
```
* Keep `Option` when absence is normal. Convert `None` to `Result` with `ok_or_else` when the caller needs a reason the operation failed.
* Lastly `unwrap_or`, `unwrap_or_else` or `unwrap_or_default`, these functions help you create alternative exits to unwrap that manage the uninitialized values.

## 4.3 `thiserror` for Crate level errors

Deriving Error manually is verbose and error prone, the rust ecosystem has a really good crate to help with this, `thiserror`. It allows you to create error types that easily implement `From` trait as well as easy error message (`Display`), improving developer experience while working seamlessly with `?` and integrating with `std::error::Error`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("Network Timeout")]
    Timeout,
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid request information. Header: {headers}, Metadata: {metadata}")]
    InvalidRequest {
        headers: Headers,
        metadata: Metadata
    }
}
```

### Error Hierarchies and Wrapping

Public errors implement `std::error::Error` (`Debug` + `Display`) and preserve wrapped
causes through `source()`; `thiserror` can generate this alongside `From` conversions.
Expose categories callers can act on, not a single enum of every internal failure.
A situation-specific struct with private details and public accessors is also valid.
Keep messages concise (normally lowercase without trailing punctuation), and add
`Send + Sync + 'static` when required by the API or task boundary. Capture backtraces
when useful, without forcing the full cause chain/backtrace into `Display`. Use
`map_err` when a conversion needs context that a meaningful `From` cannot supply.

If a fallible operation consumes input that callers may need to retry, consider returning
that input in the error. Avoid making callers clone large inputs merely for recovery.

For layered systems the best practice is to use nested `enum/struct` errors with `#[from]`:

```rust
use crate::database::DbError;
use crate::external_services::ExternalHttpError;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Database handler error: {0}")]
    Db(#[from] DbError),
    #[error("External services error: {0}")]
    ExternalServices(#[from] ExternalHttpError)
}
```

## 4.4 Reserve `anyhow` for Binaries

`anyhow` is an amazing crate, and quite useful for projects that are beginning and need accelerated speed. However, there is a turning point where it just painfully propagates through your code, considering this, `anyhow` is recommended only for **binaries**, where ergonomic error handling is needed and there is no need for precise error types:

```rust
use anyhow::{Context, Result, anyhow};

fn main() -> Result<()> {
    let content = std::fs::read_to_string("config.json")
        .context("Failed to read config file")?;
    Config::from_str(&content)
        .map_err(|err| anyhow!("Config parsing error: {err}"))
}
```

### 🚨 `Anyhow` Gotchas

* Keeping the `context` and `anyhow` strings up-to-date in all code base is harder than keeping `thiserror` messages as you don't have a single point of entry.
* `anyhow::Result` erases context that a caller might need, so avoid using it in a library.
* test helper functions can use `anyhow` with little to no issues.

## 4.5 Use `?` to Bubble Errors

Prefer using `?` over verbose alternatives like `match` chains:
```rust
fn handle_request(req: &Request) -> Result<ValidatedRequest, MyError> {
    validate_headers(req)?;
    validate_body_format(req)?;
    validate_credentials(req)?;
    let body = Body::try_from(req)?;

    // Return the `Result` directly - `Ok(expr?)` would trip `clippy::needless_question_mark`
    ValidatedRequest::try_from((req, body))
}
```

> In case error recovery is needed, use `or_else`, `map_err`, `if let Ok(..) else`. To **inspect or log your error**, use `inspect_err`.

## 4.6 Unit Test should exercise errors

While many errors don't implement PartialEq and Eq, making it hard to do direct assertions between them, it is possible to check the error messages with `format!` or `to_string()`, making the errors meaningful and test validated:

```rust
#[test]
fn error_does_not_implement_partial_eq() {
    let err = divide(10., 0.0).unwrap_err();
    assert_eq!(err.to_string(), "division by zero");
}

#[test]
fn error_implements_partial_eq() {
    let err = process(my_value).unwrap_err();

    assert_eq!(
        err,
        MyError {
            ..
        }
    )
}
```

## 4.7 Important Topics

### Custom Error Structs

Sometimes you don't need an enum to handle your errors, as there is only one type of error that your module can have. This can be solved with `struct Errors`:

```rust
#[derive(Debug, thiserror::Error, PartialEq)]
#[error("Request failed with code `{code}`: {message}")]
struct HttpError {
    code: u16,
    message: String
}
```

### Async Errors

When using async runtimes, like Tokio, make sure that your errors implement `Send + Sync + 'static` where needed, specially in tasks or across `.await` boundaries:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ...
    Ok(())
}
```

> Avoid `Box<dyn std::error::Error>` in libraries unless it is really needed

### Cleanup and panic boundaries

Bind RAII guards to a local name (`let _guard = ...`); `let _ = ...` drops the value
immediately. Offer an explicit `close(self) -> Result<...>` or `finish` for fallible
cleanup, and keep `Drop` best-effort and non-panicking. Move slow or asynchronous
teardown into an explicit operation instead of starting detached work from `Drop`.
Fields already owning resources usually need no redundant custom `Drop`. Cleanup runs
on ordinary scope exit and unwinding, but cannot guarantee essential work after abort,
process termination or deliberate forgetting.

Do not use `catch_unwind` to handle ordinary errors; aborting panics cannot be caught.
For FFI-specific boundaries see [Unsafe and FFI](unsafe-and-ffi.md). Document errors,
reachable panics and unsafe preconditions in the public API.
