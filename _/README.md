# Crate orientation

`fractic-crate-scaffolding` is a `proc-macro` crate with two mostly separate codegen pipelines:

- `crud_scaffolding`: object-graph CRUD scaffolding for Dynamo-backed types
- `repository_scaffolding`: repository-function scaffolding for named handlers

Both macros follow the same top-level flow in `src/lib.rs`:

1. parse DSL into an AST
2. validate/normalize into a semantic model
3. generate Rust tokens

## Start here

- `src/lib.rs` - macro entry points and the shared parse -> model -> generate flow
- `src/crud/*` - CRUD DSL/parser/model/codegen
- `src/repository/*` - repository DSL/parser/model/codegen
- `src/helpers.rs` - local snake_case / PascalCase helpers used in generated names

## What matters most

- The `crud` and `repository` modules are the real split in the crate. Shared logic is minimal.
- `ast.rs` files define the DSL shape and user-facing parse errors.
- `model.rs` files enforce semantic rules before codegen.
- `codegen/` files define the generated API surface and naming conventions.

## External assumptions

Generated code refers to types/macros that are not defined in this crate, especially:

- `::fractic_aws_dynamo::*`
- `::fractic_aws_apigateway::*`
- `::fractic_server_error::ServerError`
- `__repo_init!()` in generated handlers

Tests are thin and currently only cover parts of CRUD parsing/modeling.
