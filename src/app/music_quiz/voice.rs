use crate::commands::MusicQuizCommandError;
use reqwest::Client;
use serenity::all::{ChannelId, GuildId};
use serenity::client::Context as SerenityContext;
use songbird::input::HttpRequest;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;
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
        debug!(
            guild_id = self.guild_id.get(),
            voice_channel_id = voice_channel_id.get(),
            "Joining music quiz voice channel"
        );
        let songbird = songbird::get(&self.serenity_ctx)
            .await
            .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

        songbird
            .join(self.guild_id, voice_channel_id)
            .await
            .map_err(MusicQuizCommandError::FailedToJoinVoiceChannel)?;

        debug!(
            guild_id = self.guild_id.get(),
            voice_channel_id = voice_channel_id.get(),
            "Joined music quiz voice channel"
        );
        Ok(())
    }

    pub async fn play_preview(&self, preview_url: &Url) -> Result<(), MusicQuizCommandError> {
        debug!(guild_id = self.guild_id.get(), "Starting music quiz preview playback");
        let handler_lock = self.get_handler().await?;
        let mut handler = handler_lock.lock().await;
        let client = Client::new();
        let source = HttpRequest::new(client, preview_url.to_string());

        handler.play_only_input(source.into());
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), MusicQuizCommandError> {
        debug!(guild_id = self.guild_id.get(), "Stopping music quiz preview playback");
        let handler_lock = self.get_handler().await?;
        let mut handler = handler_lock.lock().await;
        handler.stop();
        Ok(())
    }

    pub async fn leave(&self) -> Result<(), MusicQuizCommandError> {
        debug!(guild_id = self.guild_id.get(), "Leaving music quiz voice channel");
        let songbird = songbird::get(&self.serenity_ctx)
            .await
            .ok_or(MusicQuizCommandError::SongbirdNotInitialized)?;

        songbird
            .leave(self.guild_id)
            .await
            .map_err(MusicQuizCommandError::CouldNotLeaveChannel)?;

        debug!(guild_id = self.guild_id.get(), "Left music quiz voice channel");
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
