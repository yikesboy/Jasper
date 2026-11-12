pub enum EnvVar {
    Environment,
    SpotifyClientId,
    SpotifyClientSecret,
    SpotifyRefreshToken,
    DiscordToken,
    DiscordTestingGuildId,
}

impl EnvVar {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Environment => "ENVIRONMENT",
            Self::SpotifyClientId => "SPOTIFY_CLIENT_ID",
            Self::SpotifyClientSecret => "SPOTIFY_CLIENT_SECRET",
            Self::SpotifyRefreshToken => "SPOTIFY_REFRESH_TOKEN",
            Self::DiscordToken => "DISCORD_TOKEN",
            Self::DiscordTestingGuildId => "DISCORD_TESTING_GUILD_ID",
        }
    }
}
