use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvironmentVariable(String),

    #[error("Failed to parse: {0}")]
    FailedToParse(String),

    #[error("Invalid environment: {0}")]
    InvalidEnvironment(String),
}
