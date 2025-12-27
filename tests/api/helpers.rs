use anyhow::Result;

use zero2prod::AppHandle;

pub(crate) async fn post_subscriptions(app: &AppHandle, body: String) -> Result<reqwest::Response> {
    Ok(reqwest::Client::new()
        .post(format!("{}/subscriptions", app.config.app_address()))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?)
}
