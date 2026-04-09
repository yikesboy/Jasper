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
    Development,
    Production,
}
impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = match Self::read_any_env_var(&[EnvVar::Environment.key()])?
            .to_ascii_lowercase()
            .as_str()
        {
            "dev" | "development" => Environment::Development,
            "prod" | "production" => Environment::Production,
            value => return Err(ConfigError::InvalidEnvironment(value.to_string())),
        };

        let discord_testing_guild_id = if environment == Environment::Development {
            let guild_id =
                Self::read_any_env_var(&[EnvVar::DiscordTestingGuildId.key(), "TESTING_GUILD_ID"])?;
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
        self.environment.eq(&Environment::Development)
    }

    fn read_in_env_var(env_var: EnvVar) -> Result<String, ConfigError> {
        env::var(env_var.key())
            .map_err(|_| ConfigError::MissingEnvironmentVariable(env_var.key().to_string()))
    }

    fn read_any_env_var(keys: &[&str]) -> Result<String, ConfigError> {
        keys.iter()
            .find_map(|key| env::var(key).ok())
            .ok_or_else(|| ConfigError::MissingEnvironmentVariable(keys.join(" or ")))
    }
}
