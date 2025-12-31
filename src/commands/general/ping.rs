use crate::{Context, Error};

#[poise::command(slash_command, prefix_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    Ok(())
}
