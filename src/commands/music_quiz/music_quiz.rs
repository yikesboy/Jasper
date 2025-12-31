use crate::commands::music_quiz::error::MusicQuizCommandError;
use crate::games::music_quiz::quiz::MusicQuiz;
use crate::services::itunes::models::TrackInfo;
use crate::{Context, Error};
use reqwest::Client;
use serenity::all::{ChannelId, GuildId, UserId};
use serenity::all::{CreateEmbed, CreateMessage};
use songbird;
use songbird::input::HttpRequest;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;

const TOTAL_ROUNDS_DEFAULT: u32 = 5;

#[poise::command(slash_command, guild_only)]
pub async fn music_quiz(
    ctx: Context<'_>,
    #[description = "Public Spotify Playlist URL"] playlist: String,
    #[description = "Number of rounds (Defaults to 5)"] total_rounds: Option<u32>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let (quiz, channel_id, tracks) = prepare_music_quiz(ctx, playlist, total_rounds).await?;
    ctx.say("🎵 Starting Music Quiz! Joining voice channel...")
        .await
        .map_err(|e| Box::new(MusicQuizCommandError::ErrorCreatingResponse(e.to_string())))?;

    let guild_id = ctx.guild_id().expect("Should have GuildID");

    let songbird = songbird::get(ctx.serenity_context())
        .await
        .ok_or(Box::new(MusicQuizCommandError::SongbirdNotInitialized))?;

    songbird
        .join(guild_id, channel_id)
        .await
        .map_err(|_| Box::new(MusicQuizCommandError::FailedToJoinVoiceChannel))?;

    let quiz_arc = Arc::new(Mutex::new(quiz));

    ctx.data()
        .game_state
        .start_quiz(guild_id, quiz_arc.clone())
        .map_err(|e| Box::new(MusicQuizCommandError::GameStateError(e.to_string())))?;

    spawn_quiz_thread_and_run(ctx, guild_id, quiz_arc, tracks).await?;
    Ok(())
}

async fn run_quiz(
    ctx: serenity::all::Context,
    channel_id: &ChannelId,
    guild_id: GuildId,
    quiz_arc: Arc<Mutex<MusicQuiz>>,
    tracks: Vec<TrackInfo>,
) -> Result<(), MusicQuizCommandError> {
    for track in tracks {
        {
            let mut quiz = quiz_arc.lock().await;
            quiz.session_mut().start_round(track.clone());
        }

        let (round_number, total_rounds) = {
            let quiz = quiz_arc.lock().await;
            (quiz.session().round_number, quiz.session().total_rounds)
        };

        channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .title(format!("🎵 Round {}/{}", round_number, total_rounds))
                        .description("Listen to the song and guess the artist and/or track name!")
                        .color(0x3498db),
                ),
            )
            .await
            .map_err(|e| MusicQuizCommandError::SendingMessageFailed(e.to_string()))?;

        play_track(ctx.clone(), guild_id, &track.preview_url).await?;
        wait_for_track_finished_or_timeout(&quiz_arc).await;
        stop_audio(ctx.clone(), guild_id).await?;

        build_and_send_round_completion_msg(ctx.clone(), &channel_id.clone(), quiz_arc.clone())
            .await?;

        delay_between_rounds(&quiz_arc).await;
    }

    show_final_results(ctx.clone(), channel_id, &quiz_arc).await?;

    Ok(())
}

async fn prepare_music_quiz(
    ctx: Context<'_>,
    playlist: String,
    total_rounds: Option<u32>,
) -> Result<(MusicQuiz, ChannelId, Vec<TrackInfo>), Error> {
    let spotify_playlist =
        Url::parse(&playlist).map_err(|_| Box::new(MusicQuizCommandError::InvalidURL(playlist)))?;

    let total_rounds = total_rounds.unwrap_or(TOTAL_ROUNDS_DEFAULT);

    let guild_id = ctx
        .guild_id()
        .ok_or(Box::new(MusicQuizCommandError::MustBeUsedInGuild))?;

    if ctx.data().game_state.games.contains_key(&guild_id) {
        return Err(Box::new(MusicQuizCommandError::GameAlreadyRunningInGuild));
    }

    let channel_id = get_user_voice_channel(&ctx, guild_id, ctx.author().id).await?;

    let participants = get_voice_channel_participants(&ctx, guild_id, channel_id).await?;
    if participants.is_empty() {
        return Err(Box::new(MusicQuizCommandError::ToFewUsersInChannel));
    }

    let quiz = MusicQuiz::new(
        total_rounds,
        participants,
        ctx.data().itunes.clone(),
        ctx.data().spotify.clone(),
    );

    let tracks = quiz
        .fetch_random_tracks_from_playlist(spotify_playlist, total_rounds)
        .await
        .map_err(|e| MusicQuizCommandError::MusicQuizError(e.to_string()))?;

    Ok((quiz, channel_id, tracks))
}

