use anyhow::Result;
use uuid::Uuid;
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{method, path},
};

use crate::helpers::{create_unconfirmed_subscriber, spawn_test_app};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;
    create_unconfirmed_subscriber(&app).await?;

    Mock::given(path("/emails"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>"
        }
    });

    let response = app
        .post_newsletters(body, Some(app.get_test_user_creds().await))
        .await?;

    // Assert
    assert_eq!(response.status().as_u16(), 200);

    Ok(())
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;
    app.create_confirmed_subscriber().await?;

    Mock::given(path("/emails"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>"
        }
    });

    let response = app
        .post_newsletters(body, Some(app.get_test_user_creds().await))
        .await?;

    // Assert
    assert_eq!(response.status().as_u16(), 200);
    Ok(())
}

#[tokio::test]
async fn newsletters_return_400_for_invalid_data() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;
    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>"
                }
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!"
            }),
            "missing content",
        ),
    ];

    for (invalid_body, description) in test_cases {
        // Act
        let response = app
            .post_newsletters(invalid_body, Some(app.get_test_user_creds().await))
            .await?;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 BAD REQUEST when the payload was {}.",
            description
        );
    }

    Ok(())
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;

    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>"
        }
    });

    // Act
    let response = app.post_newsletters::<String>(body, None).await?;

    // Assert
    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        "Basic realm=\"publish\"",
        response.headers()["WWW-Authenticate"]
    );
    Ok(())
}

#[tokio::test]
async fn non_existing_user_is_rejected() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;
    let creds = (Uuid::new_v4(), Uuid::new_v4());

    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>"
        }
    });

    // Act
    let response = app.post_newsletters(body, Some(creds)).await?;

    // Assert
    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        "Basic realm=\"publish\"",
        response.headers()["WWW-Authenticate"]
    );
    Ok(())
}

#[tokio::test]
async fn invalid_password_is_rejected() -> Result<()> {
    // Arrange
    let app = spawn_test_app().await?;
    let user = app.get_test_user_creds().await;
    let invalid_creds = (user.0, Uuid::new_v4().to_string());

    let body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>"
        }
    });

    // Act
    let response = app.post_newsletters(body, Some(invalid_creds)).await?;

    // Assert
    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        "Basic realm=\"publish\"",
        response.headers()["WWW-Authenticate"]
    );
    Ok(())
}
