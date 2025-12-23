use anyhow::Result;
use zero2prod::spawn_test_app;

use crate::common::TRACING;

mod common;

#[tokio::test]
async fn health_check_works() -> Result<()> {
    std::sync::LazyLock::force(&TRACING);
    // Arrange
    let app = spawn_test_app().await?;
    // We need to bring in `reqwest`
    // to perform HTTP requests against our application.
    let client = reqwest::Client::new();
    // Act
    let response = client
        .get(format!("{}/health_check", &app.address))
        .send()
        .await?;
    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
    Ok(())
}
