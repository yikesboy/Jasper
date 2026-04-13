use crate::commands::MusicQuizCommandError;
use reqwest::Client;
use serenity::all::{ChannelId, GuildId};
use serenity::client::Context as SerenityContext;
use songbird::input::HttpRequest;
use tokio::sync::Mutex;
use std::sync::Arc;
use url::Url;

pub struct MusicQuizVoice {
    serenity_ctx: SerenityContext,
    guild_id: GuildId,
}

impl MusicQuizVoice {
    pub fn new(serenity_ctx: SerenityContext, guild_id: GuildId) -> Self {
        Self {
            serenity_ctx,
            guild_id,
        }
    }

    pub async fn join(&self, voice_channel_id: ChannelId) -> Result<(), MusicQuizCommandError> {
        let songbird = songbird::get(&self.serenity_ctx)
            .await
            .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

        songbird
            .join(self.guild_id, voice_channel_id)
            .await
            .map_err(MusicQuizCommandError::FailedToJoinVoiceChannel)?;

        Ok(())
    }

    pub async fn play_preview(&self, preview_url: &Url) -> Result<(), MusicQuizCommandError> {
        let handler_lock = self.get_handler().await?;
        let mut handler = handler_lock.lock().await;
        let client = Client::new();
        let source = HttpRequest::new(client, preview_url.to_string());

        handler.play_only_input(source.into());
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), MusicQuizCommandError> {
        let handler_lock = self.get_handler().await?;
        let mut handler = handler_lock.lock().await;
        handler.stop();
        Ok(())
    }

    pub async fn leave(&self) -> Result<(), MusicQuizCommandError> {
        let songbird = songbird::get(&self.serenity_ctx)
            .await
            .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

        songbird
            .leave(self.guild_id)
            .await
            .map_err(MusicQuizCommandError::CouldNotLeaveChannel)?;

        Ok(())
    }

    async fn get_handler(
        &self,
    ) -> Result<Arc<Mutex<songbird::Call>>, MusicQuizCommandError> {
        let songbird = songbird::get(&self.serenity_ctx)
            .await
            .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

        songbird
            .get(self.guild_id)
            .ok_or(MusicQuizCommandError::SongbirdCallDoesNotExist)
    }
}
