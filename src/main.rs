mod services;

use dotenvy::dotenv;
use poise::builtins::{register_globally, register_in_guild};
use poise::{Framework, FrameworkOptions};
use serenity::Client;
use serenity::all::{GatewayIntents, GuildId};
use services::itunes::search_track;
use songbird::SerenityInit;
use songbird::input::HttpRequest;
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

#[poise::command(slash_command)]
pub async fn play_preview(
    ctx: Context<'_>,
    #[description = "Name of the song to preview"] query: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let track_info = search_track(query.as_str()).await;
    let track_info = match track_info {
        Ok(Some(track_info)) => track_info,
        Ok(None) => {
            ctx.say("Track not found").await?;
            return Ok(());
        }
        Err(e) => {
            ctx.say("Error occurred while search for the track.")
                .await?;
            return Ok(());
        }
    };

    let channel_id = {
        let guild = ctx.guild().unwrap();
        guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id)
    };

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.say("You need to be in a voice channel first!").await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client");

    let handler_lock = manager.join(ctx.guild_id().unwrap(), connect_to).await;
    let handler_lock = match handler_lock {
        Ok(handler) => handler,
        Err(e) => {
            ctx.say(format!("Failed to join channel: {:?}", e)).await?;
            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    let client = reqwest::Client::new();
    let source = HttpRequest::new(client, track_info.preview_url.to_string());

    handler.play_input(source.into());

    ctx.say(format!(
        "🎶 Playing preview for **{}**",
        track_info.track_name
    ))
    .await?;

    Ok(())
}

fn create_framework(is_dev: bool) -> Framework<Data, Error> {
    Framework::builder()
        .options(FrameworkOptions {
            commands: vec![ping(), play_preview()],
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
