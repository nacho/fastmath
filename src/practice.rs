use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::G_LOG_DOMAIN;
use crate::choice::Choice;
use crate::generator::{Generator, Question};
use crate::level::Level;
use crate::operation::{Operation, OperationSet};

/// How many answers of each fact are kept.
const KEPT_ANSWERS: usize = 5;
/// How much each kept answer counts in the weakness of a fact, the newest
/// first.
const WEIGHTS: [f64; KEPT_ANSWERS] = [1.0, 0.6, 0.36, 0.22, 0.13];
/// How much a wrong answer and a slow right answer count, from 0 to 1.
const WRONG: f64 = 1.0;
const SLOW: f64 = 0.5;
/// A right answer is slow when it takes this many times the average of the
/// right answers of its operation.
const SLOW_FACTOR: f64 = 1.5;
/// The weakness from which a fact is practised.
const WEAK: f64 = 0.2;
/// The share of practice questions that are weak facts, when there are any.
const WEAK_SHARE: f64 = 0.6;

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

    /// The question of the fact, with the operands in their kept order.
    fn question(&self) -> Question {
        let answer = match self.op {
            Operation::Add => self.a + self.b,
            Operation::Subtract => self.a - self.b,
            Operation::Multiply => self.a * self.b,
            Operation::Divide => self.a / self.b,
        };
        Question {
            op: self.op,
            a: self.a,
            b: self.b,
            answer,
        }
    }

    /// Parses the output of `id`. Facts that cannot be asked, such as a
    /// division by 0, are rejected.
    fn from_id(id: &str) -> Option<Self> {
        let mut parts = id.split('-');
        let letter = parts.next()?;
        let op = Operation::ALL
            .into_iter()
            .find(|op| letter.len() == 1 && letter.starts_with(op.id()))?;
        let a = parts.next()?.parse().ok()?;
        let b = parts.next()?.parse().ok()?;
        let valid = match op {
            Operation::Add | Operation::Multiply => a <= b,
            Operation::Subtract => b <= a,
            Operation::Divide => b > 0 && a % b == 0,
        };
        if parts.next().is_some() || !valid {
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

/// How the player does with one fact.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FactStats {
    /// The kept answers, and how many of them were right.
    pub answered: u32,
    pub right: u32,
    /// The average time of the right answers.
    pub average_ms: Option<u64>,
    /// From 0 (strong) to 1 (weak).
    pub weakness: f64,
}

/// The colour step of a weakness, from 0 (strong) to 4 (weak).
pub fn weakness_step(weakness: f64) -> u8 {
    match weakness {
        w if w < 0.1 => 0,
        w if w < WEAK => 1,
        w if w < 0.4 => 2,
        w if w < 0.7 => 3,
        _ => 4,
    }
}

/// How the player does with all the facts of an operation.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct OperationStats {
    /// The kept answers, and how many of them were right.
    pub answered: u32,
    pub right: u32,
    /// The average time of the right answers.
    pub average_ms: Option<u64>,
    /// How many facts are weak enough to practise.
    pub weak: u32,
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

    /// Deletes the history, so practice starts again from random questions.
    pub fn forget() {
        Self::forget_at(&Self::file_path());
    }

    pub fn forget_at(path: &Path) {
        match fs::remove_file(path) {
            Ok(()) => glib::debug!("Forgot the practice history"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => glib::warn!("Could not delete {}: {e}", path.display()),
        }
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

    /// The average time of the right answers of `op`, in milliseconds.
    fn average_ms(&self, op: Operation) -> Option<f64> {
        let times: Vec<u64> = self
            .facts
            .iter()
            .filter(|(key, _)| key.op == op)
            .flat_map(|(_, answers)| answers.iter())
            .filter(|answer| answer.correct)
            .map(|answer| answer.ms)
            .collect();
        (!times.is_empty()).then(|| times.iter().sum::<u64>() as f64 / times.len() as f64)
    }

    /// How weak a fact is, from 0 (its recent answers were right and fast)
    /// to 1 (they were all wrong). A wrong answer counts the most and a
    /// right one slower than `SLOW_FACTOR` times the average of its
    /// operation half of that. Recent answers count more.
    fn weakness(&self, key: &FactKey) -> f64 {
        let Some(answers) = self.facts.get(key) else {
            return 0.0;
        };
        let slow_ms = self.average_ms(key.op).map(|average| average * SLOW_FACTOR);
        let (sum, total) =
            answers
                .iter()
                .rev()
                .zip(WEIGHTS)
                .fold((0.0, 0.0), |(sum, total), (answer, weight)| {
                    let penalty = if !answer.correct {
                        WRONG
                    } else if slow_ms.is_some_and(|slow| answer.ms as f64 > slow) {
                        SLOW
                    } else {
                        0.0
                    };
                    (sum + weight * penalty, total + weight)
                });
        if total > 0.0 { sum / total } else { 0.0 }
    }

    /// The weak facts that `generator` could ask, with their weakness.
    fn weak_facts(&self, generator: &Generator) -> Vec<(FactKey, f64)> {
        let mut weak: Vec<(FactKey, f64)> = self
            .facts
            .keys()
            .filter(|key| generator.could_ask(&key.question()))
            .map(|key| (*key, self.weakness(key)))
            .filter(|(_, weakness)| *weakness >= WEAK)
            .collect();
        // The order of a HashMap changes on every run.
        weak.sort_by_key(|(key, _)| key.id());
        weak
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// How the player does with the fact of `question`, if it was answered.
    pub fn fact_stats(&self, question: &Question) -> Option<FactStats> {
        let key = FactKey::of(question);
        let answers = self.facts.get(&key)?;
        let times: Vec<u64> = answers
            .iter()
            .filter(|answer| answer.correct)
            .map(|answer| answer.ms)
            .collect();
        Some(FactStats {
            answered: answers.len() as u32,
            right: times.len() as u32,
            average_ms: (!times.is_empty()).then(|| times.iter().sum::<u64>() / times.len() as u64),
            weakness: self.weakness(&key),
        })
    }

    pub fn operation_stats(&self, op: Operation) -> OperationStats {
        let facts: Vec<(&FactKey, &Vec<AnswerRecord>)> =
            self.facts.iter().filter(|(key, _)| key.op == op).collect();
        let answers = facts.iter().flat_map(|(_, answers)| answers.iter());
        OperationStats {
            answered: answers.clone().count() as u32,
            right: answers.filter(|answer| answer.correct).count() as u32,
            average_ms: self.average_ms(op).map(|ms| ms.round() as u64),
            weak: facts
                .iter()
                .filter(|(key, _)| self.weakness(key) >= WEAK)
                .count() as u32,
        }
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

/// Produces practice questions: about `WEAK_SHARE` of them are weak facts,
/// the weaker the more often, and the rest new random questions.
pub struct PracticeGenerator {
    generator: Generator,
    weak: Vec<(FactKey, f64)>,
    rng: fastrand::Rng,
    last: Option<FactKey>,
}

impl PracticeGenerator {
    pub fn new(ops: OperationSet, level: Level, history: &FactHistory) -> Self {
        Self::with_rng(Generator::new(ops, level), history, fastrand::Rng::new())
    }

    pub fn with_rng(generator: Generator, history: &FactHistory, rng: fastrand::Rng) -> Self {
        PracticeGenerator {
            weak: history.weak_facts(&generator),
            generator,
            rng,
            last: None,
        }
    }

    /// Returns the next question, never the same fact as the previous one.
    pub fn next_question(&mut self) -> Question {
        loop {
            let question = if !self.weak.is_empty() && self.rng.f64() < WEAK_SHARE {
                self.weak_question()
            } else {
                self.generator.next_question()
            };
            let key = FactKey::of(&question);
            if self.last != Some(key) {
                self.last = Some(key);
                return question;
            }
        }
    }

    /// A weak fact, picked by weakness, with the operands of a sum or a
    /// product in either order.
    fn weak_question(&mut self) -> Question {
        let total: f64 = self.weak.iter().map(|(_, weakness)| weakness).sum();
        let mut target = self.rng.f64() * total;
        let key = self
            .weak
            .iter()
            .find(|(_, weakness)| {
                target -= weakness;
                target < 0.0
            })
            .or(self.weak.last())
            .map(|(key, _)| *key)
            .expect("weak facts are not empty");
        let mut question = key.question();
        if matches!(question.op, Operation::Add | Operation::Multiply) && self.rng.bool() {
            std::mem::swap(&mut question.a, &mut question.b);
        }
        question
    }

    /// Practises `question` as a weak fact for the rest of the round.
    pub fn missed(&mut self, question: &Question) {
        let key = FactKey::of(question);
        match self.weak.iter_mut().find(|(k, _)| *k == key) {
            Some((_, weakness)) => *weakness = WRONG,
            None => self.weak.push((key, WRONG)),
        }
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
            let key = FactKey::of(&q(op, 12, 3));
            assert_eq!(FactKey::from_id(&key.id()), Some(key));
        }
        assert_eq!(FactKey::of(&q(Operation::Multiply, 8, 7)).id(), "m-7-8");
        for bad in [
            "", "x-1-2", "m-1", "m-1-2-3", "m-a-2", "mm-1-2", "m-8-7", "s-5-9", "d-7-0", "d-7-2",
        ] {
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

        // Forgetting deletes the file, and a missing file is fine.
        history.save_to(&path);
        FactHistory::forget_at(&path);
        assert!(!path.exists());
        assert_eq!(FactHistory::load_from(&path), FactHistory::default());
        FactHistory::forget_at(&path);
    }

    #[test]
    fn wrong_answers_are_weaker_than_slow_ones_and_slow_than_fast() {
        let mut history = FactHistory::default();
        let (wrong, slow, fast) = (
            q(Operation::Multiply, 7, 8),
            q(Operation::Multiply, 6, 9),
            q(Operation::Multiply, 3, 4),
        );
        history.record(&wrong, false, ms(2000));
        history.record(&slow, true, ms(6000));
        for _ in 0..4 {
            history.record(&fast, true, ms(1000));
        }
        // The average of the right answers of × is 2000 ms, the wrong one
        // does not count, so 6000 ms is slow.
        assert_eq!(history.average_ms(Operation::Multiply), Some(2000.0));
        assert_eq!(history.average_ms(Operation::Add), None);
        let weakness = |question| history.weakness(&FactKey::of(question));
        assert_eq!(weakness(&wrong), WRONG);
        assert_eq!(weakness(&slow), SLOW);
        assert_eq!(weakness(&fast), 0.0);
        assert_eq!(weakness(&q(Operation::Multiply, 2, 2)), 0.0);
    }

    #[test]
    fn newer_answers_count_more() {
        let mut history = FactHistory::default();
        let (old_mistake, new_mistake) = (q(Operation::Add, 2, 3), q(Operation::Add, 4, 5));
        history.record(&old_mistake, false, ms(1000));
        history.record(&old_mistake, true, ms(1000));
        history.record(&new_mistake, true, ms(1000));
        history.record(&new_mistake, false, ms(1000));
        let weakness = |question| history.weakness(&FactKey::of(question));
        assert!(weakness(&new_mistake) > weakness(&old_mistake));

        // Getting a fact right and fast again makes it less weak, until it
        // is no longer practised.
        let mut history = FactHistory::default();
        let fact = q(Operation::Add, 4, 5);
        history.record(&fact, false, ms(1000));
        let mut previous = history.weakness(&FactKey::of(&fact));
        for _ in 0..4 {
            history.record(&fact, true, ms(1000));
            let now = history.weakness(&FactKey::of(&fact));
            assert!(now < previous, "{now} {previous}");
            previous = now;
        }
        assert!(previous < WEAK, "{previous}");
    }

    fn generator(ops: &str, level: u32, seed: u64) -> Generator {
        Generator::with_rng(
            OperationSet::from_id_string(ops),
            Level::new(level),
            fastrand::Rng::with_seed(seed),
        )
    }

    const SAMPLES: usize = 3000;

    #[test]
    fn without_history_practice_is_like_a_normal_round() {
        let mut practice = PracticeGenerator::with_rng(
            generator("asmd", 2, 1),
            &FactHistory::default(),
            fastrand::Rng::with_seed(2),
        );
        let mut normal = generator("asmd", 2, 1);
        for _ in 0..100 {
            assert_eq!(practice.next_question(), normal.next_question());
        }
    }

    #[test]
    fn weak_facts_come_more_often_the_weaker_they_are() {
        let mut history = FactHistory::default();
        let wrong = [(7, 8), (6, 7), (8, 9)].map(|(a, b)| q(Operation::Multiply, a, b));
        let slow = [(6, 9), (4, 7), (3, 8)].map(|(a, b)| q(Operation::Multiply, a, b));
        for fact in &wrong {
            history.record(fact, false, ms(1000));
        }
        for fact in &slow {
            history.record(fact, true, ms(9000));
        }
        for _ in 0..5 {
            history.record(&q(Operation::Multiply, 2, 3), true, ms(1000));
        }
        // Weak, but another operation or too large for level 2.
        history.record(&q(Operation::Add, 7, 8), false, ms(1000));
        history.record(&q(Operation::Multiply, 7, 12), false, ms(1000));

        let mut practice = PracticeGenerator::with_rng(
            generator("m", 2, 3),
            &history,
            fastrand::Rng::with_seed(4),
        );
        let questions: Vec<Question> = (0..SAMPLES).map(|_| practice.next_question()).collect();
        let count = |facts: &[Question]| {
            let keys: Vec<FactKey> = facts.iter().map(FactKey::of).collect();
            questions
                .iter()
                .filter(|q| keys.contains(&FactKey::of(q)))
                .count()
        };
        let share = (count(&wrong) + count(&slow)) as f64 / SAMPLES as f64;
        assert!((0.5..0.7).contains(&share), "{share}");
        assert!(
            count(&wrong) > count(&slow) * 3 / 2,
            "{} {}",
            count(&wrong),
            count(&slow)
        );
        assert!(
            questions
                .iter()
                .all(|q| q.op == Operation::Multiply && q.a <= 10 && q.b <= 10)
        );
        // Both orders of a product come up.
        assert!(questions.contains(&q(Operation::Multiply, 8, 7)));
        assert!(questions.contains(&q(Operation::Multiply, 7, 8)));
        // The same fact never comes twice in a row.
        assert!(
            questions
                .windows(2)
                .all(|w| FactKey::of(&w[0]) != FactKey::of(&w[1]))
        );
    }

    #[test]
    fn missed_facts_come_back() {
        let mut practice = PracticeGenerator::with_rng(
            generator("asmd", 3, 5),
            &FactHistory::default(),
            fastrand::Rng::with_seed(6),
        );
        let missed = practice.next_question();
        practice.missed(&missed);
        let back = (0..10).any(|_| FactKey::of(&practice.next_question()) == FactKey::of(&missed));
        assert!(back);
    }

    #[test]
    fn operation_stats_count_all_answers_and_time_the_right_ones() {
        let mut history = FactHistory::default();
        assert!(history.is_empty());
        history.record(&q(Operation::Multiply, 7, 8), false, ms(9000));
        history.record(&q(Operation::Multiply, 8, 7), true, ms(3000));
        history.record(&q(Operation::Multiply, 3, 4), true, ms(1000));
        history.record(&q(Operation::Multiply, 3, 4), true, ms(1000));
        history.record(&q(Operation::Add, 2, 2), true, ms(500));
        assert!(!history.is_empty());

        let stats = history.operation_stats(Operation::Multiply);
        assert_eq!(stats.answered, 4);
        assert_eq!(stats.right, 3);
        // The wrong answer does not count for the time.
        assert_eq!(stats.average_ms, Some(1667));
        // 7 × 8 has a wrong and a slow answer; 3 × 4 was right and fast.
        assert_eq!(stats.weak, 1);

        let empty = history.operation_stats(Operation::Divide);
        assert_eq!((empty.answered, empty.right, empty.weak), (0, 0, 0));
        assert_eq!(empty.average_ms, None);
    }

    #[test]
    fn fact_stats_of_answered_facts() {
        let mut history = FactHistory::default();
        history.record(&q(Operation::Multiply, 7, 8), false, ms(9000));
        history.record(&q(Operation::Multiply, 8, 7), true, ms(3000));
        history.record(&q(Operation::Multiply, 7, 8), true, ms(2000));
        let stats = history.fact_stats(&q(Operation::Multiply, 8, 7)).unwrap();
        assert_eq!((stats.answered, stats.right), (3, 2));
        assert_eq!(stats.average_ms, Some(2500));
        assert_eq!(
            stats.weakness,
            history.weakness(&FactKey::of(&q(Operation::Multiply, 7, 8)))
        );
        assert_eq!(history.fact_stats(&q(Operation::Multiply, 6, 9)), None);

        history.record(&q(Operation::Divide, 56, 7), false, ms(4000));
        let stats = history.fact_stats(&q(Operation::Divide, 56, 7)).unwrap();
        assert_eq!(stats.average_ms, None);
        assert_eq!(weakness_step(stats.weakness), 4);
    }

    #[test]
    fn weakness_steps() {
        let steps: Vec<u8> = [0.0, 0.09, 0.1, 0.19, 0.2, 0.39, 0.4, 0.69, 0.7, 1.0]
            .into_iter()
            .map(weakness_step)
            .collect();
        assert_eq!(steps, [0, 0, 1, 1, 2, 2, 3, 3, 4, 4]);
    }
}
