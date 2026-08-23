---
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
---
# Rust Hooks

Optional PostToolUse setup (`~/.claude/settings.json`) to run after editing `.rs` files: `cargo fmt` (format), `cargo clippy` (lint), `cargo check` (fast compile check). Not enforced unless configured.
