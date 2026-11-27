use std::cmp::Ordering;
use std::time::Duration;

use crate::comparison::{Comparison, ComparisonGenerator, ComparisonKinds, Part};
use crate::fraction::{FractionGenerator, FractionQuestion};
use crate::generator::{Generator, Question};
use crate::level::Level;
use crate::operation::OperationSet;
use crate::practice::{FactHistory, PracticeGenerator};

/// The number of questions in a round.
pub const QUESTIONS_PER_ROUND: u32 = 60;
/// The number of questions in a practice round, which has no clock.
pub const PRACTICE_QUESTIONS: u32 = 20;

/// The kind of questions in a round.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mode {
    /// Calculations with the selected operations.
    Arithmetic,
    /// Whether a number is less than, equal to or greater than another.
    Comparison,
    /// Which fraction of a picture was eaten or is left.
    Fractions,
}

impl Mode {
    /// The name used to store the mode in settings.
    pub fn id(self) -> &'static str {
        match self {
            Mode::Arithmetic => "arithmetic",
            Mode::Comparison => "comparison",
            Mode::Fractions => "fractions",
        }
    }

    /// Parses the output of `id`. Anything else is arithmetic.
    pub fn from_id(id: &str) -> Self {
        [Mode::Comparison, Mode::Fractions]
            .into_iter()
            .find(|mode| mode.id() == id)
            .unwrap_or(Mode::Arithmetic)
    }
}

/// What the player chose to play.
#[derive(Clone, Copy, PartialEq, Eq, Debug, glib::Boxed)]
#[boxed_type(name = "FastmathGameConfig", nullable)]
pub struct GameConfig {
    pub mode: Mode,
    /// The operations of an arithmetic round.
    pub ops: OperationSet,
    /// What a comparison round compares.
    pub kinds: ComparisonKinds,
    /// Whether an arithmetic round practises the weak facts, with no clock.
    pub practice: bool,
    pub level: Level,
    pub duration_secs: u32,
}

impl GameConfig {
    /// Whether the round practises weak facts, with no clock and no
    /// leaderboard. Only arithmetic rounds can.
    pub fn is_practice(&self) -> bool {
        self.practice && self.mode == Mode::Arithmetic
    }
}

/// What the player answers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Answer {
    Number(u32),
    Order(Ordering),
    /// A numerator and a denominator.
    Fraction(u32, u32),
}

/// A question of either mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prompt {
    Arithmetic(Question),
    Comparison(Comparison),
    Fraction(FractionQuestion),
}

impl Prompt {
    /// The answer, in its simplest form for fractions.
    pub fn answer(&self) -> Answer {
        match self {
            Prompt::Arithmetic(q) => Answer::Number(q.answer),
            Prompt::Comparison(c) => Answer::Order(c.answer()),
            Prompt::Fraction(q) => {
                let answer = q.answer();
                Answer::Fraction(answer.numerator, answer.denominator)
            }
        }
    }

    /// Whether `answer` is right. Fractions accept equivalent answers
    /// unless the question asks for the simplest form.
    pub fn is_correct(&self, answer: Answer) -> bool {
        match (self, answer) {
            (Prompt::Fraction(q), Answer::Fraction(n, d)) => q.is_correct(n, d),
            _ => answer == self.answer(),
        }
    }

    /// The question as shown to the player, with `separator` between the
    /// whole part and the decimals of a number.
    pub fn parts(&self, separator: &str) -> Vec<Part> {
        match self {
            Prompt::Arithmetic(q) => vec![Part::Text(q.text())],
            Prompt::Comparison(c) => c.parts(separator).into(),
            Prompt::Fraction(q) => vec![Part::Picture(*q)],
        }
    }
}

/// Where the questions of a round come from.
pub enum Questions {
    Arithmetic(Generator),
    Practice(PracticeGenerator),
    Comparison(ComparisonGenerator),
    Fractions(FractionGenerator),
}

