use crate::games::music_quiz::quiz::MusicQuiz;
use crate::{games, Error as AppError};
use dashmap::DashMap;
use serenity::all::{GuildId, Message};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

pub enum GameType {
    Quiz(Arc<Mutex<MusicQuiz>>),
}

impl GameType {
    pub async fn handle_message(
        &self,
        ctx: &serenity::all::Context,
        msg: &Message,
    ) -> Result<(), AppError> {
        match self {
            GameType::Quiz(quiz) => {
                games::music_quiz::message_handler::handle_message(ctx, msg, &quiz).await
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum GameStateError {
    #[error("Game is already active in this server.")]
    GameAlreadyActiveInServer,
}
pub struct GameState {
    pub games: DashMap<GuildId, GameType>,
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
        quiz: Arc<Mutex<MusicQuiz>>,
    ) -> Result<(), GameStateError> {
        if self.games.contains_key(&guild_id) {
            return Err(GameStateError::GameAlreadyActiveInServer);
        }
        self.games.insert(guild_id, GameType::Quiz(quiz));
        Ok(())
    }

    pub fn get_quiz(&self, guild_id: GuildId) -> Option<Arc<Mutex<MusicQuiz>>> {
        self.games.get(&guild_id).map(|game| match game.value() {
            GameType::Quiz(quiz) => Arc::clone(quiz),
        })
    }

    pub fn end_game(&self, guild_id: GuildId) {
        self.games.remove(&guild_id);
    }
}
