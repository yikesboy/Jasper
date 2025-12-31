use thiserror::Error;
#[derive(Error, Debug)]
pub enum MusicQuizCommandError {
    #[error("Unable to get ChannelID")]
    UnableToGetChannelId,

    #[error("Guild not cached")]
    GuildNotCached,

    #[error("User is not in a voice channel")]
    UserNotInVoiceChannel,

    #[error("User has no ChannelID")]
    UserHasNoChannelID,

    #[error("Songbird call does not exist")]
    SongbirdCallDoesNotExist,

    #[error("Sending message failed: {0}")]
    SendingMessageFailed(String),

    #[error("Music quiz error: {0}")]
    MusicQuizError(String),

    #[error("Could not leave channel: {0}")]
    CouldNotLeaveChannel(String),

    #[error("Invalid URL: {0}")]
    InvalidURL(String),

    #[error("Error creating response: {0}")]
    ErrorCreatingResponse(String),

    #[error("GameState error: {0}")]
    GameStateError(String),

    #[error("Playlist URL not provided")]
    PlaylistUrlNotProvided,

    #[error("Command must be used in a guild")]
    MustBeUsedInGuild,

    #[error("Songbird not initialized")]
    SongbirdNotInitialized,

    #[error("Game is already running on the server.")]
    GameAlreadyRunningInGuild,

    #[error("Too few users in channel.")]
    ToFewUsersInChannel,

    #[error("Failed to join voice channel")]
    FailedToJoinVoiceChannel,

    #[error("Failed to run quiz: {0}")]
    FailedWhileRunningQuiz(String),
}
