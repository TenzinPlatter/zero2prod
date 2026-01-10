use anyhow::Result;
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{method, path},
};

use crate::helpers::{get_email_links, spawn_test_app};

#[tokio::test]
async fn confirmations_without_token_are_rejected() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;

    // Act
    let response = reqwest::Client::new()
        .get(format!(
            "{}/subscriptions/confirm",
            app.config.app_address()
        ))
        .send()
        .await?;

    // Assert
    assert_eq!(400, response.status().as_u16());
    Ok(())
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() -> Result<()> {
    let app = spawn_test_app().await?;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/emails"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.to_string()).await?;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = get_email_links(email_request)?;
    let link = &links.html;

    assert_eq!(link.host_str().unwrap(), "127.0.0.1");

    let response = reqwest::Client::new().get(link.as_str()).send().await?;

    assert_eq!(200, response.status().as_u16());

    Ok(())
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() -> Result<()> {
    let app = spawn_test_app().await?;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/emails"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.to_string()).await?;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let links = get_email_links(email_request)?;
    let link = &links.html;
    let _ = reqwest::Client::new()
        .get(link.as_str())
        .send()
        .await?
        .error_for_status()?;

    let _ = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.pool)
        .await?;

    Ok(())
}
