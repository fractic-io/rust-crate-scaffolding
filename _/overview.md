# Overview

Non-obvious things worth knowing before changing this crate:

- There are effectively two separate macro pipelines: `crud` and `repository`.
- The reliable edit path is `ast.rs` -> `model.rs` -> `codegen/*.rs`.
- Generated code depends on paths/macros not defined here, especially `fractic_*` crates and `__repo_init!()`.
- Repository functions may declare `access: read|write|destructive|internal`.
  Unclassified functions default to `internal`. Generated operation catalogs
  carry this policy into transport metadata; they never infer access from an
  operation name.
- Both repository and CRUD scaffolding emit two independent integration macros:
  `generate_<repository>_operation_catalog!` creates transport-neutral
  descriptors, while `generate_<repository>_local_operation_handler!` adapts an
  injected repository to raw JSON calls. Keeping these separate lets a remote
  client compile the catalog without compiling local repository dispatch into
  its command layer.
- Local CRUD dispatch expands the same repository-injected handler cores used by
  the normal server handlers. The normal handler macro adds only repository
  initialization wrappers; there is no second CRUD implementation for the CLI.
- Consumers supply their runtime contract module. This keeps Clap, HTTP, and
  other transport dependencies out of this proc-macro crate and the generated
  API crate.
- Test coverage is minimal, and currently only covers parts of CRUD parsing/modeling.
