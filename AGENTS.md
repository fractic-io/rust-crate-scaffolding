# fractic-crate-scaffolding

Procedural macros that derive a repository's boilerplate from one definition:
`repository_scaffolding!` (named operations with typed input / output,
execution style and `class`) and `crud_scaffolding!` (a DynamoDB object
graph). Each expands to the repository trait, input / output structs, API
Gateway handler macros, CRUD annotations and a machine-readable contract
module that the storyengine agent CLI dispatches over. Public, provided as-is
(`fractic-io/rust-crate-scaffolding`, identity `fractic`). Edition 2024.

## Read first

- `~/.agents/AGENTS.md` (workspace orientation; loaded automatically by Claude
  Code, read it if you are Codex or Grok) and `~/.agents/guides/rust.md`
  (§4 shows how consumers invoke these macros).
- `_/overview.md`: the non-obvious parts (façade crate, two macro pipelines,
  contract module, operation classes, expansion in the consuming crate).

## Layout

Workspace: `src/lib.rs` is the façade (re-exports the macros and
`contract`), `macros/` the proc-macro crate with two pipelines
(`repository/`, `crud/`), each `ast.rs` (parse) → `model.rs` (validate,
normalise) → `codegen/*.rs` (emit), `contract/` the runtime library for
descriptors and serialized dispatch, `tests/repository_contract.rs` the
integration test.

## Working here

- No flake here: `nix develop /efs/fractic/storyengine/util --command cargo
  test --workspace`. Verify codegen changes by building the consumers
  (`storyengine/api/v0`, `app/backend/api/v1`) through their `[patch]`.
- A feature exposed by both pipelines (handlers, contract dispatch,
  annotations) needs changes in both `macros/src/repository` and
  `macros/src/crud`; add it to the integration test.
- Generated code must resolve in the consuming crate (`fractic_*`,
  `serde_json`, `__repo_init!()`).
- Only add docs to `_/` when they capture something non-obvious that would
  otherwise cost a newcomer real time; keep `_/overview.md` current when the
  pipelines change.
- Commit / PR titles: `[repository] …`, `[crud] …`, `[contract] …`,
  imperative, trailing period.
