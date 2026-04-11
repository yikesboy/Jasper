use thiserror::Error;
use url::ParseError;

#[derive(Error, Debug)]
pub enum ITunesAPIError {
    #[error("Request failed: {0}")]
    RequestFailed(#[source] reqwest::Error),

    #[error("Request unsuccessful")]
    RequestUnsuccessful,

    #[error("Invalid response body: {0}")]
    InvalidResponseBody(#[source] reqwest::Error),

    #[error("Invalid preview URL: {0}")]
    InvalidPreviewUrl(#[source] ParseError),
}
