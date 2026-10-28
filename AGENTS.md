# fractic-crate-scaffolding

Procedural macros that derive a repository's boilerplate from one
definition: `repository_scaffolding!` (named operations with typed input /
output, execution style and `class`) and `crud_scaffolding!` (a DynamoDB
object graph). Each expands to the repository trait, input / output structs,
API Gateway handler macros, CRUD annotations and a machine-readable contract
module that the storyengine agent CLI dispatches over. Consumer side:
`/efs/fractic/_/rust.md` §4.

Workspace: `src/lib.rs` is the façade (re-exports the macros and
`contract`), `macros/` the proc-macro crate with two pipelines
(`repository/`, `crud/`), each `ast.rs` (parse) → `model.rs` (validate,
normalise) → `codegen/*.rs` (emit), `contract/` the runtime library for
descriptors and serialized dispatch, `tests/repository_contract.rs` the
integration test.

- Test with `--workspace` (through a product devshell); verify codegen
  changes by building the consumers (`storyengine/api/v0`,
  `app/backend/api/v1`) through their `[patch]`.
- A feature exposed by both pipelines (handlers, contract dispatch,
  annotations) needs changes in both `macros/src/repository` and
  `macros/src/crud`, and a case in the integration test.
- Generated code must resolve in the consuming crate (`fractic_*`,
  `serde_json`, `__repo_init!()`).
- Commit / PR titles: `[repository] …`, `[crud] …`, `[contract] …`.

## Docs

- `_/overview.md`: the non-obvious parts (façade crate, two pipelines,
  contract module, operation classes). Keep it current when the pipelines
  change.
