use crate::level::Level;
use crate::operation::{Operation, OperationSet};

/// An arithmetic question with an integer answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Question {
    pub op: Operation,
    pub a: u32,
    pub b: u32,
    pub answer: u32,
}

impl Question {
    /// The question as shown to the player, for example `7 × 8`.
    pub fn text(&self) -> String {
        format!("{} {} {}", self.a, self.op.symbol(), self.b)
    }
}

/// Produces random questions for a set of operations and a level.
pub struct Generator {
    ops: Vec<Operation>,
    level: Level,
    rng: fastrand::Rng,
    last: Option<Question>,
}

impl Generator {
    pub fn new(ops: OperationSet, level: Level) -> Self {
        Self::with_rng(ops, level, fastrand::Rng::new())
    }

    pub fn with_rng(ops: OperationSet, level: Level, rng: fastrand::Rng) -> Self {
        // An empty set cannot produce questions, so play with everything.
        let ops = if ops.is_empty() {
            OperationSet::ALL
        } else {
            ops
        };
        Generator {
            ops: ops.iter().collect(),
            level,
            rng,
            last: None,
        }
    }

    /// Whether this generator could ask `question`: it uses one of its
    /// operations, with the operands of its level.
    pub fn could_ask(&self, question: &Question) -> bool {
        let q = question;
        if !self.ops.contains(&q.op) {
            return false;
        }
        let max = self.level.max_operand(q.op);
        let operand = |n: u32, min: u32| (min..=max).contains(&n);
        match q.op {
            Operation::Add => operand(q.a, 1) && operand(q.b, 1) && q.answer == q.a + q.b,
            Operation::Subtract => {
                operand(q.a, 1) && (1..=q.a).contains(&q.b) && q.answer == q.a - q.b
            }
            Operation::Multiply => operand(q.a, 2) && operand(q.b, 2) && q.answer == q.a * q.b,
            Operation::Divide => operand(q.b, 2) && operand(q.answer, 2) && q.a == q.b * q.answer,
        }
    }

    /// Returns the next question, never equal to the previous one.
    pub fn next_question(&mut self) -> Question {
        loop {
            let op = self.ops[self.rng.usize(..self.ops.len())];
            let question = self.make(op);
            if self.last != Some(question) {
                self.last = Some(question);
                return question;
            }
        }
    }

