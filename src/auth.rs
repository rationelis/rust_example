//! JWT authentication with dev mode bypass.
//!
//! This module demonstrates how to implement authentication as a cross-cutting concern
//! using Poem's `FromRequest` trait. The `AuthenticatedUser` extractor can be added to
//! any handler to require authentication.

use std::sync::Arc;

use jsonwebtoken::{decode, DecodingKey, Validation};
use poem::{error::ResponseError, http::StatusCode, FromRequest, Request};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ACCESS_TOKEN_COOKIE: &str = "notes_access_token";

/// Header used in dev mode to specify the user ID without JWT authentication.
/// **WARNING**: Dev mode should never be enabled in production!
const DEV_USER_ID_HEADER: &str = "x-dev-user-id";

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub dev_mode: bool,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    user_id: String,
}

impl AuthenticatedUser {
    #[must_use]
    pub fn user_id(&self) -> &str {
        &self.user_id
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String,
    exp: i64,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Authentication configuration not found")]
    ConfigNotFound,

    #[error("No authentication token provided")]
    TokenNotProvided,

    #[error("Invalid token: {0}")]
    InvalidToken(String),
}

impl ResponseError for AuthError {
    fn status(&self) -> StatusCode {
        match self {
            Self::ConfigNotFound => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TokenNotProvided | Self::InvalidToken(_) => StatusCode::UNAUTHORIZED,
        }
    }
}

impl<'a> FromRequest<'a> for AuthenticatedUser {
    async fn from_request(req: &'a Request, _body: &mut poem::RequestBody) -> poem::Result<Self> {
        let config = req
            .data::<Arc<AuthConfig>>()
            .ok_or(AuthError::ConfigNotFound)?;

        if config.dev_mode {
            return extract_dev_user(req);
        }

        extract_jwt_user(req, config)
    }
}

fn extract_dev_user(req: &Request) -> poem::Result<AuthenticatedUser> {
    let user_id = req
        .header(DEV_USER_ID_HEADER)
        .ok_or(AuthError::TokenNotProvided)?
        .to_string();

    tracing::debug!(user_id = %user_id, "Dev mode auth");

    Ok(AuthenticatedUser { user_id })
}

fn extract_jwt_user(req: &Request, config: &AuthConfig) -> poem::Result<AuthenticatedUser> {
    let token = extract_token(req)?;

    let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());
    let validation = Validation::default();

    let token_data = decode::<JwtClaims>(&token, &decoding_key, &validation)
        .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

    tracing::debug!(user_id = %token_data.claims.sub, "JWT auth");

    Ok(AuthenticatedUser {
        user_id: token_data.claims.sub,
    })
}

fn extract_token(req: &Request) -> Result<String, AuthError> {
    if let Some(auth_header) = req.header("authorization") {
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AuthError::InvalidToken("Invalid Authorization header format".into()))?;
        return Ok(token.to_string());
    }

    if let Some(cookie) = req.cookie().get(ACCESS_TOKEN_COOKIE) {
        return Ok(cookie.value_str().to_string());
    }

    Err(AuthError::TokenNotProvided)
}
