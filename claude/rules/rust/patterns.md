---
paths:
  - "**/*.rs"
---
# Rust Patterns

## Repository + service layer

Encapsulate data access behind a `Send + Sync` trait; concrete impls handle storage (Postgres/SQLite/in-memory-for-tests). Business logic lives in a service struct that takes `Box<dyn Trait>` dependencies via constructor.

```rust
pub trait OrderRepository: Send + Sync {
    fn find_by_id(&self, id: u64) -> Result<Option<Order>, StorageError>;
    fn save(&self, order: &Order) -> Result<Order, StorageError>;
}
```

## Newtype for type safety

Wrap primitives (`struct UserId(u64)`, `struct OrderId(u64)`) so call sites can't swap argument order by accident.

## Enum state machines

Model states as enums with data — illegal states unrepresentable. Match exhaustively; no wildcard `_` on business-critical enums.

```rust
enum ConnectionState {
    Disconnected,
    Connecting { attempt: u32 },
    Connected { session_id: String },
    Failed { reason: String, retries: u32 },
}
```

## Builder

For structs with many optional fields: `T::builder(required...)` returning a builder with chainable setters and a `.build()`.

## Sealed traits

Private `mod private { pub trait Sealed {} }`, require `Format: private::Sealed` to block external implementations of a public trait.

## API response envelope

```rust
#[derive(serde::Serialize)]
#[serde(tag = "status")]
pub enum ApiResponse<T: serde::Serialize> {
    #[serde(rename = "ok")] Ok { data: T },
    #[serde(rename = "error")] Error { message: String },
}
```
