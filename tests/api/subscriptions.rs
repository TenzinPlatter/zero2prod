use anyhow::{Ok, Result};
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{header_exists, method, path},
};

use crate::helpers::{get_email_links, spawn_test_app};

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/emails"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.to_string()).await?;

    // Assert
    assert_eq!(200, response.status().as_u16());
    Ok(())
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() -> Result<()> {
    let app = spawn_test_app().await?;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    Mock::given(path("/emails"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.to_string()).await?;
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.pool)
        .await?;

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");

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
        let response = app.post_subscriptions(invalid_body.to_string()).await?;

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
        let response = app.post_subscriptions(body.to_string()).await?;

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

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(header_exists("Authorization"))
        .and(path("/emails"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.to_string()).await?;

    Ok(())
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(header_exists("Authorization"))
        .and(path("/emails"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.to_string()).await?;

    // Assert
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = get_email_links(email_request)?;

    assert_eq!(links.html, links.plain_text);

    Ok(())
}

#[tokio::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() -> Result<()> {
    let app = spawn_test_app().await?;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    sqlx::query!("ALTER TABLE subscription_tokens DROP COLUMN subscriber_id;",)
        .execute(&app.pool)
        .await?;

    let response = app.post_subscriptions(body.to_string()).await?;
    assert_eq!(500, response.status().as_u16());

    Ok(())
}
