use super::notifier::MusicQuizNotifier;
use super::voice::MusicQuizVoice;
use crate::commands::MusicQuizCommandError;
use crate::games::music_quiz::{MusicQuizHandle, QuizTrack, quiz::MusicQuiz};
use crate::games::state::{GameState, GameStateError};
use crate::Context;
use serenity::all::{ChannelId, GuildId, UserId};
use serenity::client::Context as SerenityContext;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

pub(crate) struct PreparedMusicQuiz {
    quiz: MusicQuiz,
    voice_channel_id: ChannelId,
    tracks: Vec<QuizTrack>,
}

impl PreparedMusicQuiz {
    pub(crate) fn new(quiz: MusicQuiz, voice_channel_id: ChannelId, tracks: Vec<QuizTrack>) -> Self {
        Self {
            quiz,
            voice_channel_id,
            tracks,
        }
    }
}

pub struct MusicQuizRuntime {
    notifier: MusicQuizNotifier,
    voice: MusicQuizVoice,
    game_state: Arc<GameState>,
    guild_id: GuildId,
}

impl MusicQuizRuntime {
    pub fn new(
        serenity_ctx: SerenityContext,
        response_channel_id: ChannelId,
        guild_id: GuildId,
        game_state: Arc<GameState>,
    ) -> Self {
        Self {
            notifier: MusicQuizNotifier::new(serenity_ctx.clone(), response_channel_id),
            voice: MusicQuizVoice::new(serenity_ctx, guild_id),
            guild_id,
            game_state,
        }
    }

    pub async fn start(self, prepared: PreparedMusicQuiz) -> Result<(), MusicQuizCommandError> {
        let round_count = prepared.tracks.len();
        let quiz = MusicQuizHandle::new(prepared.quiz);

        self.game_state
            .start_quiz(self.guild_id, quiz.clone())
            .map_err(|error| match error {
                GameStateError::GameAlreadyActiveInServer => {
                    MusicQuizCommandError::GameAlreadyRunningInGuild
                }
            })?;

        if let Err(error) = self.voice.join(prepared.voice_channel_id).await {
            self.game_state.end_game(self.guild_id);
            return Err(error);
        }

        info!(
            guild_id = self.guild_id.get(),
            voice_channel_id = prepared.voice_channel_id.get(),
            rounds = round_count,
            "Music quiz runtime started"
        );
        self.spawn_run_task(quiz, prepared.tracks);
        Ok(())
    }

    fn spawn_run_task(self, quiz: MusicQuizHandle, tracks: Vec<QuizTrack>) {
        tokio::spawn(async move {
            let result = self.run(quiz, tracks).await;

            if let Err(error) = self.voice.leave().await {
                warn!(
                    guild_id = self.guild_id.get(),
                    error = %error,
                    "Failed to leave voice channel during music quiz cleanup"
                );
            }
            self.game_state.end_game(self.guild_id);

            match result {
                Ok(()) => {
                    info!(guild_id = self.guild_id.get(), "Music quiz finished successfully");
                }
                Err(error) => {
                    error!(
                        guild_id = self.guild_id.get(),
                        error = %error,
                        "Music quiz runtime failed"
                    );

                    if let Err(notification_error) = self.notifier.send_failure(&error).await {
                        error!(
                            guild_id = self.guild_id.get(),
                            error = %notification_error,
                            "Failed to send music quiz failure notification"
                        );
                    }
                }
            }
        });
    }

    async fn run(
        &self,
        quiz: MusicQuizHandle,
        tracks: Vec<QuizTrack>,
    ) -> Result<(), MusicQuizCommandError> {
        for track in tracks {
            let preview_url = track.preview_url().clone();
            quiz.start_round(track).await;
            let progress = quiz.round_progress().await;

            debug!(
                guild_id = self.guild_id.get(),
                round = progress.round_number,
                total_rounds = progress.total_rounds,
                "Starting music quiz round"
            );

            self.notifier.send_round_start(&quiz).await?;
            self.voice.play_preview(&preview_url).await?;
            let round_end = wait_for_track_finished_or_timeout(&quiz).await;
            self.voice.stop().await?;
            self.notifier.send_round_completion(&quiz).await?;
            debug!(
                guild_id = self.guild_id.get(),
                round = progress.round_number,
                reason = round_end.as_str(),
                "Music quiz round finished"
            );
            delay_between_rounds(&quiz).await;
        }

        self.notifier.send_final_results(&quiz).await
    }
}

enum RoundEndReason {
    GuessedEarly,
    TimedOut,
}

impl RoundEndReason {
    fn as_str(&self) -> &'static str {
        match self {
            Self::GuessedEarly => "guessed_early",
            Self::TimedOut => "timed_out",
        }
    }
}

pub fn get_user_voice_channel(
    ctx: &Context<'_>,
    guild_id: GuildId,
    user_id: UserId,
) -> Result<ChannelId, MusicQuizCommandError> {
    let guild = guild_id
        .to_guild_cached(&ctx.serenity_context().cache)
        .ok_or(MusicQuizCommandError::GuildNotCached)?;

    let voice_state = guild
        .voice_states
        .get(&user_id)
        .ok_or(MusicQuizCommandError::UserNotInVoiceChannel)?;

    voice_state
        .channel_id
        .ok_or(MusicQuizCommandError::UserHasNoChannelId)
}

pub fn get_voice_channel_participants(
    ctx: &Context<'_>,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<Vec<UserId>, MusicQuizCommandError> {
    let guild = guild_id
        .to_guild_cached(&ctx.serenity_context().cache)
        .ok_or(MusicQuizCommandError::GuildNotCached)?;

    Ok(guild
        .voice_states
        .values()
        .filter(|voice_state| voice_state.channel_id == Some(channel_id))
        .map(|voice_state| voice_state.user_id)
        .filter(|user_id| {
            guild
                .members
                .get(user_id)
                .map(|member| !member.user.bot)
                .unwrap_or(false)
        })
        .collect())
}

async fn wait_for_track_finished_or_timeout(quiz: &MusicQuizHandle) -> RoundEndReason {
    let notify = quiz.notify_round_complete().await;

    tokio::select! {
        _ = notify.notified() => RoundEndReason::GuessedEarly,
        _ = tokio::time::sleep(Duration::from_secs(25)) => RoundEndReason::TimedOut,
    }
}

async fn delay_between_rounds(quiz: &MusicQuizHandle) {
    let is_finished = quiz.is_finished().await;

    if !is_finished {
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
