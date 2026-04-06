# Overview

Non-obvious things worth knowing before changing this crate:

- There are effectively two separate macro pipelines: `crud` and `repository`.
- The reliable edit path is `ast.rs` -> `model.rs` -> `codegen/*.rs`.
- Generated code depends on paths/macros not defined here, especially `fractic_*` crates and `__repo_init!()`.
- Test coverage is minimal, and currently only covers parts of CRUD parsing/modeling.
