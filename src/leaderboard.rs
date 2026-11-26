use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use crate::G_LOG_DOMAIN;
use crate::round::{GameConfig, Mode};

pub const MAX_ENTRIES: usize = 10;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub name: String,
    pub correct: u32,
    /// The average time per answered question, in milliseconds.
    pub average_ms: u64,
}

impl LeaderboardEntry {
    /// More correct answers rank first, then a lower average time.
    fn cmp_rank(&self, other: &Self) -> Ordering {
        other
            .correct
            .cmp(&self.correct)
            .then(self.average_ms.cmp(&other.average_ms))
    }
}

/// The top scores for one combination of operations or comparisons, level
/// and duration.
#[derive(Default, Debug)]
pub struct Leaderboard {
    pub entries: Vec<LeaderboardEntry>,
}

impl Leaderboard {
    fn file_path(config: &GameConfig) -> PathBuf {
        // Operations are stored as the letters a, s, m and d, so "c" for
        // comparisons cannot clash with them.
        let questions = match config.mode {
            Mode::Arithmetic => config.ops.to_id_string(),
            Mode::Comparison => "c".to_string(),
        };
        glib::user_data_dir().join("fastmath").join(format!(
            "{}-{}-{}.json",
            questions,
            config.level.number(),
            config.duration_secs
        ))
    }

    pub fn load(config: &GameConfig) -> Self {
        Self::load_from(&Self::file_path(config))
    }

    pub fn save(&self, config: &GameConfig) {
        self.save_to(&Self::file_path(config));
    }

    /// Reads a leaderboard file. A missing or malformed file gives an empty
    /// leaderboard.
    pub fn load_from(path: &Path) -> Self {
        let mut entries: Vec<LeaderboardEntry> = fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        entries.sort_by(LeaderboardEntry::cmp_rank);
        entries.truncate(MAX_ENTRIES);
        Leaderboard { entries }
    }

    pub fn save_to(&self, path: &Path) {
        if let Some(dir) = path.parent()
            && let Err(e) = fs::create_dir_all(dir)
        {
            glib::warn!("Could not create {}: {e}", dir.display());
            return;
        }
        match serde_json::to_string_pretty(&self.entries) {
            Ok(json) => {
                if let Err(e) = fs::write(path, json) {
                    glib::warn!("Could not write {}: {e}", path.display());
                }
            }
            Err(e) => {
                glib::warn!("Could not serialize the leaderboard: {e}");
            }
        }
    }

    fn candidate(correct: u32, average_ms: u64) -> LeaderboardEntry {
        LeaderboardEntry {
            name: String::new(),
            correct,
            average_ms,
        }
    }

    /// Whether a score earns a place. Scoring nothing never does.
    pub fn qualifies(&self, correct: u32, average_ms: u64) -> bool {
        if correct == 0 {
            return false;
        }
        if self.entries.len() < MAX_ENTRIES {
            return true;
        }
        let candidate = Self::candidate(correct, average_ms);
        self.entries
            .last()
            .is_none_or(|worst| candidate.cmp_rank(worst) == Ordering::Less)
    }

    /// Inserts an entry and returns its position. Ties go below the
    /// existing entries.
    pub fn insert(&mut self, entry: LeaderboardEntry) -> usize {
        let rank = self
            .entries
            .iter()
            .position(|e| entry.cmp_rank(e) == Ordering::Less)
            .unwrap_or(self.entries.len());
        self.entries.insert(rank, entry);
        self.entries.truncate(MAX_ENTRIES);
        rank
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, correct: u32, average_ms: u64) -> LeaderboardEntry {
        LeaderboardEntry {
            name: name.to_string(),
            correct,
            average_ms,
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("fastmath-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn more_correct_answers_rank_first_then_faster_average() {
        let mut board = Leaderboard::default();
        assert_eq!(board.insert(entry("a", 20, 3000)), 0);
        assert_eq!(board.insert(entry("b", 30, 4000)), 0);
        assert_eq!(board.insert(entry("c", 20, 2000)), 1);
        assert_eq!(board.insert(entry("d", 5, 100)), 3);
        let names: Vec<&str> = board.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["b", "c", "a", "d"]);
    }

    #[test]
    fn ties_go_below_existing_entries() {
        let mut board = Leaderboard::default();
        board.insert(entry("first", 10, 1000));
        assert_eq!(board.insert(entry("second", 10, 1000)), 1);
    }

    #[test]
    fn only_ten_entries_are_kept() {
        let mut board = Leaderboard::default();
        for i in 0..15 {
            board.insert(entry("x", i + 1, 1000));
        }
        assert_eq!(board.entries.len(), MAX_ENTRIES);
        assert_eq!(board.entries.first().unwrap().correct, 15);
        assert_eq!(board.entries.last().unwrap().correct, 6);
    }

    #[test]
    fn qualification() {
        let mut board = Leaderboard::default();
        assert!(!board.qualifies(0, 0));
        assert!(board.qualifies(1, 9999));

        for i in 0..MAX_ENTRIES as u32 {
            board.insert(entry("x", 10 + i, 2000));
        }
        // The worst entry has 10 correct answers at 2000 ms.
        assert!(!board.qualifies(9, 1));
        assert!(!board.qualifies(10, 2000));
        assert!(!board.qualifies(10, 2500));
        assert!(board.qualifies(10, 1999));
        assert!(board.qualifies(11, 9999));
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_path("roundtrip.json");
        let mut board = Leaderboard::default();
        board.insert(entry("Ana", 12, 1500));
        board.insert(entry("Luis", 20, 2500));
        board.save_to(&path);

        let loaded = Leaderboard::load_from(&path);
        assert_eq!(loaded.entries, board.entries);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn a_missing_or_malformed_file_gives_an_empty_leaderboard() {
        let missing = temp_path("missing.json");
        assert!(Leaderboard::load_from(&missing).entries.is_empty());

        let malformed = temp_path("malformed.json");
        fs::write(&malformed, "{ not json").unwrap();
        assert!(Leaderboard::load_from(&malformed).entries.is_empty());
        let _ = fs::remove_file(&malformed);
    }
}
