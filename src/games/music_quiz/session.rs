use crate::services::itunes::models::TrackInfo;
use serenity::all::UserId;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Round {
    track: TrackInfo,
    artist_guessed_by: Option<UserId>,
    track_guessed_by: Option<UserId>,
}

impl Round {
    pub fn new(track: TrackInfo) -> Self {
        Self {
            track,
            artist_guessed_by: None,
            track_guessed_by: None,
        }
    }

    pub fn track(&self) -> &TrackInfo {
        &self.track
    }

    pub fn artist_guessed_by(&self) -> Option<UserId> {
        self.artist_guessed_by
    }

    pub fn track_guessed_by(&self) -> Option<UserId> {
        self.track_guessed_by
    }

    pub fn set_artist_guessed_by(&mut self, user_id: UserId) {
        self.artist_guessed_by = Some(user_id);
    }

    pub fn set_track_guessed_by(&mut self, user_id: UserId) {
        self.track_guessed_by = Some(user_id);
    }

    pub fn is_complete(&self) -> bool {
        self.artist_guessed_by.is_some() && self.track_guessed_by.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct MusicQuizSession {
    current_round: Option<Round>,
    scores: HashMap<UserId, u32>,
    round_number: u32,
    total_rounds: u32,
    participants: Vec<UserId>,
}

impl MusicQuizSession {
    pub fn new(total_rounds: u32, participants: Vec<UserId>) -> Self {
        Self {
            current_round: None,
            scores: HashMap::new(),
            round_number: 0,
            total_rounds,
            participants,
        }
    }

    pub fn start_round(&mut self, track: TrackInfo) {
        self.round_number += 1;
        self.current_round = Some(Round::new(track))
    }

    pub fn current_round(&self) -> Option<&Round> {
        self.current_round.as_ref()
    }

    pub fn current_round_mut(&mut self) -> Option<&mut Round> {
        self.current_round.as_mut()
    }

    pub fn round_number(&self) -> u32 {
        self.round_number
    }

    pub fn total_rounds(&self) -> u32 {
        self.total_rounds
    }

    pub fn add_score(&mut self, user_id: UserId, points: u32) {
        *self.scores.entry(user_id).or_insert(0) += points;
    }

    pub fn is_finished(&self) -> bool {
        self.round_number >= self.total_rounds
    }

    pub fn get_leaderboard(&self) -> Vec<(UserId, u32)> {
        let mut leaderboard: Vec<_> = self
            .participants
            .iter()
            .map(|&id| (id, *self.scores.get(&id).unwrap_or(&0)))
            .collect();
        leaderboard.sort_by(|a, b| b.1.cmp(&a.1));
        leaderboard
    }
}
