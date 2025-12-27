use anyhow::{Ok, Result};
use zero2prod::spawn_test_app;

use crate::helpers::post_subscriptions;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = post_subscriptions(&app, body.to_string()).await?;

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
    // Arrange
    let app = spawn_test_app().await?;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = post_subscriptions(&app, invalid_body.to_string()).await?;

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

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = post_subscriptions(&app, body.to_string()).await?;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 BADREQUEST when the payload was {}.",
            description
        );
    }

    Ok(())
}
