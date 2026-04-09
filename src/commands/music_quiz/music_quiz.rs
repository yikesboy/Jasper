use crate::app::music_quiz::start_from_command;
use crate::{Context, Error};

#[poise::command(slash_command, guild_only)]
pub async fn music_quiz(
    ctx: Context<'_>,
    #[description = "Public Spotify Playlist URL"] playlist: String,
    #[description = "Number of rounds (Defaults to 5)"] total_rounds: Option<u32>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    start_from_command(ctx, playlist, total_rounds).await?;
    Ok(())
}
