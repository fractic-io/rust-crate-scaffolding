# Overview

Non-obvious things worth knowing before changing this crate:

- There are effectively two separate macro pipelines: `crud` and `repository`.
- The reliable edit path is `ast.rs` -> `model.rs` -> `codegen/*.rs`.
- Generated code depends on paths/macros not defined here, especially `fractic_*` crates and `__repo_init!()`.
- Repository functions may declare `access: read|write|destructive|internal`.
  Unclassified functions default to `internal`. The generated `*_cli_endpoint`
  macro carries this policy into transport metadata; it never infers access
  from an operation name.
- Both repository and CRUD scaffolding emit `generate_<repository>_cli_endpoint!`.
  The consumer supplies a module name, public endpoint name, and a runtime
  module implementing the generated endpoint contract. This keeps Clap and
  transport dependencies out of the proc-macro crate and API crate.
- Test coverage is minimal, and currently only covers parts of CRUD parsing/modeling.
