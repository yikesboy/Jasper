mod preparation;
mod notifier;
mod runtime;
mod voice;

use crate::commands::MusicQuizCommandError;
use crate::games::music_quiz::quiz::MusicQuiz;
use crate::Context;
use std::sync::Arc;
use tracing::{info, warn};
use url::Url;
use preparation::MusicQuizPreparationService;
use runtime::{
    MusicQuizRuntime, PreparedMusicQuiz, get_user_voice_channel, get_voice_channel_participants,
};

const TOTAL_ROUNDS_DEFAULT: u32 = 5;

pub async fn start_from_command(
    ctx: Context<'_>,
    playlist: String,
    total_rounds: Option<u32>,
) -> Result<(), MusicQuizCommandError> {
    let guild_id = ctx
        .guild_id()
        .ok_or(MusicQuizCommandError::MustBeUsedInGuild)?;
    let requested_rounds = total_rounds.unwrap_or(TOTAL_ROUNDS_DEFAULT);

    info!(
        guild_id = guild_id.get(),
        user_id = ctx.author().id.get(),
        requested_rounds,
        "Received music quiz command"
    );

    let runtime = MusicQuizRuntime::new(
        ctx.serenity_context().clone(),
        ctx.channel_id(),
        guild_id,
        Arc::clone(&ctx.data().game_state),
    );

    let prepared = match prepare_quiz(&ctx, playlist, Some(requested_rounds)).await {
        Ok(prepared) => prepared,
        Err(error) => {
            warn!(
                guild_id = guild_id.get(),
                user_id = ctx.author().id.get(),
                error = %error,
                "Music quiz preparation failed"
            );
            return Err(error);
        }
    };

    ctx.say("🎵 Starting Music Quiz! Joining voice channel...")
        .await
        .map_err(|error| {
            warn!(
                guild_id = guild_id.get(),
                user_id = ctx.author().id.get(),
                error = %error,
                "Failed to send music quiz startup response"
            );
            MusicQuizCommandError::ErrorCreatingResponse(error)
        })?;

    runtime.start(prepared).await.map_err(|error| {
        warn!(
            guild_id = guild_id.get(),
            user_id = ctx.author().id.get(),
            error = %error,
            "Failed to launch music quiz runtime"
        );
        error
    })?;

    info!(
        guild_id = guild_id.get(),
        user_id = ctx.author().id.get(),
        requested_rounds,
        "Music quiz runtime launched"
    );

    Ok(())
}

async fn prepare_quiz(
    ctx: &Context<'_>,
    playlist: String,
    total_rounds: Option<u32>,
) -> Result<PreparedMusicQuiz, MusicQuizCommandError> {
    let spotify_playlist =
        Url::parse(&playlist).map_err(MusicQuizCommandError::InvalidUrl)?;

    let guild_id = ctx
        .guild_id()
        .ok_or(MusicQuizCommandError::MustBeUsedInGuild)?;

    let voice_channel_id = get_user_voice_channel(ctx, guild_id, ctx.author().id)?;
    let participants = get_voice_channel_participants(ctx, guild_id, voice_channel_id)?;
    validate_participant_count(participants.len())?;

    let total_rounds = total_rounds.unwrap_or(TOTAL_ROUNDS_DEFAULT);
    info!(
        guild_id = guild_id.get(),
        voice_channel_id = voice_channel_id.get(),
        participant_count = participants.len(),
        total_rounds,
        "Prepared music quiz context"
    );
    let quiz = MusicQuiz::new(total_rounds, participants);
    let tracks =
        MusicQuizPreparationService::fetch_quiz_tracks(ctx.data(), spotify_playlist, total_rounds)
            .await?;

    Ok(PreparedMusicQuiz::new(quiz, voice_channel_id, tracks))
}

fn validate_participant_count(participant_count: usize) -> Result<(), MusicQuizCommandError> {
    if participant_count < 1 {
        return Err(MusicQuizCommandError::TooFewUsersInChannel {
            actual: participant_count,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_participant_count;
    use crate::commands::MusicQuizCommandError;

    #[test]
    fn one_human_user_is_enough_to_start_quiz() {
        assert!(validate_participant_count(1).is_ok());
    }

    #[test]
    fn multiple_human_users_are_still_allowed() {
        assert!(validate_participant_count(3).is_ok());
    }

    #[test]
    fn zero_human_users_is_rejected() {
        let error = validate_participant_count(0).unwrap_err();

        assert!(matches!(
            error,
            MusicQuizCommandError::TooFewUsersInChannel { actual: 0 }
        ));
    }
}
