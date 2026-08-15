use std::sync::Arc;

use fractic_repository_protocol::{OperationClass, ValueShape};
use serde_json::json;

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
    );

    pub struct TestRepositoryImpl;

    #[async_trait::async_trait]
    impl TestRepository for TestRepositoryImpl {
        async fn echo(&self, value: String) -> Result<String, fractic_server_error::ServerError> {
            Ok(value)
        }
    }
}

use fixture::{TestRepositoryImpl, test_repository_protocol};

#[tokio::test]
async fn generated_protocol_describes_and_dispatches_the_repository() {
    assert_eq!(test_repository_protocol::DESCRIPTOR.name, "test");
    assert_eq!(
        test_repository_protocol::DESCRIPTOR.operations[0].class,
        OperationClass::Read
    );
    assert_eq!(
        test_repository_protocol::DESCRIPTOR.operations[0]
            .input
            .shape,
        ValueShape::Object
    );

    let output = test_repository_protocol::dispatch(
        Arc::new(TestRepositoryImpl),
        "echo",
        json!({ "value": "hello" }),
    )
    .await
    .unwrap();

    assert_eq!(output, json!({ "value": "hello" }));
}

#[tokio::test]
async fn generated_protocol_rejects_unknown_operations() {
    let error = test_repository_protocol::dispatch(
        Arc::new(TestRepositoryImpl),
        "missing",
        serde_json::Value::Null,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("has no operation `missing`"));
}
