use crate::games::music_quiz::MusicQuizError;
use serenity::Error as SerenityError;
use songbird::error::JoinError;
use thiserror::Error;
use url::ParseError;

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
    SendingMessageFailed(#[source] SerenityError),

    #[error(transparent)]
    MusicQuiz(#[from] MusicQuizError),

    #[error("Could not leave channel: {0}")]
    CouldNotLeaveChannel(#[source] JoinError),

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[source] ParseError),

    #[error("Error creating response: {0}")]
    ErrorCreatingResponse(#[source] SerenityError),

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

    #[error("Failed to join voice channel: {0}")]
    FailedToJoinVoiceChannel(#[source] JoinError),
}
