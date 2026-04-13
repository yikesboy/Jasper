use crate::commands::MusicQuizCommandError;
use crate::games::music_quiz::MusicQuizHandle;
use serenity::all::{ChannelId, CreateEmbed, CreateMessage, UserId};
use serenity::client::Context as SerenityContext;

pub struct MusicQuizNotifier {
    serenity_ctx: SerenityContext,
    response_channel_id: ChannelId,
}

impl MusicQuizNotifier {
    pub fn new(serenity_ctx: SerenityContext, response_channel_id: ChannelId) -> Self {
        Self {
            serenity_ctx,
            response_channel_id,
        }
    }

    pub async fn send_round_start(
        &self,
        quiz: &MusicQuizHandle,
    ) -> Result<(), MusicQuizCommandError> {
        let progress = quiz.round_progress().await;

        self.response_channel_id
            .send_message(
                &self.serenity_ctx.http,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .title(format!(
                            "🎵 Round {}/{}",
                            progress.round_number, progress.total_rounds
                        ))
                        .description("Listen to the song and guess the artist and/or track name!")
                        .color(0x3498db),
                ),
            )
            .await
            .map_err(MusicQuizCommandError::SendingMessageFailed)?;

        Ok(())
    }

    pub async fn send_round_completion(
        &self,
        quiz: &MusicQuizHandle,
    ) -> Result<(), MusicQuizCommandError> {
        let round = match quiz.round_completion().await {
            Some(round) => round,
            None => return Ok(()),
        };

        let mut description = format!(
            "**Song:** {} by {}\n\n",
            round.track_name, round.artist_name
        );

        if let Some(artist_guesser) = round.artist_guessed_by {
            append_guesser_line(&mut description, "🎤 Artist guessed by", artist_guesser);
        }

        if let Some(track_guesser) = round.track_guessed_by {
            append_guesser_line(&mut description, "🎵 Track guessed by", track_guesser);
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

    pub async fn send_final_results(
        &self,
        quiz: &MusicQuizHandle,
    ) -> Result<(), MusicQuizCommandError> {
        let leaderboard = quiz.leaderboard().await;
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

    pub async fn send_failure(
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
}

fn append_guesser_line(description: &mut String, label: &str, user_id: UserId) {
    description.push_str(&format!("{} <@{}>\n", label, user_id));
}
