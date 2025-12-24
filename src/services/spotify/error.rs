use reqwest::Error;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum SpotifyAPIError {
    #[error("Not a spotify link: {0}")]
    InvalidLink(Url),

    #[error("Malformed url: {0}")]
    MalformedUrl(String),

    #[error("Not a playlist link: {0}")]
    NotPlaylistLink(Url),

    #[error("Unexpected Link Segment: {0}")]
    UnexpectedSegment(String),

    #[error("Playlist Id Missing")]
    PlaylistIdMissing,

    #[error("Failed to authenticate: {0}")]
    FailedToAuthenticate(String),

    #[error("Failed to retrieve playlist tracks")]
    FailedToRetrievePlaylistTracks,

    #[error("Request failed: {0}")]
    RequestFailed(Error),
}
