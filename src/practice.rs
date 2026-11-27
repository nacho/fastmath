use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::G_LOG_DOMAIN;
use crate::choice::Choice;
use crate::generator::Question;
use crate::operation::Operation;

/// How many answers of each fact are kept.
const KEPT_ANSWERS: usize = 5;

/// An arithmetic fact. 7 × 8 and 8 × 7 are the same fact, and so are 3 + 5
/// and 5 + 3, so the operands of additions and multiplications are kept in
/// order.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FactKey {
    op: Operation,
    a: u32,
    b: u32,
}

impl FactKey {
    pub fn of(question: &Question) -> Self {
        let (a, b) = match question.op {
            Operation::Add | Operation::Multiply => {
                (question.a.min(question.b), question.a.max(question.b))
            }
            Operation::Subtract | Operation::Divide => (question.a, question.b),
        };
        FactKey {
            op: question.op,
            a,
            b,
        }
    }

    /// The key as stored in the history file, for example `"m-7-8"`.
    fn id(&self) -> String {
        format!("{}-{}-{}", self.op.id(), self.a, self.b)
    }

    /// Parses the output of `id`.
    fn from_id(id: &str) -> Option<Self> {
        let mut parts = id.split('-');
        let letter = parts.next()?;
        let op = Operation::ALL
            .into_iter()
            .find(|op| letter.len() == 1 && letter.starts_with(op.id()))?;
        let a = parts.next()?.parse().ok()?;
        let b = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(FactKey { op, a, b })
    }
}

/// One answer to a fact.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AnswerRecord {
    pub correct: bool,
    /// How long the answer took, in milliseconds.
    pub ms: u64,
}

/// The last answers to every arithmetic fact the player has answered, from
/// every round.
#[derive(Default, Debug, PartialEq)]
pub struct FactHistory {
    /// The answers of each fact, the newest last.
    facts: HashMap<FactKey, Vec<AnswerRecord>>,
}

impl FactHistory {
    fn file_path() -> PathBuf {
        glib::user_data_dir().join("fastmath").join("facts.json")
    }

    pub fn load() -> Self {
        Self::load_from(&Self::file_path())
    }

    pub fn save(&self) {
        self.save_to(&Self::file_path());
    }

    /// Reads a history file. A missing or malformed file gives an empty
    /// history, and unknown facts are skipped.
    pub fn load_from(path: &Path) -> Self {
        let stored: BTreeMap<String, Vec<AnswerRecord>> = fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let facts = stored
            .into_iter()
            .filter_map(|(id, mut answers)| {
                let excess = answers.len().saturating_sub(KEPT_ANSWERS);
                answers.drain(..excess);
                Some((FactKey::from_id(&id)?, answers))
            })
            .collect();
        FactHistory { facts }
    }

    pub fn save_to(&self, path: &Path) {
        if let Some(dir) = path.parent()
            && let Err(e) = fs::create_dir_all(dir)
        {
            glib::warn!("Could not create {}: {e}", dir.display());
            return;
        }
        // Sorted, so the file changes as little as possible.
        let stored: BTreeMap<String, &Vec<AnswerRecord>> = self
            .facts
            .iter()
            .map(|(key, answers)| (key.id(), answers))
            .collect();
        match serde_json::to_string_pretty(&stored) {
            Ok(json) => {
                if let Err(e) = fs::write(path, json) {
                    glib::warn!("Could not write {}: {e}", path.display());
                }
            }
            Err(e) => {
                glib::warn!("Could not serialize the practice history: {e}");
            }
        }
    }

    /// Adds an answer to `question`, forgetting the oldest one of the fact
    /// once it has `KEPT_ANSWERS`.
    pub fn record(&mut self, question: &Question, correct: bool, time: Duration) {
        let answers = self.facts.entry(FactKey::of(question)).or_default();
        answers.push(AnswerRecord {
            correct,
            ms: time.as_millis() as u64,
        });
        let excess = answers.len().saturating_sub(KEPT_ANSWERS);
        answers.drain(..excess);
    }

