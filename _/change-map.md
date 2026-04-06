# Change map

## If you are changing the CRUD DSL

- grammar and parse errors: `src/crud/ast.rs`
- semantic rules for object kinds/parents/children: `src/crud/model.rs`
- generated repository trait: `src/crud/codegen/repository.rs`
- generated impl macro: `src/crud/codegen/repository_impl.rs`
- generated annotations: `src/crud/codegen/annotations.rs`
- generated CRUD handlers: `src/crud/codegen/handlers.rs`

Useful context:

- CRUD object kinds are `root`, `ordered`, `unordered`, `batch`, `singleton`, and `singleton_family`.
- `src/crud/codegen/handlers.rs` is the largest/highest-coupling file; it maps API Gateway `CrudOperation` values to repository calls.
- Generated CRUD handlers build placeholder objects when only IDs are available, so typed repository calls can still be made.

## If you are changing the repository DSL

- grammar and parse errors: `src/repository/ast.rs`
- semantic model and inline-struct lifting: `src/repository/model.rs`
- generated repository trait: `src/repository/codegen/repository.rs`
- generated handlers: `src/repository/codegen/handlers.rs`

Useful context:

- Function kinds are `function`, `function_direct`, `blocking`, and `blocking_direct`.
- Inline `{ ... }` input/output structs are converted into helper structs in `src/repository/model.rs`.

## Small but easy-to-miss details

- Both public macros append `const _: () = ();` after generated tokens in `src/lib.rs`.
- `src/crud/codegen/annotations.rs` intentionally includes some root ordered/batch annotation support that may be ahead of available downstream manager types.
