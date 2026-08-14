# Overview

- `crud` and `repository` are separate generators. A shared feature may need a
  change in both.
- Exported macros expand in the crate that uses them. Paths such as
  `fractic_*`, `serde_json`, and `__repo_init!()` therefore need to resolve in
  that crate, not this one.
- Repository functions can set
  `class: read|write|destructive|internal`. A missing class means `internal`;
  the generator does not guess from the function name.
- `generate_<repository>_cli_interface!` uses the normal generated handlers and
  the caller's repository initializer. If that initializer later returns a
  network-backed repository, its callers do not need to change.