    fn make(&mut self, op: Operation) -> Question {
        let max = self.level.max_operand(op);
        match op {
            Operation::Add => {
                let a = self.rng.u32(1..=max);
                let b = self.rng.u32(1..=max);
                Question {
                    op,
                    a,
                    b,
                    answer: a + b,
                }
            }
            Operation::Subtract => {
                let a = self.rng.u32(1..=max);
                let b = self.rng.u32(1..=a);
                Question {
                    op,
                    a,
                    b,
                    answer: a - b,
                }
            }
            Operation::Multiply => {
                let a = self.rng.u32(2..=max);
                let b = self.rng.u32(2..=max);
                Question {
                    op,
                    a,
                    b,
                    answer: a * b,
                }
            }
            Operation::Divide => {
                let b = self.rng.u32(2..=max);
                let answer = self.rng.u32(2..=max);
                Question {
                    op,
                    a: b * answer,
                    b,
                    answer,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LEVEL_COUNT;

    const SAMPLES: usize = 2000;

    fn generator(ops: OperationSet, level: u32) -> Generator {
        Generator::with_rng(ops, Level::new(level), fastrand::Rng::with_seed(42))
    }

    fn single(op: Operation) -> OperationSet {
        OperationSet::from_id_string("").with(op, true)
    }

    #[test]
    fn answers_match_the_operation() {
        for op in Operation::ALL {
            for level in 1..=LEVEL_COUNT {
                let mut gen_ = generator(single(op), level);
                for _ in 0..SAMPLES {
                    let q = gen_.next_question();
                    assert_eq!(q.op, op);
                    let expected = match op {
                        Operation::Add => q.a + q.b,
                        Operation::Subtract => q.a - q.b,
                        Operation::Multiply => q.a * q.b,
                        Operation::Divide => q.a / q.b,
                    };
                    assert_eq!(q.answer, expected, "{q:?}");
                }
            }
        }
    }

    #[test]
    fn division_is_exact_and_never_by_zero() {
        for level in 1..=LEVEL_COUNT {
            let mut gen_ = generator(single(Operation::Divide), level);
            for _ in 0..SAMPLES {
                let q = gen_.next_question();
                assert!(q.b >= 2, "{q:?}");
                assert_eq!(q.a % q.b, 0, "{q:?}");
            }
        }
    }

    #[test]
    fn subtraction_is_never_negative() {
        for level in 1..=LEVEL_COUNT {
            let mut gen_ = generator(single(Operation::Subtract), level);
            for _ in 0..SAMPLES {
                let q = gen_.next_question();
                assert!(q.a >= q.b, "{q:?}");
            }
        }
    }

    #[test]
    fn operands_stay_in_range() {
        for op in Operation::ALL {
            for n in 1..=LEVEL_COUNT {
                let level = Level::new(n);
                let max = level.max_operand(op);
                let mut gen_ = generator(single(op), n);
                for _ in 0..SAMPLES {
                    let q = gen_.next_question();
                    match op {
                        Operation::Add | Operation::Subtract => {
                            assert!((1..=max).contains(&q.a) && (1..=max).contains(&q.b));
                        }
                        Operation::Multiply => {
                            assert!((2..=max).contains(&q.a) && (2..=max).contains(&q.b));
                        }
                        Operation::Divide => {
                            assert!((2..=max).contains(&q.b) && (2..=max).contains(&q.answer));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn mixed_sets_use_only_and_all_of_the_selected_operations() {
        let set = single(Operation::Add).with(Operation::Divide, true);
        let mut gen_ = generator(set, 2);
        let mut seen = Vec::new();
        for _ in 0..SAMPLES {
            let q = gen_.next_question();
            assert!(set.contains(q.op), "{q:?}");
            if !seen.contains(&q.op) {
                seen.push(q.op);
            }
        }
        assert_eq!(seen.len(), 2);
    }

    #[test]
    fn the_same_question_never_comes_twice_in_a_row() {
        // The smallest pool: level 1 division has 4 x 4 = 16 questions.
        for set in [single(Operation::Divide), OperationSet::ALL] {
            let mut gen_ = generator(set, 1);
            let mut previous = gen_.next_question();
            for _ in 0..SAMPLES {
                let q = gen_.next_question();
                assert_ne!(q, previous);
                previous = q;
            }
        }
    }

    #[test]
    fn an_empty_set_falls_back_to_all_operations() {
        let mut gen_ = generator(OperationSet::from_id_string(""), 1);
        let ops: Vec<Operation> = (0..SAMPLES).map(|_| gen_.next_question().op).collect();
        for op in Operation::ALL {
            assert!(ops.contains(&op));
        }
    }

    #[test]
    fn generators_could_ask_their_own_questions_only() {
        for op in Operation::ALL {
            for level in 1..=LEVEL_COUNT {
                let mut gen_ = generator(single(op), level);
                for _ in 0..SAMPLES {
                    let q = gen_.next_question();
                    assert!(gen_.could_ask(&q), "{q:?}");
                }
            }
        }
        let gen_ = generator(single(Operation::Multiply), 2);
        let q = |op, a, b, answer| Question { op, a, b, answer };
        assert!(gen_.could_ask(&q(Operation::Multiply, 7, 8, 56)));
        // Another operation, too large for the level, a factor of 1 or a
        // wrong answer.
        assert!(!gen_.could_ask(&q(Operation::Add, 7, 8, 15)));
        assert!(!gen_.could_ask(&q(Operation::Multiply, 7, 11, 77)));
        assert!(!gen_.could_ask(&q(Operation::Multiply, 1, 8, 8)));
        assert!(!gen_.could_ask(&q(Operation::Multiply, 7, 8, 54)));
        let gen_ = generator(single(Operation::Divide), 2);
        assert!(gen_.could_ask(&q(Operation::Divide, 56, 7, 8)));
        assert!(!gen_.could_ask(&q(Operation::Divide, 56, 7, 9)));
        let gen_ = generator(single(Operation::Subtract), 1);
        assert!(gen_.could_ask(&q(Operation::Subtract, 9, 9, 0)));
        assert!(!gen_.could_ask(&q(Operation::Subtract, 5, 9, 0)));
    }

    #[test]
    fn text_uses_the_operation_symbol() {
        let q = Question {
            op: Operation::Multiply,
            a: 7,
            b: 8,
            answer: 56,
        };
        assert_eq!(q.text(), "7 × 8");
    }
}
