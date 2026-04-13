use crate::games::music_quiz::MusicQuizHandle;
use crate::{games, Error as AppError};
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use serenity::all::{GuildId, Message};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GameStateError {
    #[error("Game is already active in this server.")]
    GameAlreadyActiveInServer,
}

pub struct GameState {
    games: DashMap<GuildId, MusicQuizHandle>,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            games: DashMap::new(),
        }
    }

    pub fn start_quiz(
        &self,
        guild_id: GuildId,
        quiz: MusicQuizHandle,
    ) -> Result<(), GameStateError> {
        match self.games.entry(guild_id) {
            Entry::Occupied(_) => Err(GameStateError::GameAlreadyActiveInServer),
            Entry::Vacant(entry) => {
                entry.insert(quiz);
                Ok(())
            }
        }
    }

    pub async fn handle_message(
        &self,
        ctx: &serenity::all::Context,
        msg: &Message,
    ) -> Result<(), AppError> {
        let Some(guild_id) = msg.guild_id else {
            return Ok(());
        };

        if let Some(quiz) = self.games.get(&guild_id) {
            games::music_quiz::message_handler::handle_message(ctx, msg, quiz.value()).await?;
        }

        Ok(())
    }

    pub fn end_game(&self, guild_id: GuildId) {
        self.games.remove(&guild_id);
    }
}
