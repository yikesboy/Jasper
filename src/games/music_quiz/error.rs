use crate::services::itunes::ITunesAPIError;
use crate::services::spotify::SpotifyAPIError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MusicQuizError {
    #[error("There is no track running.")]
    NoTrackRunning,

    #[error("There is no round in progress.")]
    NoRoundInProgress,

    #[error("Failed to fetch tracks from playlist")]
    FailedToFetchTracksFromPlaylist(#[source] SpotifyAPIError),

    #[error("Playlist contains not enough songs. Expected: {expected} Actual: {actual}")]
    PlaylistContainsNotEnoughSongs { expected: u32, actual: u32 },

    #[error("Playlist contains not enough previewable songs. Expected: {expected} Actual: {actual}")]
    PlaylistContainsNotEnoughPreviewableSongs { expected: u32, actual: u32 },

    #[error("Failed to fetch track preview")]
    FetchError(#[source] ITunesAPIError),

    #[error("Unable to lock mutex.")]
    UnableToLockMutex,
}
