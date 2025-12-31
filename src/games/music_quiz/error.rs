use thiserror::Error;

#[derive(Error, Debug)]
pub enum MusicQuizError {
    #[error("There is no track running.")]
    NoTrackRunning,

    #[error("There is no round in progress.")]
    NoRoundInProgress,

    #[error("Failed to fetch tracks from playlist {0}")]
    FailedToFetchTracksFromPlaylist(String),

    #[error("Playlist contains not enough songs. Expected: {expected} Actual: {actual}")]
    PlaylistContainsNotEnoughSongs { expected: u32, actual: u32 },

    #[error("Failed to fetch tracks: {0}")]
    FetchError(String),

    #[error("Unable to lock mutex.")]
    UnableToLockMutex,
}
