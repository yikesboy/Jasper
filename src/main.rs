use dotenvy::dotenv;
use poise::builtins::{register_globally, register_in_guild};
use poise::{Framework, FrameworkOptions};
use serenity::Client;
use serenity::all::{GatewayIntents, GuildId};
use songbird::SerenityInit;
use std::env;

pub struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
#[tokio::main]
async fn main() {
    dotenv().ok();

    let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN is missing");
    let is_dev = env::var("ENVIRONMENT")
        .expect("ENVIRONMENT is missing")
        .to_lowercase()
        == "dev";

    let framework: Framework<Data, Error> = create_framework(is_dev);
    let intents = GatewayIntents::non_privileged();
    let mut client = Client::builder(discord_token, intents)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Failed to create client");

    client.start().await.expect("Failed to start client");
}

#[poise::command(slash_command, prefix_command)]
async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

fn create_framework(is_dev: bool) -> Framework<Data, Error> {
    Framework::builder()
        .options(FrameworkOptions {
            commands: vec![ping()],
            ..Default::default()
        })
        .setup(move |context, _ready, framework| {
            let commands = &framework.options().commands;
            Box::pin(async move {
                if is_dev {
                    let testing_guild_id: u64 = env::var("TESTING_GUILD_ID")
                        .expect("TESTING_GUILD_ID is missing")
                        .parse()
                        .expect("TESTING_GUILD_ID has wrong format");
                    register_in_guild(context, commands, GuildId::new(testing_guild_id)).await?;
                } else {
                    register_globally(context, commands).await?;
                }
                Ok(Data {})
            })
        })
        .build()
}
