use crate::games::music_quiz::quiz::{GuessOutcome, MusicQuiz};
use crate::Error;
use serenity::all::{Context, Message, ReactionType};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_message(
    ctx: &Context,
    msg: &Message,
    quiz_arc: &Arc<Mutex<MusicQuiz>>,
) -> Result<(), Error> {
    let outcome = {
        let mut quiz = quiz_arc.lock().await;

        match quiz.make_guess(msg.author.id, &msg.content) {
            Ok(outcome) => outcome,
            Err(_) => return Ok(()),
        }
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
