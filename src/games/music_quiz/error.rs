use thiserror::Error;

#[derive(Error, Debug)]
pub enum MusicQuizError {
    #[error("There is no round in progress.")]
    NoRoundInProgress,

    #[error("This round is closed and no longer accepts guesses.")]
    RoundClosed,
}
