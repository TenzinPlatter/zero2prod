use std::time::Duration;

use anyhow::Result;
use reqwest::{Client, Url};
use secrecy::{ExposeSecret, Secret};
use tracing::instrument;

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    sender: SubscriberEmail,
    base_url: String,
    http_client: Client,
    auth_token: Secret<String>,
    timeout: std::time::Duration,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl EmailClient {
    pub fn new(
        sender: SubscriberEmail,
        base_url: String,
        auth_token: Secret<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            sender,
            base_url,
            http_client: Client::new(),
            auth_token,
            timeout,
        }
    }

    #[instrument("Sending email", skip(self))]
    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<()> {
        let url = Url::parse(self.base_url.as_str())?.join("/email")?;
        let body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        self.http_client
            .post(url)
            .header("X-Postmark-Server-Token", self.auth_token.expose_secret())
            .json(&body)
            .timeout(self.timeout)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use claims::{assert_err, assert_ok};
    use fake::{
        Fake, Faker,
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
    };
    use secrecy::Secret;
    use wiremock::{
        Match, Mock, MockServer, ResponseTemplate,
        matchers::{any, header, header_exists, method, path},
    };

    use crate::{domain::SubscriberEmail, email_client::EmailClient};

    struct SendEmailBodyMatcher;

    impl Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String) -> EmailClient {
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        EmailClient::new(
            sender,
            base_url,
            Secret::new(Faker.fake()),
            Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(SendEmailBodyMatcher)
            .and(header("Content-type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = email();
        let subject = subject();
        let content = content();

        let _ = email_client
            .send_email(&subscriber_email, &subject, &content, &content)
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = email();
        let subject = subject();
        let content = content();

        let outcome = email_client
            .send_email(&subscriber_email, &subject, &content, &content)
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = email();
        let subject = subject();
        let content = content();

        let outcome = email_client
            .send_email(&subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(180)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = email();
        let subject = subject();
        let content = content();

        let outcome = email_client
            .send_email(&subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }
}
