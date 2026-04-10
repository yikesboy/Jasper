use crate::games::state::GameState;
use crate::AppError;
use serenity::all::{Context, Message};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageEventError {
    #[error("Game handler error")]
    GameHandlerError(#[source] Box<AppError>),
}

pub async fn handle_message(
    ctx: &Context,
    msg: &Message,
    game_state: Arc<GameState>,
) -> Result<(), MessageEventError> {
    if msg.author.bot {
        return Ok(());
    }

    match msg.guild_id {
        Some(_) => {}
        None => return Ok(()),
    }

    game_state
        .handle_message(ctx, msg)
        .await
        .map_err(|error| MessageEventError::GameHandlerError(Box::new(error)))?;

    Ok(())
}
