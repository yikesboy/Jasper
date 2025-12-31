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
}

impl MusicQuizSession {
    pub fn new(total_rounds: u32) -> Self {
        Self {
            current_round: None,
            scores: DashMap::new(),
            round_number: 0,
            total_rounds,
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
        let mut scores: Vec<_> = self
            .scores
            .iter()
            .map(|entry| (*entry.key(), *entry.value()))
            .collect();
        scores.sort_by(|a, b| b.1.cmp(&a.1));
        scores
    }
}
