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

    #[error(
        "Spotify authorization has expired or been revoked. Reauthorize the operator's Spotify account and replace SPOTIFY_REFRESH_TOKEN."
    )]
    ReauthorizationRequired,

    #[error("Spotify returned an empty token or invalid token lifetime")]
    InvalidTokenLifetime,

    #[error(
        "Spotify denied playlist access. In development mode, the authorized Spotify user must own or collaborate on the playlist. Check playlist-read-private permission, the app's allowed users, and the app owner's Premium subscription."
    )]
    PlaylistAccessDenied,

    #[error("Invalid authentication response: {0}")]
    AuthenticationResponseInvalid(#[source] Error),

    #[error("Failed to retrieve playlist tracks: Spotify returned {status} ({body})")]
    FailedToRetrievePlaylistTracks { status: StatusCode, body: String },

    #[error("Invalid playlist response: {0}")]
    InvalidPlaylistResponse(#[source] Error),

    #[error(
        "Spotify did not return playlist contents. Development-mode apps require SPOTIFY_REFRESH_TOKEN for a user who owns or collaborates on this playlist; public visibility alone is not sufficient."
    )]
    PlaylistContentsUnavailable,

    #[error("Request failed: {0}")]
    RequestFailed(#[source] Error),
}
