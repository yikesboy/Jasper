use crate::games::music_quiz::handle::MusicQuizHandle;
use crate::games::music_quiz::quiz::GuessOutcome;
use crate::Error;
use serenity::all::{Message, ReactionType};
use serenity::http::Http;
use tracing::{debug, warn};

pub async fn handle_message(
    http: &Http,
    msg: &Message,
    quiz: &MusicQuizHandle,
) -> Result<(), Error> {
    let outcome = match quiz.guess(msg.author.id, &msg.content).await {
        Ok(outcome) => outcome,
        Err(error) => {
            debug!(
                user_id = msg.author.id.get(),
                message_id = msg.id.get(),
                error = %error,
                "Ignoring music quiz guess because no round is currently active"
            );
            return Ok(());
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
            react_to_message(http, msg, ReactionType::Unicode("❌".to_string())).await;
            return Ok(());
        }
    };

    if let Err(error) = msg.reply(http, reply_content).await {
        warn!(
            user_id = msg.author.id.get(),
            message_id = msg.id.get(),
            error = %error,
            "Failed to send music quiz guess reply"
        );
    }
    Ok(())
}

async fn react_to_message(http: &Http, msg: &Message, reaction: ReactionType) {
    if let Err(error) = msg.react(http, reaction).await {
        warn!(
            user_id = msg.author.id.get(),
            message_id = msg.id.get(),
            error = %error,
            "Failed to react to incorrect music quiz guess"
        );
    }
}
