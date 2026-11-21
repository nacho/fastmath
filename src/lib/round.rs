use std::time::Duration;

use super::generator::{Generator, Question};
use super::level::Level;
use super::operation::OperationSet;

/// The number of questions in a round.
pub const QUESTIONS_PER_ROUND: u32 = 60;

/// What the player chose to play.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GameConfig {
    pub ops: OperationSet,
    pub level: Level,
    pub duration_secs: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnswerResult {
    Correct,
    Wrong,
}

/// The outcome of a round.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RoundResult {
    pub correct: u32,
    pub wrong: u32,
    pub unanswered: u32,
    /// The average time spent on each answered question.
    pub average_time: Option<Duration>,
}

/// The state of one round: the current question and the score so far.
///
/// The round has no clock of its own. The caller passes the time elapsed
/// since the round started, and ends it with `finish` when time runs out.
pub struct Round {
    generator: Generator,
    current: Option<Question>,
    limit: u32,
    correct: u32,
    wrong: u32,
    answer_time: Duration,
    last_answer_at: Duration,
}

impl Round {
    pub fn new(config: &GameConfig) -> Self {
        Self::with_generator(
            Generator::new(config.ops, config.level),
            QUESTIONS_PER_ROUND,
        )
    }

    pub fn with_generator(mut generator: Generator, limit: u32) -> Self {
        let current = (limit > 0).then(|| generator.next_question());
        Round {
            generator,
            current,
            limit,
            correct: 0,
            wrong: 0,
            answer_time: Duration::ZERO,
            last_answer_at: Duration::ZERO,
        }
    }

    /// The question waiting for an answer, or `None` once the round is over.
    pub fn current(&self) -> Option<&Question> {
        self.current.as_ref()
    }

    pub fn is_finished(&self) -> bool {
        self.current.is_none()
    }

    pub fn answered(&self) -> u32 {
        self.correct + self.wrong
    }

    pub fn limit(&self) -> u32 {
        self.limit
    }

    /// Answers the current question `elapsed` after the round started, and
    /// moves on to the next one. Returns `None` if the round is over.
    pub fn submit(&mut self, answer: u32, elapsed: Duration) -> Option<AnswerResult> {
        let question = self.current?;

        self.answer_time += elapsed.saturating_sub(self.last_answer_at);
        self.last_answer_at = elapsed;

        let result = if answer == question.answer {
            self.correct += 1;
            AnswerResult::Correct
        } else {
            self.wrong += 1;
            AnswerResult::Wrong
        };

        self.current = (self.answered() < self.limit).then(|| self.generator.next_question());
        Some(result)
    }

    /// Ends the round early, for example when the time is up.
    pub fn finish(&mut self) {
        self.current = None;
    }

    pub fn result(&self) -> RoundResult {
        let answered = self.answered();
        RoundResult {
            correct: self.correct,
            wrong: self.wrong,
            unanswered: self.limit - answered,
            average_time: (answered > 0).then(|| self.answer_time / answered),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(limit: u32) -> Round {
        let generator = Generator::with_rng(
            OperationSet::ALL,
            Level::new(1),
            fastrand::Rng::with_seed(7),
        );
        Round::with_generator(generator, limit)
    }

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn answering_all_questions_ends_the_round() {
        let mut round = round(QUESTIONS_PER_ROUND);
        for i in 0..QUESTIONS_PER_ROUND {
            assert!(!round.is_finished());
            let answer = round.current().unwrap().answer;
            let result = round.submit(answer, secs(i as u64 + 1));
            assert_eq!(result, Some(AnswerResult::Correct));
        }
        assert!(round.is_finished());
        assert!(round.current().is_none());
        assert_eq!(round.submit(1, secs(100)), None);

        let result = round.result();
        assert_eq!(result.correct, QUESTIONS_PER_ROUND);
        assert_eq!(result.wrong, 0);
        assert_eq!(result.unanswered, 0);
        assert_eq!(result.average_time, Some(secs(1)));
    }

    #[test]
    fn wrong_answers_are_counted_and_the_round_moves_on() {
        let mut round = round(5);
        let first = *round.current().unwrap();
        let result = round.submit(first.answer + 1, secs(2));
        assert_eq!(result, Some(AnswerResult::Wrong));
        assert_ne!(round.current(), Some(&first));
        assert_eq!(round.result().wrong, 1);
        assert_eq!(round.result().correct, 0);
    }

    #[test]
    fn finishing_early_counts_the_rest_as_unanswered() {
        let mut round = round(QUESTIONS_PER_ROUND);
        for i in 0..10u64 {
            let answer = round.current().unwrap().answer;
            let answer = if i % 2 == 0 { answer } else { answer + 1 };
            round.submit(answer, secs(2 * (i + 1)));
        }
        round.finish();
        assert!(round.is_finished());

        let result = round.result();
        assert_eq!(result.correct, 5);
        assert_eq!(result.wrong, 5);
        assert_eq!(result.unanswered, QUESTIONS_PER_ROUND - 10);
        assert_eq!(
            result.correct + result.wrong + result.unanswered,
            QUESTIONS_PER_ROUND
        );
        // The ten answers took 20 s in total, the time after the last one
        // was spent on a question that was never answered.
        assert_eq!(result.average_time, Some(secs(2)));
    }

    #[test]
    fn a_round_without_answers_has_no_average_time() {
        let mut round = round(QUESTIONS_PER_ROUND);
        round.finish();
        let result = round.result();
        assert_eq!(result.unanswered, QUESTIONS_PER_ROUND);
        assert_eq!(result.average_time, None);
    }
}