    #[cfg(test)]
    fn answers(&self, question: &Question) -> &[AnswerRecord] {
        self.facts
            .get(&FactKey::of(question))
            .map_or(&[], Vec::as_slice)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.facts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(op: Operation, a: u32, b: u32) -> Question {
        let answer = match op {
            Operation::Add => a + b,
            Operation::Subtract => a - b,
            Operation::Multiply => a * b,
            Operation::Divide => a / b,
        };
        Question { op, a, b, answer }
    }

    fn ms(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn swapped_operands_are_the_same_fact_for_sums_and_products() {
        let key = |op, a, b| FactKey::of(&q(op, a, b));
        assert_eq!(
            key(Operation::Multiply, 8, 7),
            key(Operation::Multiply, 7, 8)
        );
        assert_eq!(key(Operation::Add, 3, 5), key(Operation::Add, 5, 3));
        assert_ne!(
            key(Operation::Subtract, 12, 5),
            key(Operation::Subtract, 12, 7)
        );
        assert_ne!(key(Operation::Divide, 12, 3), key(Operation::Divide, 12, 4));
        assert_ne!(key(Operation::Add, 7, 8), key(Operation::Multiply, 7, 8));
    }

    #[test]
    fn keys_round_trip() {
        for op in Operation::ALL {
            let key = FactKey { op, a: 12, b: 3 };
            assert_eq!(FactKey::from_id(&key.id()), Some(key));
        }
        assert_eq!(FactKey::of(&q(Operation::Multiply, 8, 7)).id(), "m-7-8");
        for bad in ["", "x-1-2", "m-1", "m-1-2-3", "m-a-2", "mm-1-2"] {
            assert_eq!(FactKey::from_id(bad), None, "{bad}");
        }
    }

    #[test]
    fn only_the_last_answers_are_kept() {
        let mut history = FactHistory::default();
        let fact = q(Operation::Multiply, 7, 8);
        for i in 0..8 {
            history.record(&fact, i % 2 == 0, ms(1000 + i));
        }
        let answers = history.answers(&fact);
        assert_eq!(answers.len(), KEPT_ANSWERS);
        assert_eq!(answers.first().unwrap().ms, 1003);
        assert_eq!(answers.last().unwrap().ms, 1007);
        // 8 × 7 adds to the same fact.
        history.record(&q(Operation::Multiply, 8, 7), true, ms(900));
        assert_eq!(history.answers(&fact).last().unwrap().ms, 900);
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn save_and_load_round_trip() {
        let path =
            std::env::temp_dir().join(format!("fastmath-test-{}-facts.json", std::process::id()));
        let mut history = FactHistory::default();
        history.record(&q(Operation::Multiply, 7, 8), false, ms(4200));
        history.record(&q(Operation::Subtract, 12, 5), true, ms(1500));
        history.save_to(&path);
        assert_eq!(FactHistory::load_from(&path), history);

        // Unknown facts are skipped and long lists are cut.
        fs::write(
            &path,
            r#"{"x-1-2": [], "a-1-2": [{"correct": true, "ms": 1}, {"correct": true, "ms": 2}, {"correct": true, "ms": 3}, {"correct": true, "ms": 4}, {"correct": true, "ms": 5}, {"correct": false, "ms": 6}]}"#,
        )
        .unwrap();
        let loaded = FactHistory::load_from(&path);
        assert_eq!(loaded.len(), 1);
        let answers = loaded.answers(&q(Operation::Add, 2, 1));
        assert_eq!(answers.len(), KEPT_ANSWERS);
        assert_eq!(answers[0].ms, 2);

        fs::write(&path, "not json").unwrap();
        assert_eq!(FactHistory::load_from(&path), FactHistory::default());
        let _ = fs::remove_file(&path);
        assert_eq!(FactHistory::load_from(&path), FactHistory::default());
    }
}
