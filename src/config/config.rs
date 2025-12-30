use crate::config::envvar::EnvVar;
use crate::config::error::ConfigError;
use std::env;

#[derive(Clone)]
pub struct Config {
    pub environment: Environment,
    pub spotify_client_id: String,
    pub spotify_client_secret: String,
    pub discord_token: String,
    pub discord_testing_guild_id: Option<u64>,
}

#[derive(Clone, PartialEq)]
pub enum Environment {
    DEV,
    PROD,
}
impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = match Self::read_in_env_var(EnvVar::Environment)?.as_str() {
            "development" => Environment::DEV,
            _ => Environment::PROD,
        };

        let discord_testing_guild_id = if environment == Environment::DEV {
            let guild_id = Self::read_in_env_var(EnvVar::DiscordTestingGuildId)?;
            Some(
                guild_id
                    .parse()
                    .map_err(|_| ConfigError::FailedToParse(guild_id))?,
            )
        } else {
            None
        };

        Ok(Self {
            environment,
            spotify_client_id: Self::read_in_env_var(EnvVar::SpotifyClientId)?,
            spotify_client_secret: Self::read_in_env_var(EnvVar::SpotifyClientSecret)?,
            discord_token: Self::read_in_env_var(EnvVar::DiscordToken)?,
            discord_testing_guild_id,
        })
    }

    pub fn is_dev(&self) -> bool {
        self.environment.eq(&Environment::DEV)
    }

    fn read_in_env_var(env_var: EnvVar) -> Result<String, ConfigError> {
        env::var(env_var.key())
            .map_err(|_| ConfigError::MissingEnvironmentVariable(env_var.key().to_string()))
    }
}
