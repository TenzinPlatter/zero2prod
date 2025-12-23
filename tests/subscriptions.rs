use anyhow::Result;
use zero2prod::spawn_test_app;

use crate::common::TRACING;

mod common;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() -> Result<()> {
    std::sync::LazyLock::force(&TRACING);
    // Arrange
    let app = spawn_test_app().await?;

    // Act
    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?;

    // Assert
    assert_eq!(200, response.status().as_u16());
    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.pool)
        .await?;

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    Ok(())
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() -> Result<()> {
    std::sync::LazyLock::force(&TRACING);
    // Arrange
    let app = spawn_test_app().await?;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await?;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
    Ok(())
}
