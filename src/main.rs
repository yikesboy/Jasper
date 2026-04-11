mod app;
mod commands;
mod config;
mod events;
mod games;
mod services;

use crate::commands::MusicQuizCommandError;
use crate::config::{Config, ConfigError};
use crate::events::{MessageEventError, handle_message};
use crate::games::state::GameState;
use crate::services::itunes::ItunesClient;
use crate::services::spotify::{SpotifyClient, SpotifyClientError};
use dotenvy::dotenv;
use poise::builtins::{register_globally, register_in_guild};
use poise::{Framework, FrameworkOptions};
use serenity::Client;
use serenity::all::{FullEvent, GatewayIntents, GuildId};
use songbird::SerenityInit;
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone)]
pub struct Data {
    pub game_state: Arc<GameState>,
    pub itunes: Arc<ItunesClient>,
    pub spotify: Arc<SpotifyClient>,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Serenity(#[from] serenity::Error),

    #[error(transparent)]
    Spotify(#[from] SpotifyClientError),

    #[error(transparent)]
    MusicQuizCommand(#[from] MusicQuizCommandError),

    #[error(transparent)]
    MessageEvent(#[from] MessageEventError),
}

type Error = AppError;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();

    let config = Config::from_env()?;

    let framework = create_framework(config.clone());
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&config.discord_token, intents)
        .framework(framework)
        .register_songbird()
        .await?;

    client.start().await?;
    Ok(())
}

fn create_framework(config: Config) -> Framework<Data, Error> {
    Framework::builder()
        .options(FrameworkOptions {
            commands: commands::get_commands(),
            event_handler: |ctx, event, _framework, data| {
                Box::pin(async move { handle_discord_event(ctx, event, data).await })
            },
            ..Default::default()
        })
        .setup(move |context, _ready, framework| {
            let config = config.clone();
            let commands = &framework.options().commands;
            Box::pin(async move {
                register_commands(context, &config, commands).await?;

                let spotify = SpotifyClient::new(
                    &config.spotify_client_id,
                    &config.spotify_client_secret,
                    None,
                )
                .await?;

                Ok(Data {
                    game_state: Arc::new(GameState::new()),
                    itunes: Arc::new(ItunesClient::new(None)),
                    spotify: Arc::new(spotify),
                })
            })
        })
        .build()
}

async fn register_commands(
    context: &serenity::all::Context,
    config: &Config,
    commands: &[poise::Command<Data, Error>],
) -> Result<(), Error> {
    if config.is_dev() {
        let guild_id = config
            .discord_testing_guild_id
            .ok_or_else(|| ConfigError::MissingEnvironmentVariable("DISCORD_TESTING_GUILD_ID".to_string()))?;

        register_in_guild(context, commands, GuildId::new(guild_id)).await?;
    } else {
        register_globally(context, commands).await?;
    }

    Ok(())
}

async fn handle_discord_event(
    ctx: &serenity::all::Context,
    event: &FullEvent,
    data: &Data,
) -> Result<(), Error> {
    if let FullEvent::Message { new_message } = event {
        handle_message(ctx, new_message, Arc::clone(&data.game_state)).await?;
    }

    Ok(())
}
