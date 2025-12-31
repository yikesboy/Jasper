use crate::games::state::GameState;
use serenity::all::{Context, Message};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageEventError {
    #[error("Game handler error: {0}")]
    GameHandlerError(String),
}

pub async fn handle_message(
    ctx: &Context,
    msg: &Message,
    game_state: Arc<GameState>,
) -> Result<(), MessageEventError> {
    if msg.author.bot {
        return Ok(());
    }

    let guild_id = match msg.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    if let Some(game) = game_state.games.get(&guild_id) {
        game.value()
            .handle_message(ctx, msg)
            .await
            .map_err(|e| MessageEventError::GameHandlerError(e.to_string()))?;
    }

    Ok(())
}
