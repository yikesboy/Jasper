mod general;
mod music_quiz;

use poise::Command;

use crate::{Data, Error};

pub fn get_commands() -> Vec<Command<Data, Error>> {
    vec![general::ping::ping(), music_quiz::music_quiz::music_quiz()]
}
