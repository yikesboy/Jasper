use crate::commands::MusicQuizCommandError;
use crate::games::music_quiz::quiz::MusicQuiz;
use crate::games::state::{GameState, GameStateError};
use crate::services::itunes::TrackInfo;
use crate::Context;
use reqwest::Client;
use serenity::all::{ChannelId, CreateEmbed, CreateMessage, GuildId, UserId};
use serenity::client::Context as SerenityContext;
use songbird::input::HttpRequest;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;

const TOTAL_ROUNDS_DEFAULT: u32 = 5;

pub async fn start_from_command(
    ctx: Context<'_>,
    playlist: String,
    total_rounds: Option<u32>,
) -> Result<(), MusicQuizCommandError> {
    let guild_id = ctx
        .guild_id()
        .ok_or(MusicQuizCommandError::MustBeUsedInGuild)?;

    let runtime = QuizRuntime::new(
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

struct PreparedMusicQuiz {
    quiz: MusicQuiz,
    voice_channel_id: ChannelId,
    tracks: Vec<TrackInfo>,
}

struct QuizRuntime {
    serenity_ctx: SerenityContext,
    response_channel_id: ChannelId,
    guild_id: GuildId,
    game_state: Arc<GameState>,
}

impl QuizRuntime {
    fn new(
        serenity_ctx: SerenityContext,
        response_channel_id: ChannelId,
        guild_id: GuildId,
        game_state: Arc<GameState>,
    ) -> Self {
        Self {
            serenity_ctx,
            response_channel_id,
            guild_id,
            game_state,
        }
    }

    async fn start(self, prepared: PreparedMusicQuiz) -> Result<(), MusicQuizCommandError> {
        let quiz = Arc::new(Mutex::new(prepared.quiz));

        self.game_state
            .start_quiz(self.guild_id, Arc::clone(&quiz))
            .map_err(|error| match error {
                GameStateError::GameAlreadyActiveInServer => {
                    MusicQuizCommandError::GameAlreadyRunningInGuild
                }
            })?;

        let songbird = songbird::get(&self.serenity_ctx)
            .await
            .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

        if let Err(error) = songbird
            .join(self.guild_id, prepared.voice_channel_id)
            .await
        {
            self.game_state.end_game(self.guild_id);
            return Err(MusicQuizCommandError::FailedToJoinVoiceChannel(error));
        }

        self.spawn_run_task(quiz, prepared.tracks);
        Ok(())
    }

    fn spawn_run_task(self, quiz: Arc<Mutex<MusicQuiz>>, tracks: Vec<TrackInfo>) {
        tokio::spawn(async move {
            let result = self.run(Arc::clone(&quiz), tracks).await;

            let _ = self.leave_voice_channel().await;
            self.game_state.end_game(self.guild_id);

            if let Err(error) = result {
                let _ = self.send_failure_message(&error).await;
            }
        });
    }

    async fn run(
        &self,
        quiz: Arc<Mutex<MusicQuiz>>,
        tracks: Vec<TrackInfo>,
    ) -> Result<(), MusicQuizCommandError> {
        for track in tracks {
            {
                let mut locked_quiz = quiz.lock().await;
                locked_quiz.session_mut().start_round(track.clone());
            }

            self.send_round_start_message(&quiz).await?;
            self.play_track(&track.preview_url).await?;
            wait_for_track_finished_or_timeout(&quiz).await;
            self.stop_audio().await?;
            self.send_round_completion_message(&quiz).await?;
            delay_between_rounds(&quiz).await;
        }

        self.send_final_results(&quiz).await
    }

    async fn send_round_start_message(
        &self,
        quiz: &Arc<Mutex<MusicQuiz>>,
    ) -> Result<(), MusicQuizCommandError> {
        let (round_number, total_rounds) = {
            let locked_quiz = quiz.lock().await;
            (
                locked_quiz.session().round_number,
                locked_quiz.session().total_rounds,
            )
        };

        self.response_channel_id
            .send_message(
                &self.serenity_ctx.http,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .title(format!("🎵 Round {}/{}", round_number, total_rounds))
                        .description("Listen to the song and guess the artist and/or track name!")
                        .color(0x3498db),
                ),
            )
            .await
            .map_err(MusicQuizCommandError::SendingMessageFailed)?;

        Ok(())
    }

    async fn send_round_completion_message(
        &self,
        quiz: &Arc<Mutex<MusicQuiz>>,
    ) -> Result<(), MusicQuizCommandError> {
        let locked_quiz = quiz.lock().await;

        let round = match &locked_quiz.session().current_round {
            Some(round) => round,
            None => return Ok(()),
        };

        let mut description = format!(
            "**Song:** {} by {}\n\n",
            round.track.track_name, round.track.artist_name
        );

        if let Some(artist_guesser) = round.artist_guessed_by {
            description.push_str(&format!("🎤 Artist guessed by <@{}>\n", artist_guesser));
        }

        if let Some(track_guesser) = round.track_guessed_by {
            description.push_str(&format!("🎵 Track guessed by <@{}>\n", track_guesser));
        }

        self.response_channel_id
            .send_message(
                &self.serenity_ctx.http,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .title("Round Complete!")
                        .description(description)
                        .color(0x2ecc71),
                ),
            )
            .await
            .map_err(MusicQuizCommandError::SendingMessageFailed)?;

        Ok(())
    }

    async fn send_final_results(
        &self,
        quiz: &Arc<Mutex<MusicQuiz>>,
    ) -> Result<(), MusicQuizCommandError> {
        let leaderboard = {
            let locked_quiz = quiz.lock().await;
            locked_quiz.session().get_leaderboard()
        };

        let mut description = String::from("**Final Scores:**\n\n");

        for (index, (user_id, score)) in leaderboard.iter().enumerate() {
            let medal = match index {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            description.push_str(&format!("{} <@{}> - {} points\n", medal, user_id, score));
        }

        self.response_channel_id
            .send_message(
                &self.serenity_ctx.http,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .title("🎉 Music Quiz Complete!")
                        .description(description)
                        .color(0xf39c12),
                ),
            )
            .await
            .map_err(MusicQuizCommandError::SendingMessageFailed)?;

        Ok(())
    }

    async fn send_failure_message(
        &self,
        error: &MusicQuizCommandError,
    ) -> Result<(), MusicQuizCommandError> {
        self.response_channel_id
            .send_message(
                &self.serenity_ctx.http,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .title("❌ Music Quiz Failed")
                        .description(format!("An error occurred: {}", error))
                        .color(0xe74c3c),
                ),
            )
            .await
            .map_err(MusicQuizCommandError::SendingMessageFailed)?;

        Ok(())
    }

    async fn play_track(&self, preview_url: &str) -> Result<(), MusicQuizCommandError> {
        let songbird = songbird::get(&self.serenity_ctx)
            .await
            .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

        let handler_lock = songbird
            .get(self.guild_id)
            .ok_or(MusicQuizCommandError::SongbirdCallDoesNotExist)?;

        let mut handler = handler_lock.lock().await;
        let client = Client::new();
        let source = HttpRequest::new(client, preview_url.to_string());

        handler.play_only_input(source.into());

        Ok(())
    }

    async fn stop_audio(&self) -> Result<(), MusicQuizCommandError> {
        let songbird = songbird::get(&self.serenity_ctx)
            .await
            .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

        let handler_lock = songbird
            .get(self.guild_id)
            .ok_or(MusicQuizCommandError::SongbirdCallDoesNotExist)?;

        let mut handler = handler_lock.lock().await;
        handler.stop();

        Ok(())
    }

    async fn leave_voice_channel(&self) -> Result<(), MusicQuizCommandError> {
        let songbird = songbird::get(&self.serenity_ctx)
            .await
            .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

        songbird
            .leave(self.guild_id)
            .await
            .map_err(MusicQuizCommandError::CouldNotLeaveChannel)?;

        Ok(())
    }
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
    if participants.is_empty() {
        return Err(MusicQuizCommandError::ToFewUsersInChannel);
    }

    let total_rounds = total_rounds.unwrap_or(TOTAL_ROUNDS_DEFAULT);
    let quiz = MusicQuiz::new(
        total_rounds,
        participants,
        Arc::clone(&ctx.data().itunes),
        Arc::clone(&ctx.data().spotify),
    );

    let tracks = quiz
        .fetch_random_tracks_from_playlist(spotify_playlist, total_rounds)
        .await?;

    Ok(PreparedMusicQuiz {
        quiz,
        voice_channel_id,
        tracks,
    })
}

fn get_user_voice_channel(
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
        .ok_or(MusicQuizCommandError::UserHasNoChannelID)
}

fn get_voice_channel_participants(
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

async fn wait_for_track_finished_or_timeout(quiz: &Arc<Mutex<MusicQuiz>>) {
    let notify = {
        let locked_quiz = quiz.lock().await;
        locked_quiz.notify_round_complete()
    };

    tokio::select! {
        _ = notify.notified() => {},
        _ = tokio::time::sleep(Duration::from_secs(25)) => {},
    }
}

async fn delay_between_rounds(quiz: &Arc<Mutex<MusicQuiz>>) {
    let is_finished = {
        let locked_quiz = quiz.lock().await;
        locked_quiz.session().is_finished()
    };

    if !is_finished {
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
