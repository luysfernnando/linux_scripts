---
paths:
  - "**/*.rs"
---
# Rust Coding Style

> Extends [common/coding-style.md](../common/coding-style.md).

## Formatting

`cargo fmt` before commit; `cargo clippy -- -D warnings` (treat warnings as errors).

## Immutability & naming

`let` by default, `let mut` only when needed; return new values over mutating in place; `Cow<'_, T>` when a function may or may not allocate. `snake_case` (fns/vars/modules), `PascalCase` (types/traits), `SCREAMING_SNAKE_CASE` (consts), short lowercase lifetimes (`'a`, `'de`).

## Ownership & borrowing

Borrow (`&T`) by default, take ownership only to store/consume. Accept `&str`/`&[T]` over `String`/`Vec<T>` in params; `impl Into<String>` for owning constructors. Never clone just to dodge the borrow checker.

## Error handling

`Result<T, E>` + `?`; never `unwrap()` outside tests/unreachable states. Libraries: typed errors via `thiserror`. Applications: `anyhow` + `.with_context(...)`.

```rust
fn load_config(path: &str) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {path}"))?;
    toml::from_str(&content).with_context(|| format!("failed to parse {path}"))
}
```

## Iterators over loops

Iterator chains for transformations; loops only for complex control flow (early returns, side effects).

## Module organization & visibility

Organize `src/` by domain (`auth/`, `orders/`), not by type (`models/`, `controllers/`). Default private; `pub(crate)` for internal sharing; re-export public API from `lib.rs`.
