use crate::services::itunes::models::TrackInfo;
use dashmap::DashMap;
use serenity::all::UserId;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Round {
    pub track: TrackInfo,
    pub started_at: Instant,
    pub artist_guessed_by: Option<UserId>,
    pub track_guessed_by: Option<UserId>,
}

impl Round {
    pub fn new(track: TrackInfo) -> Self {
        Self {
            track,
            started_at: Instant::now(),
            artist_guessed_by: None,
            track_guessed_by: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MusicQuizSession {
    pub current_round: Option<Round>,
    pub scores: DashMap<UserId, u32>,
    pub round_number: u32,
    pub total_rounds: u32,
    pub participants: Vec<UserId>,
}

impl MusicQuizSession {
    pub fn new(total_rounds: u32, participants: Vec<UserId>) -> Self {
        Self {
            current_round: None,
            scores: DashMap::new(),
            round_number: 0,
            total_rounds,
            participants,
        }
    }

    pub fn start_round(&mut self, track: TrackInfo) {
        self.round_number += 1;
        self.current_round = Some(Round::new(track))
    }

    pub fn add_score(&self, user_id: UserId, points: u32) {
        self.scores
            .entry(user_id)
            .and_modify(|score| *score += points)
            .or_insert(points);
    }

    pub fn is_finished(&self) -> bool {
        self.round_number >= self.total_rounds
    }

    pub fn get_leaderboard(&self) -> Vec<(UserId, u32)> {
        let mut leaderboard: Vec<_> = self
            .participants
            .iter()
            .map(|&id| (id, *self.scores.get(&id).as_deref().unwrap_or(&0)))
            .collect();
        leaderboard.sort_by(|a, b| b.1.cmp(&a.1));
        leaderboard
    }
}
