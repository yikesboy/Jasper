use crate::games::music_quiz::handle::MusicQuizHandle;
use crate::games::music_quiz::quiz::GuessOutcome;
use crate::Error;
use serenity::all::{Context, Message, ReactionType};

pub async fn handle_message(
    ctx: &Context,
    msg: &Message,
    quiz: &MusicQuizHandle,
) -> Result<(), Error> {
    let outcome = match quiz.guess(msg.author.id, &msg.content).await {
        Ok(outcome) => outcome,
        Err(_) => return Ok(()),
    };

    let reply_content = match outcome {
        GuessOutcome::Artist { points } => format!("🎤 Correct artist! +{} points!", points),
        GuessOutcome::Track { points } => format!("🎵 Correct track! +{} points!", points),
        GuessOutcome::Both { points } => {
            format!("🎉 Correct! You guessed both! +{} points!", points)
        }
        GuessOutcome::AlreadyGuessed => return Ok(()),
        GuessOutcome::Wrong => {
            react_to_message(ctx, msg, ReactionType::Unicode("❌".to_string())).await;
            return Ok(());
        }
    };

    msg.reply(&ctx.http, reply_content).await.ok();
    Ok(())
}

async fn react_to_message(ctx: &Context, msg: &Message, reaction: ReactionType) {
    msg.react(&ctx.http, reaction).await.ok();
}
