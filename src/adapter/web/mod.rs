//! Web handlers, DTOs, and error responses.
//!
//! This module is the "driving adapter" in hexagonal architecture terminology.
//! It translates HTTP requests into domain operations and domain results back
//! into HTTP responses. The domain layer has no knowledge of HTTP concerns.

pub mod note;

use poem::http::StatusCode;
use poem_openapi::Tags;
use serde::Serialize;

pub use note::NoteApi;

#[derive(Debug, Tags)]
pub enum NotesApiTags {
    Notes,
}

/// RFC 7807 error response.
#[derive(Debug, Clone, Serialize, poem_openapi::Object)]
pub struct ErrorResponse {
    #[oai(rename = "type")]
    #[serde(rename = "type")]
    pub error_type: String,
    pub status: u16,
    pub title: String,
    #[oai(skip_serializing_if_is_none)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ErrorResponse {
    #[must_use]
    pub fn new(status: StatusCode, title: impl Into<String>) -> Self {
        Self {
            error_type: "urn:notes-api:error".to_string(),
            status: status.as_u16(),
            title: title.into(),
            detail: None,
        }
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Generates an error response enum with OpenAPI support.
///
/// This macro reduces boilerplate when defining HTTP error types that need to:
/// - Implement `Display` and `Error` for standard error handling
/// - Implement `ResponseError` for Poem's error conversion
/// - Implement `ApiResponse` for OpenAPI documentation
///
/// # Example
///
/// ```ignore
/// gen_error_response! {
///     MyError {
///         NotFound -> (404, "Resource not found"),
///         Forbidden -> (403, "Access denied"),
///     }
/// }
/// ```
macro_rules! gen_error_response {
    ( $name:ident { $( $key:ident -> ( $status:literal, $title:literal ) ),* $(,)? } ) => {
        #[derive(Debug)]
        pub enum $name {
            $( $key, )*
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( Self::$key => write!(f, $title), )*
                }
            }
        }

        impl std::error::Error for $name {}

        impl poem::error::ResponseError for $name {
            fn status(&self) -> poem::http::StatusCode {
                match self {
                    $( Self::$key => poem::http::StatusCode::from_u16($status).unwrap(), )*
                }
            }

            fn as_response(&self) -> poem::Response {
                use poem::IntoResponse;
                let error_response = $crate::adapter::web::ErrorResponse::new(self.status(), self.to_string());
                poem::web::Json(error_response).with_status(self.status()).into_response()
            }
        }

        impl poem_openapi::ApiResponse for $name {
            fn meta() -> poem_openapi::registry::MetaResponses {
                poem_openapi::registry::MetaResponses {
                    responses: vec![
                        $(
                            poem_openapi::registry::MetaResponse {
                                description: $title,
                                status: Some($status),
                                status_range: None,
                                content: vec![poem_openapi::registry::MetaMediaType {
                                    content_type: "application/json",
                                    schema: poem_openapi::registry::MetaSchemaRef::Reference("ErrorResponse".to_string()),
                                }],
                                headers: vec![],
                            },
                        )*
                    ],
                }
            }

            fn register(registry: &mut poem_openapi::registry::Registry) {
                <poem_openapi::payload::Json<$crate::adapter::web::ErrorResponse>
                    as poem_openapi::ResponseContent>::register(registry);
            }
        }
    };
}

pub(crate) use gen_error_response;
