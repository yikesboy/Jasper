use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum ITunesAPIError {
    #[error("Not a spotify link: {0}")]
    InvalidLink(Url),

    #[error("Request failed: {0}")]
    RequestFailed(reqwest::Error),

    #[error("Request unsuccessful")]
    RequestUnsuccessful,

    #[error("Invalid response body")]
    InvalidResponseBody,
}
