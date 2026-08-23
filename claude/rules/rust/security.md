---
paths:
  - "**/*.rs"
---
# Rust Security

## Secrets

Never hardcode; `std::env::var(...)`, fail fast if missing, `.env` in `.gitignore`.

## SQL injection

Always parameterized queries via sqlx/diesel/sea-orm bind params — never format user input into SQL strings.

## Input validation

Parse, don't validate: convert unstructured input to typed structs at the boundary (newtype pattern), reject with clear errors.

```rust
pub struct Email(String);
impl Email {
    pub fn parse(input: &str) -> Result<Self, ValidationError> {
        let trimmed = input.trim();
        let at = trimmed.find('@').filter(|&p| p > 0 && p < trimmed.len() - 1)
            .ok_or_else(|| ValidationError::InvalidEmail(input.to_string()))?;
        if trimmed.len() > 254 || !trimmed[at + 1..].contains('.') {
            return Err(ValidationError::InvalidEmail(input.to_string()));
        }
        Ok(Self(trimmed.to_string())) // prefer a validated `email_address` crate in production
    }
}
```

## Unsafe code

Minimize; every `unsafe` block needs a `// SAFETY:` comment stating the invariant; never used to dodge the borrow checker; audit during review.

## Dependency security

`cargo audit` (CVEs), `cargo deny check` (license/advisory), `cargo tree -d` (duplicates). Minimize dependency count; keep updated (Dependabot/Renovate).

## Error messages

Never leak internal paths/stack traces/DB errors to clients — log detail server-side (`tracing`), return generic messages.
