# Overview

This crate provides a structured way to describe a repository so that its
boilerplate can be derived from one source of truth. A definition becomes a
repository trait plus adapters such as `fractic-aws-apigateway` handlers and a
machine-readable CLI interface. The point is that every way of calling a
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
- The entry macros generate the repository contract, then emit
  `generate_<repository>_*` macros for adapters that need choices from the
  consuming crate, especially the repository initializer and runtime context.
- The CLI interface is metadata and JSON dispatch over the normal generated
  handlers, not a second implementation of repository behavior. It uses the
  same caller-supplied initializer, which may produce a local, test, or
  network-backed repository.
- An arbitrary operation's `class` (`read`, `write`, `destructive`, or
  `internal`) is explicit policy metadata. Missing `class` means `internal`; it
  is not guessed from the operation name.
- Generated macros expand in the consuming crate. Paths such as `fractic_*`,
  `serde_json`, and helper macros such as `__repo_init!()` must therefore
  resolve there.
- Internally, both pipelines follow `ast.rs` (parse) -> `model.rs` (validate and
  normalize) -> `codegen/*.rs` (emit Rust).
