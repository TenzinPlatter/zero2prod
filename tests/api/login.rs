use crate::helpers::{assert_is_redirect_to, spawn_test_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() -> anyhow::Result<()> {
    let app = spawn_test_app().await?;
    let body = serde_json::json!({
        "username": "random",
        "password": "random"
    });

    let response = app.post_login(&body).await?;

    assert_is_redirect_to(&response, "/login");

    let login_html = app.get_login_html().await?;
    assert!(login_html.contains(r#"<p><i>Authentication error</i></p>"#));

    assert!(
        !app.get_login_html()
            .await?
            .contains(r#"Authentication error"#)
    );

    Ok(())
}