impl Questions {
    /// The questions of `config`. Practice rounds pick the weak facts of
    /// `history`.
    pub fn new(config: &GameConfig, history: &FactHistory) -> Self {
        match config.mode {
            Mode::Arithmetic if config.is_practice() => {
                Questions::Practice(PracticeGenerator::new(config.ops, config.level, history))
            }
            Mode::Arithmetic => Questions::Arithmetic(Generator::new(config.ops, config.level)),
            Mode::Comparison => {
                Questions::Comparison(ComparisonGenerator::new(config.kinds, config.level))
            }
            Mode::Fractions => Questions::Fractions(FractionGenerator::new(config.level)),
        }
    }

    fn next(&mut self) -> Prompt {
        match self {
            Questions::Arithmetic(g) => Prompt::Arithmetic(g.next_question()),
            Questions::Practice(g) => Prompt::Arithmetic(g.next_question()),
            Questions::Comparison(g) => Prompt::Comparison(g.next_comparison()),
            Questions::Fractions(g) => Prompt::Fraction(g.next_question()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnswerResult {
    Correct,
    Wrong,
}

/// How an answer went.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Answered {
    pub result: AnswerResult,
    /// The time since the previous answer, or since the round started.
    pub time: Duration,
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
    questions: Questions,
    current: Option<Prompt>,
    limit: u32,
    correct: u32,
    wrong: u32,
    answer_time: Duration,
    last_answer_at: Duration,
}

impl Round {
    pub fn new(config: &GameConfig, history: &FactHistory) -> Self {
        let limit = if config.is_practice() {
            PRACTICE_QUESTIONS
        } else {
            QUESTIONS_PER_ROUND
        };
        Self::with_questions(Questions::new(config, history), limit)
    }

    pub fn with_questions(mut questions: Questions, limit: u32) -> Self {
        let current = (limit > 0).then(|| questions.next());
        Round {
            questions,
            current,
            limit,
            correct: 0,
            wrong: 0,
            answer_time: Duration::ZERO,
            last_answer_at: Duration::ZERO,
        }
    }

    /// The question waiting for an answer, or `None` once the round is over.
    pub fn current(&self) -> Option<&Prompt> {
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
    pub fn submit(&mut self, answer: Answer, elapsed: Duration) -> Option<Answered> {
        let question = self.current?;

        let time = elapsed.saturating_sub(self.last_answer_at);
        self.answer_time += time;
        self.last_answer_at = elapsed;

        let result = if question.is_correct(answer) {
            self.correct += 1;
            AnswerResult::Correct
        } else {
            self.wrong += 1;
            // A missed fact comes back later in a practice round.
            if let (Questions::Practice(g), Prompt::Arithmetic(q)) = (&mut self.questions, question)
            {
                g.missed(&q);
            }
            AnswerResult::Wrong
        };

        self.current = (self.answered() < self.limit).then(|| self.questions.next());
        Some(Answered { result, time })
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
        Round::with_questions(Questions::Arithmetic(generator), limit)
    }

    fn correct(round: &Round) -> Answer {
        round.current().unwrap().answer()
    }

    fn wrong(round: &Round) -> Answer {
        match correct(round) {
            Answer::Number(n) => Answer::Number(n + 1),
            Answer::Order(Ordering::Equal) => Answer::Order(Ordering::Less),
            Answer::Order(order) => Answer::Order(order.reverse()),
            Answer::Fraction(n, d) => Answer::Fraction(n + d, d),
        }
    }

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn answering_all_questions_ends_the_round() {
        let mut round = round(QUESTIONS_PER_ROUND);
        for i in 0..QUESTIONS_PER_ROUND {
            assert!(!round.is_finished());
            let answered = round.submit(correct(&round), secs(i as u64 + 1));
            assert_eq!(
                answered,
                Some(Answered {
                    result: AnswerResult::Correct,
                    time: secs(1),
                })
            );
        }
        assert!(round.is_finished());
        assert!(round.current().is_none());
        assert_eq!(round.submit(Answer::Number(1), secs(100)), None);

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
        let answered = round.submit(wrong(&round), secs(2)).unwrap();
        assert_eq!(answered.result, AnswerResult::Wrong);
        // The first answer takes the time since the round started.
        assert_eq!(answered.time, secs(2));
        assert_ne!(round.current(), Some(&first));
        assert_eq!(round.result().wrong, 1);
        assert_eq!(round.result().correct, 0);
    }

    #[test]
    fn each_answer_takes_the_time_since_the_previous_one() {
        let mut round = round(5);
        let times: Vec<Duration> = [3, 4, 10]
            .into_iter()
            .map(|at| round.submit(correct(&round), secs(at)).unwrap().time)
            .collect();
        assert_eq!(times, [secs(3), secs(1), secs(6)]);
        round.finish();
        assert_eq!(round.submit(Answer::Number(1), secs(11)), None);
    }

    #[test]
    fn finishing_early_counts_the_rest_as_unanswered() {
        let mut round = round(QUESTIONS_PER_ROUND);
        for i in 0..10u64 {
            let answer = if i % 2 == 0 {
                correct(&round)
            } else {
                wrong(&round)
            };
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

    #[test]
    fn comparison_rounds_take_the_order_as_the_answer() {
        let generator = ComparisonGenerator::with_rng(
            ComparisonKinds::ALL,
            Level::new(2),
            fastrand::Rng::with_seed(7),
        );
        let mut round = Round::with_questions(Questions::Comparison(generator), 10);
        for i in 0..10u64 {
            assert!(matches!(round.current(), Some(Prompt::Comparison(_))));
            let answer = if i < 7 {
                correct(&round)
            } else {
                wrong(&round)
            };
            assert!(matches!(answer, Answer::Order(_)));
            round.submit(answer, secs(i + 1));
        }
        assert!(round.is_finished());
        let result = round.result();
        assert_eq!((result.correct, result.wrong), (7, 3));
    }

    #[test]
    fn fraction_rounds_accept_equivalent_answers() {
        let generator = FractionGenerator::with_rng(Level::new(2), fastrand::Rng::with_seed(7));
        let mut round = Round::with_questions(Questions::Fractions(generator), 3);
        let Answer::Fraction(n, d) = correct(&round) else {
            panic!("not a fraction question");
        };
        // Twice the numerator and the denominator is the same fraction.
        round.submit(Answer::Fraction(2 * n, 2 * d), secs(1));
        round.submit(Answer::Fraction(1, 0), secs(2));
        round.submit(wrong(&round), secs(3));
        let result = round.result();
        assert_eq!((result.correct, result.wrong), (1, 2));
    }

    #[test]
    fn practice_rounds_have_20_questions() {
        let config = GameConfig {
            mode: Mode::Arithmetic,
            ops: OperationSet::ALL,
            kinds: ComparisonKinds::ALL,
            practice: true,
            level: Level::new(1),
            duration_secs: 120,
        };
        let history = FactHistory::default();
        let mut round = Round::new(&config, &history);
        assert_eq!(round.limit(), PRACTICE_QUESTIONS);
        for i in 0..PRACTICE_QUESTIONS {
            assert!(matches!(round.current(), Some(Prompt::Arithmetic(_))));
            round.submit(correct(&round), secs(i.into()));
        }
        assert!(round.is_finished());

        // Only arithmetic rounds practise.
        let comparison = GameConfig {
            mode: Mode::Comparison,
            ..config
        };
        assert_eq!(
            Round::new(&comparison, &history).limit(),
            QUESTIONS_PER_ROUND
        );
        let timed = GameConfig {
            practice: false,
            ..config
        };
        assert_eq!(Round::new(&timed, &history).limit(), QUESTIONS_PER_ROUND);
    }

    #[test]
    fn mode_id_round_trips() {
        for mode in [Mode::Arithmetic, Mode::Comparison, Mode::Fractions] {
            assert_eq!(Mode::from_id(mode.id()), mode);
        }
        assert_eq!(Mode::from_id("nonsense"), Mode::Arithmetic);
    }
}
