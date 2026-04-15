use reqwest::Error;
use reqwest::StatusCode;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum SpotifyClientError {
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

    #[error("Authentication request failed: {0}")]
    AuthenticationRequestFailed(#[source] Error),

    #[error("Authentication rejected with status {0}")]
    AuthenticationRejected(StatusCode),

    #[error("Invalid authentication response: {0}")]
    AuthenticationResponseInvalid(#[source] Error),

    #[error("Failed to retrieve playlist tracks: Spotify returned {status} ({body})")]
    FailedToRetrievePlaylistTracks { status: StatusCode, body: String },

    #[error("Invalid playlist response: {0}")]
    InvalidPlaylistResponse(#[source] Error),

    #[error("Request failed: {0}")]
    RequestFailed(#[source] Error),
}
