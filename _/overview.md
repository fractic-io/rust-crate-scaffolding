# Overview

Non-obvious things worth knowing before changing this crate:

- There are effectively two separate macro pipelines: `crud` and `repository`.
- The reliable edit path is `ast.rs` -> `model.rs` -> `codegen/*.rs`.
- Generated code depends on paths/macros not defined here, especially `fractic_*` crates and `__repo_init!()`.
- Repository functions may declare `access: read|write|destructive|internal`.
  Unclassified functions default to `internal`. Generated operation catalogs
  carry this policy into transport metadata; they never infer access from an
  operation name.
- Both repository and CRUD scaffolding emit
  `generate_<repository>_cli_interface!`. It combines discovery metadata with a
  raw-JSON `call` function and accepts the same repository initializer as the
  normal generated handler macro.
- The CLI interface calls the normal generated typed handlers. It does not emit
  a second handler family or own a transport abstraction. A CLI can initialize
  its repository through a context link; changing that link from a local alias
  to a network-forwarding repository leaves the generated interface and its
  callers unchanged.
- Consumers supply their runtime contract module. This keeps Clap, HTTP, and
  other transport dependencies out of this proc-macro crate and the generated
  API crate.
- Test coverage is minimal, and currently only covers parts of CRUD parsing/modeling.
