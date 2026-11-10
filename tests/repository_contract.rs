use std::sync::Arc;

use fractic_crate_scaffolding::contract::{OperationClass, ValueShape};
use serde_json::json;

#[deny(deprecated)]
mod fixture {
    use fractic_crate_scaffolding::repository_scaffolding;

    repository_scaffolding!(
        TestRepository;

        function echo {
            class: read
            input: {
                value: String,
            }
            output: {
                value: String,
            }
        }
        function legacy_echo {
            deprecated: "use echo instead"
            input: {
                value: String,
            }
            output: {
                value: String,
            }
        }
    );

    pub struct TestRepositoryImpl;

    #[async_trait::async_trait]
    impl TestRepository for TestRepositoryImpl {
        async fn echo(&self, value: String) -> Result<String, fractic_server_error::ServerError> {
            Ok(value)
        }

        async fn legacy_echo(
            &self,
            value: String,
        ) -> Result<String, fractic_server_error::ServerError> {
            Ok(value)
        }
    }
}

// A repository with no functions is the resource layout's empty slot (a
// secondary access repository that has no operations yet).
mod empty_fixture {
    use fractic_crate_scaffolding::repository_scaffolding;

    repository_scaffolding!(EmptyRepository;);

    pub struct EmptyRepositoryImpl;

    impl EmptyRepository for EmptyRepositoryImpl {}
}

use empty_fixture::{EmptyRepositoryImpl, empty_repository_contract};
use fixture::{TestRepositoryImpl, test_repository_contract};

#[tokio::test]
async fn generated_contract_describes_and_dispatches_the_repository() {
    assert_eq!(test_repository_contract::DESCRIPTOR.name, "test");
    assert_eq!(
        test_repository_contract::DESCRIPTOR.operations[0].class,
        OperationClass::Read
    );
    assert_eq!(
        test_repository_contract::DESCRIPTOR.operations[0]
            .input
            .shape,
        ValueShape::Object
    );

    let output = test_repository_contract::dispatch(
        Arc::new(TestRepositoryImpl),
        "echo",
        json!({ "value": "hello" }),
    )
    .await
    .unwrap();

    assert_eq!(output, json!({ "value": "hello" }));
}

#[tokio::test]
async fn generated_contract_rejects_unknown_operations() {
    let error = test_repository_contract::dispatch(
        Arc::new(TestRepositoryImpl),
        "missing",
        serde_json::Value::Null,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("has no operation `missing`"));
}

#[tokio::test]
async fn generated_contract_of_an_empty_repository_rejects_every_operation() {
    assert_eq!(empty_repository_contract::DESCRIPTOR.name, "empty");
    assert!(empty_repository_contract::DESCRIPTOR.operations.is_empty());

    let error = empty_repository_contract::dispatch(
        Arc::new(EmptyRepositoryImpl),
        "missing",
        serde_json::Value::Null,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("has no operation `missing`"));
}

#[tokio::test]
async fn generated_contract_dispatches_deprecated_operations() {
    assert!(test_repository_contract::DESCRIPTOR.operations[1].deprecated);

    let output = test_repository_contract::dispatch(
        Arc::new(TestRepositoryImpl),
        "legacy-echo",
        json!({ "value": "hello" }),
    )
    .await
    .unwrap();

    assert_eq!(output, json!({ "value": "hello" }));
}
