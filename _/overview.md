# Overview

The non-obvious parts of the crate:

- The public `fractic-crate-scaffolding` crate is a façade: it re-exports
  the procedural macros and the contract API so consumers need only this one
  dependency. The implementations live under `macros` and `contract`.
- The two macros are separate pipelines under `macros/src/repository` and
  `macros/src/crud`, each `ast.rs` (parse) → `model.rs` (validate and
  normalise) → `codegen/*.rs` (emit). A feature exposed by both, such as
  handlers or contract dispatch, usually needs changes in both.
- Each repository exposes a generated `<repository>_contract` module with a
  static descriptor and a serialized dispatch function. Dispatch takes an
  `Arc<dyn Repository>`, so a caller can supply a local, test or
  network-backed implementation without another macro invocation.
- Descriptors and codecs are implemented by the internal
  `fractic-repository-contract` library, exposed as
  `fractic_crate_scaffolding::contract`. Generated dispatch returns the same
  `ServerError` as the repository traits.
- An operation's `class` (`read`, `write`, `destructive`, `internal`) is
  explicit policy metadata. A missing `class` means `internal`; it is never
  guessed from the name.
- API Gateway handler macros expand in the consuming crate, so paths such as
  `fractic_*`, `serde_json` and helper macros such as `__repo_init!()` must
  resolve there.
