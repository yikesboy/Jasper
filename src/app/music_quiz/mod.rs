mod preparation;
mod runtime;

use crate::commands::MusicQuizCommandError;
use crate::games::music_quiz::quiz::MusicQuiz;
use crate::Context;
use std::sync::Arc;
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

    let runtime = MusicQuizRuntime::new(
        ctx.serenity_context().clone(),
        ctx.channel_id(),
        guild_id,
        Arc::clone(&ctx.data().game_state),
    );

    let prepared = prepare_quiz(&ctx, playlist, total_rounds).await?;

    ctx.say("🎵 Starting Music Quiz! Joining voice channel...")
        .await
        .map_err(MusicQuizCommandError::ErrorCreatingResponse)?;

    runtime.start(prepared).await
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
    if participants.len() < 2 {
        return Err(MusicQuizCommandError::TooFewUsersInChannel {
            actual: participants.len(),
        });
    }

    let total_rounds = total_rounds.unwrap_or(TOTAL_ROUNDS_DEFAULT);
    let quiz = MusicQuiz::new(total_rounds, participants);
    let tracks =
        MusicQuizPreparationService::fetch_quiz_tracks(ctx.data(), spotify_playlist, total_rounds)
            .await?;

    Ok(PreparedMusicQuiz::new(quiz, voice_channel_id, tracks))
}