async fn show_final_results(
    ctx: serenity::all::Context,
    channel_id: &ChannelId,
    quiz_arc: &Arc<Mutex<MusicQuiz>>,
) -> Result<(), MusicQuizCommandError> {
    let leaderboard = {
        let quiz = quiz_arc.lock().await;
        quiz.session().get_leaderboard()
    };

    let mut description = String::from("**Final Scores:**\n\n");

    for (index, (user_id, score)) in leaderboard.iter().enumerate() {
        let medal = match index {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        description.push_str(&format!("{} <@{}> - {} points\n", medal, user_id, score))
    }

    channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .title("🎉 Music Quiz Complete!")
                    .description(description)
                    .color(0xf39c12),
            ),
        )
        .await
        .map_err(|e| MusicQuizCommandError::SendingMessageFailed(e.to_string()))?;

    Ok(())
}

async fn get_user_voice_channel(
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

async fn get_voice_channel_participants(
    context: &Context<'_>,
    guild_id: GuildId,
    channel_id: ChannelId,
) -> Result<Vec<UserId>, MusicQuizCommandError> {
    let guild = guild_id
        .to_guild_cached(context.cache())
        .ok_or(MusicQuizCommandError::GuildNotCached)?;

    Ok(guild
        .voice_states
        .values()
        .filter(|vs| vs.channel_id == Some(channel_id))
        .map(|vs| vs.user_id)
        .filter(|u_id| {
            if let Some(member) = guild.members.get(u_id) {
                !member.user.bot
            } else {
                false
            }
        })
        .collect::<Vec<UserId>>())
}

async fn play_track(
    ctx: serenity::all::Context,
    guild_id: GuildId,
    preview_url: &str,
) -> Result<(), MusicQuizCommandError> {
    println!("Attempting to play track: {}", preview_url);

    let songbird = songbird::get(&ctx)
        .await
        .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

    let handler_lock = songbird
        .get(guild_id)
        .ok_or(MusicQuizCommandError::SongbirdCallDoesNotExist)?;

    let mut handler = handler_lock.lock().await;
    let client = Client::new();
    let source = HttpRequest::new(client, preview_url.to_string());

    handler.play_only_input(source.into());

    println!("Track started playing");

    Ok(())
}

async fn stop_audio(
    ctx: serenity::all::Context,
    guild_id: GuildId,
) -> Result<(), MusicQuizCommandError> {
    let songbird = songbird::get(&ctx)
        .await
        .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

    let handler_lock = songbird
        .get(guild_id)
        .ok_or(MusicQuizCommandError::SongbirdCallDoesNotExist)?;

    let mut handler = handler_lock.lock().await;
    handler.stop();

    Ok(())
}

async fn leave_voice_channel(
    ctx: &serenity::all::Context,
    guild_id: GuildId,
) -> Result<(), MusicQuizCommandError> {
    let songbird = songbird::get(&ctx)
        .await
        .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

    songbird
        .leave(guild_id)
        .await
        .map_err(|e| MusicQuizCommandError::CouldNotLeaveChannel(e.to_string()))?;

    Ok(())
}

async fn build_and_send_round_completion_msg(
    ctx: serenity::all::Context,
    channel_id: &ChannelId,
    quiz_arc: Arc<Mutex<MusicQuiz>>,
) -> Result<(), MusicQuizCommandError> {
    let quiz = quiz_arc.lock().await;

    let round = match &quiz.session().current_round {
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

    channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .title("Round Complete!")
                    .description(description)
                    .color(0x2ecc71),
            ),
        )
        .await
        .map_err(|e| MusicQuizCommandError::SendingMessageFailed(e.to_string()))?;

    Ok(())
}

async fn wait_for_track_finished_or_timeout(quiz_arc: &Arc<Mutex<MusicQuiz>>) {
    let notify = {
        let quiz = quiz_arc.lock().await;
        quiz.notify_round_complete()
    };

    tokio::select! {
        _ = notify.notified() => {},
        _ = tokio::time::sleep(Duration::from_secs(25)) => {},
    }
}

async fn spawn_quiz_thread_and_run(
    ctx: Context<'_>,
    guild_id: GuildId,
    quiz_arc: Arc<Mutex<MusicQuiz>>,
    tracks: Vec<TrackInfo>,
) -> Result<(), MusicQuizCommandError> {
    let serenity_ctx = ctx.serenity_context().clone();
    let channel_id = ctx.channel_id();
    let game_state = ctx.data().game_state.clone();

    tokio::spawn(async move {
        let result = run_quiz(
            serenity_ctx.clone(),
            &channel_id,
            guild_id,
            quiz_arc,
            tracks,
        )
        .await
        .map_err(|e| MusicQuizCommandError::FailedWhileRunningQuiz(e.to_string()));

        let _ = leave_voice_channel(&serenity_ctx, guild_id).await;
        game_state.end_game(guild_id);

        if let Err(e) = result {
            let _ = channel_id
                .send_message(
                    &serenity_ctx.http,
                    CreateMessage::new().embed(
                        CreateEmbed::new()
                            .title("❌ Music Quiz Failed")
                            .description(format!("An error occurred: {}", e))
                            .color(0xe74c3c),
                    ),
                )
                .await;
        }
    });

    Ok(())
}

async fn delay_between_rounds(quiz_arc: &Arc<Mutex<MusicQuiz>>) {
    let is_finished = {
        let quiz = quiz_arc.lock().await;
        quiz.session().is_finished()
    };

    if !is_finished {
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
