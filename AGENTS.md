# Agent guide

Start with `_/README.md`.

If you are changing macro behavior, the fast path is:
- grammar/errors: `src/crud/ast.rs` or `src/repository/ast.rs`
- semantic validation/model shaping: `src/crud/model.rs` or `src/repository/model.rs`
- emitted Rust: `src/*/codegen/*.rs`

Keep docs in `_` minimal. Add or update notes there only when they materially improve contributor orientation.
