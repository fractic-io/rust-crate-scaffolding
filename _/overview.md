# Overview

This crate provides a structured way to describe a repository so that its
boilerplate can be derived from one source of truth. A definition becomes a
repository trait plus adapters such as `fractic-aws-apigateway` handlers and a
machine-readable repository protocol. The point is that every way of calling a
repository shares the same operations, types, and behavior instead of
reimplementing them by hand.

There are two kinds of definition:

- `repository_scaffolding!` describes arbitrary operations by name, input,
  output, execution style, and operation class.
- `crud_scaffolding!` describes a DynamoDB object graph; its object kinds and
  relationships imply the repository methods and CRUD operations.

The non-obvious parts:

- These are separate pipelines under `src/repository` and `src/crud`. Features
  exposed by both, such as handlers or CLI support, usually need changes in
  both.
- Each repository directly exposes a generated `<repository>_protocol` module
  containing a static descriptor and serialized dispatch function. Dispatch
  accepts an `Arc<dyn Repository>`, so callers can provide a local, test, or
  network-backed implementation without another generated macro invocation.
- Protocol descriptors and codecs live in the normal
  `fractic-repository-protocol` library crate. Generated dispatch returns the
  same `ServerError` used by repository traits.
- An arbitrary operation's `class` (`read`, `write`, `destructive`, or
  `internal`) is explicit policy metadata. Missing `class` means `internal`; it
  is not guessed from the operation name.
- API Gateway handler macros still expand in the consuming crate. Paths such as
  `fractic_*`, `serde_json`, and helper macros such as `__repo_init!()` must
  therefore resolve there.
- Internally, both pipelines follow `ast.rs` (parse) -> `model.rs` (validate and
  normalize) -> `codegen/*.rs` (emit Rust).
