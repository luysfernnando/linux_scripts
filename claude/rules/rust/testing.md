---
paths:
  - "**/*.rs"
---
# Rust Testing

Tooling: `#[test]`/`#[cfg(test)]` for units, `rstest` (parameterized), `proptest` (property-based), `mockall` (trait mocking), `#[tokio::test]` (async).

Unit tests live in `#[cfg(test)] mod tests` beside the code; integration tests in `tests/` (each file its own binary); benches in `benches/`.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_email() {
        let result = User::new("Bob", "not-an-email");
        assert!(result.is_err());
    }

    #[rstest]
    #[case("hello", 5)]
    #[case("", 0)]
    fn test_string_length(#[case] input: &str, #[case] expected: usize) {
        assert_eq!(input.len(), expected);
    }
}
```

Mock traits (not concrete types) with `mockall::mock!`. Name tests by scenario: `rejects_order_when_insufficient_stock`.

Coverage floor 80% via `cargo-llvm-cov` (`cargo llvm-cov --fail-under-lines 80`), excluding generated code/FFI bindings.
