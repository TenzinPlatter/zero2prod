use crate::auth::Credentials;
use actix_web::{HttpRequest, HttpResponse, ResponseError, http::header::HeaderMap, web};
use anyhow::Context;
use base64::{Engine, prelude::BASE64_STANDARD};
use secrecy::Secret;
use sqlx::PgPool;
use tracing::{error, instrument, warn};

use crate::{auth::validate_credentials, domain::SubscriberEmail, email_client::EmailClient};

type Result<T> = std::result::Result<T, PublishNewsletterError>;

#[derive(thiserror::Error, Debug)]
pub enum PublishNewsletterError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct ConfirmedSubscriber {
    pub email: SubscriberEmail,
}

#[derive(serde::Deserialize, Debug)]
pub struct Content {
    pub html: String,
    pub text: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct BodyData {
    pub title: String,
    pub content: Content,
}
impl ResponseError for PublishNewsletterError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            Self::UnexpectedError(_) => HttpResponse::InternalServerError().finish(),
            Self::AuthError(_) => {
                let mut response = HttpResponse::Unauthorized();
                response.append_header(("WWW-Authenticate", r#"Basic realm="publish""#));
                response.finish()
            }
        }
    }
}

#[instrument("Sending newsletters", skip(pool, email_client))]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse> {
    let credentials = basic_authentication(request.headers())
        .map_err(|e| PublishNewsletterError::AuthError(e.into()))?;

    // ignore the returned id as we have no use for it
    let _ = validate_credentials(&pool, &credentials)
        .await
        .map_err(|e| PublishNewsletterError::AuthError(e.into()))?;

    let active_subs = get_active_subscribers(pool.get_ref())
        .await?
        .into_iter()
        .filter_map(|s| match s {
            Ok(subscriber) => Some(subscriber),
            Err(e) => {
                warn!(e.cause_chain = ?e, "Skipping a confirmed subscriber as stored contact details are invalid. {}", e);
                None
            }
        });

    for subscriber in active_subs {
        email_client
            .send_email(
                &subscriber.email,
                &body.title,
                &body.content.html,
                &body.content.text,
            )
            .await
            .with_context(|| format!("Failed to send email to: {}", subscriber.email.as_ref()))?;
    }

    Ok(HttpResponse::Ok().finish())
}

#[instrument("Fetching active subscribers", skip(pool))]
async fn get_active_subscribers(pool: &PgPool) -> Result<Vec<Result<ConfirmedSubscriber>>> {
    let emails = sqlx::query!(
        r#"
        SELECT email FROM subscriptions WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to query DB for active subscribers")?
    .into_iter()
    .map(|r| match r.email.parse() {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(e) => Err(PublishNewsletterError::UnexpectedError(anyhow::anyhow!(e))),
    })
    .collect();

    Ok(emails)
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials> {
    let header_value = headers
        .get("Authorization")
        .context("Authorization header was not present")?
        .to_str()
        .context("Failed to convert to str")?;

    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("Authorization header is not a Basic auth")?;

    let bytes = BASE64_STANDARD
        .decode(base64encoded_segment)
        .context("Failed to decode base64 encoded segment of Basic auth")?;

    let creds = String::from_utf8(bytes).context("Basic auth is not valid UTF8")?;

    let mut credentials = creds.splitn(2, ':');
    let username = credentials
        .next()
        .context("There is no username in Basic auth")?
        .to_string();

    let password = credentials
        .next()
        .context("There is no password in Basic auth")?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}
