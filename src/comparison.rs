use std::cmp::Ordering;

use crate::generator::{Generator, Question};
use crate::level::Level;
use crate::operation::{Operation, OperationSet};

/// The largest whole number compared at level 1.
pub const WHOLE_MAX: u32 = 100;
/// The most decimal places a number can have.
const MAX_PLACES: u32 = 3;
/// How many thousandths make one unit.
const SCALE: i64 = 10i64.pow(MAX_PLACES);

/// A number with a fixed number of decimal places, so that 8.3 and 8.30
/// are the same value written in two different ways.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Decimal {
    /// The value in thousandths: 8.30 is 8300.
    thousandths: i64,
    /// How many decimal places are shown, from 0 to `MAX_PLACES`.
    places: u32,
}

impl Decimal {
    /// The number `digits / 10^places`, for example 830 with 2 places is 8.30.
    pub fn new(digits: i64, places: u32) -> Self {
        assert!(places <= MAX_PLACES);
        Decimal {
            thousandths: digits * 10i64.pow(MAX_PLACES - places),
            places,
        }
    }

    fn negated(self) -> Self {
        Decimal {
            thousandths: -self.thousandths,
            ..self
        }
    }

    /// The number with all its places, for example `8,30` with "," as
    /// `separator`. Negative numbers use the minus sign, not a hyphen.
    pub fn text(&self, separator: &str) -> String {
        let sign = if self.thousandths < 0 { "−" } else { "" };
        let abs = self.thousandths.unsigned_abs();
        let whole = abs / SCALE as u64;
        if self.places == 0 {
            return format!("{sign}{whole}");
        }
        let fraction = abs % SCALE as u64 / 10u64.pow(MAX_PLACES - self.places);
        let width = self.places as usize;
        format!("{sign}{whole}{separator}{fraction:0width$}")
    }
}

/// One side of a comparison: a number or a calculation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Number(Decimal),
    Calculation(Question),
}

impl Side {
    /// The value in thousandths.
    fn value(&self) -> i64 {
        match self {
            Side::Number(n) => n.thousandths,
            Side::Calculation(q) => i64::from(q.answer) * SCALE,
        }
    }

    pub fn text(&self, separator: &str) -> String {
        match self {
            Side::Number(n) => n.text(separator),
            Side::Calculation(q) => q.text(),
        }
    }
}

/// A question asking whether the left side is less than, equal to or
/// greater than the right side.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Comparison {
    pub left: Side,
    pub right: Side,
}

impl Comparison {
    pub fn answer(&self) -> Ordering {
        self.left.value().cmp(&self.right.value())
    }

    /// The question as shown to the player, for example `8.3 ? 8.25`.
    pub fn text(&self, separator: &str) -> String {
        format!(
            "{} ? {}",
            self.left.text(separator),
            self.right.text(separator)
        )
    }
}

/// Produces random comparisons for a level:
///
/// 1. whole numbers up to `WHOLE_MAX`,
/// 2. decimals with up to two places,
/// 3. decimals with up to three places, some of them negative,
/// 4. a calculation and a number,
/// 5. two calculations.
///
/// Each answer comes up about a third of the time. Unequal decimals are
/// often built to catch place value mistakes, such as thinking that 8.25 is
/// greater than 8.3 because 25 is greater than 3.
pub struct ComparisonGenerator {
    level: Level,
    rng: fastrand::Rng,
    calculations: Generator,
    last: Option<Comparison>,
}

impl ComparisonGenerator {
    pub fn new(level: Level) -> Self {
        Self::with_rng(level, fastrand::Rng::new())
    }

    pub fn with_rng(level: Level, mut rng: fastrand::Rng) -> Self {
        // Calculations against a number use the operands of arithmetic
        // level 2, two calculations those of level 3.
        let calculation_level = Level::new(if level.number() <= 4 { 2 } else { 3 });
        let calculations = Generator::with_rng(OperationSet::ALL, calculation_level, rng.fork());
        ComparisonGenerator {
            level,
            rng,
            calculations,
            last: None,
        }
    }

    /// Returns the next comparison, never equal to the previous one.
    pub fn next_comparison(&mut self) -> Comparison {
        loop {
            let answer = [Ordering::Less, Ordering::Equal, Ordering::Greater][self.rng.usize(..3)];
            let comparison = self.make(answer);
            if self.last != Some(comparison) {
                self.last = Some(comparison);
                return comparison;
            }
        }
    }

    fn make(&mut self, answer: Ordering) -> Comparison {
        let equal = answer == Ordering::Equal;
        let (low, high) = match self.level.number() {
            1 => self.whole_numbers(equal),
            2 => self.decimals(equal, 2, 9),
            3 => self.signed_decimals(equal),
            4 => self.calculation_and_number(equal),
            _ => self.two_calculations(equal),
        };
        let (left, right) = match answer {
            Ordering::Less => (low, high),
            Ordering::Greater => (high, low),
            Ordering::Equal if self.rng.bool() => (low, high),
            Ordering::Equal => (high, low),
        };
        Comparison { left, right }
    }

    /// Two whole numbers up to `WHOLE_MAX`, the second one greater unless
    /// `equal`.
    fn whole_numbers(&mut self, equal: bool) -> (Side, Side) {
        let low = self.rng.u32(1..WHOLE_MAX);
        let high = if equal {
            low
        } else {
            self.rng.u32(low + 1..=(low + 20).min(WHOLE_MAX))
        };
        (
            Side::Number(Decimal::new(low.into(), 0)),
            Side::Number(Decimal::new(high.into(), 0)),
        )
    }

    /// Two numbers from 0 to `max_whole` with up to `max_places` decimals,
    /// the second one greater unless `equal`. Equal numbers are written with
    /// a different number of places, as in 8.3 and 8.30.
    fn decimals(&mut self, equal: bool, max_places: u32, max_whole: i64) -> (Side, Side) {
        let whole = self.rng.i64(0..=max_whole);
        let (low, high) = if equal {
            let short = self.rng.u32(0..max_places);
            let long = self.rng.u32(short + 1..=max_places);
            let digits = whole * 10i64.pow(short) + self.rng.i64(0..10i64.pow(short));
            (
                Decimal::new(digits, short),
                Decimal::new(digits * 10i64.pow(long - short), long),
            )
        } else {
            match self.rng.u8(0..3) {
                0 => self.fewer_places_but_greater(whole, max_places),
                1 => self.shifted_digits(whole, max_places),
                _ => self.close_decimals(whole, max_places),
            }
        };
        (Side::Number(low), Side::Number(high))
    }

    /// A pair like 8.25 and 8.3, where the greater number has fewer places.
    fn fewer_places_but_greater(&mut self, whole: i64, max_places: u32) -> (Decimal, Decimal) {
        let short = self.rng.u32(1..max_places);
        let long = self.rng.u32(short + 1..=max_places);
        let high = whole * 10i64.pow(short) + self.rng.i64(1..10i64.pow(short));
        // Take less than one unit of the last short place away, without
        // ending in 0 so the number really has `long` places.
        let step = 10i64.pow(long - short);
        let below = loop {
            let below = self.rng.i64(1..step);
            if below % 10 != 0 {
                break below;
            }
        };
        (
            Decimal::new(high * step - below, long),
            Decimal::new(high, short),
        )
    }

    /// A pair like 3.04 and 3.4, the same digits one place apart.
    fn shifted_digits(&mut self, whole: i64, max_places: u32) -> (Decimal, Decimal) {
        let places = self.rng.u32(1..max_places);
        let fraction = loop {
            let fraction = self.rng.i64(1..10i64.pow(places));
            if fraction % 10 != 0 {
                break fraction;
            }
        };
        let high = whole * 10i64.pow(places) + fraction;
        let low = whole * 10i64.pow(places + 1) + fraction;
        (Decimal::new(low, places + 1), Decimal::new(high, places))
    }

    /// Two different numbers with the same whole part.
    fn close_decimals(&mut self, whole: i64, max_places: u32) -> (Decimal, Decimal) {
        loop {
            let [a, b] = [(); 2].map(|_| {
                let places = self.rng.u32(1..=max_places);
                Decimal::new(
                    whole * 10i64.pow(places) + self.rng.i64(0..10i64.pow(places)),
                    places,
                )
            });
            match a.thousandths.cmp(&b.thousandths) {
                Ordering::Less => return (a, b),
                Ordering::Greater => return (b, a),
                Ordering::Equal => continue,
            }
        }
    }

    /// Decimals with up to three places from 0 to 20, negative about half
    /// of the time.
    fn signed_decimals(&mut self, equal: bool) -> (Side, Side) {
        let (Side::Number(low), Side::Number(high)) = self.decimals(equal, 3, 20) else {
            unreachable!("decimals() returns numbers");
        };
        let (low, high) = match self.rng.u8(0..10) {
            // Both negative: −8.3 is less than −8.25.
            0..4 => (high.negated(), low.negated()),
            4 if !equal && low.thousandths > 0 => (low.negated(), high),
            _ => (low, high),
        };
        (Side::Number(low), Side::Number(high))
    }

    /// A calculation and a number close to its result.
    fn calculation_and_number(&mut self, equal: bool) -> (Side, Side) {
        let calculation = self.calculations.next_question();
        let delta = self.rng.u32(1..=3);
        let result = calculation.answer;
        let calculation = Side::Calculation(calculation);
        if equal {
            (calculation, Side::Number(Decimal::new(result.into(), 0)))
        } else if result >= delta && self.rng.bool() {
            (
                Side::Number(Decimal::new((result - delta).into(), 0)),
                calculation,
            )
        } else {
            (
                calculation,
                Side::Number(Decimal::new((result + delta).into(), 0)),
            )
        }
    }

    /// Two calculations with the same or a close result.
    fn two_calculations(&mut self, equal: bool) -> (Side, Side) {
        let first = self.calculations.next_question();
        let delta = self.rng.u32(1..=5);
        let (target, first_is_low) = if equal {
            (first.answer, true)
        } else if first.answer >= delta && self.rng.bool() {
            (first.answer - delta, false)
        } else {
            (first.answer + delta, true)
        };
        let second = loop {
            let second = self.calculation_for(target);
            if second != first {
                break second;
            }
        };
        let (first, second) = (Side::Calculation(first), Side::Calculation(second));
        if first_is_low {
            (first, second)
        } else {
            (second, first)
        }
    }

    /// A random calculation whose result is `target`.
    fn calculation_for(&mut self, target: u32) -> Question {
        let max = Level::new(3).max_operand(Operation::Multiply);
        let factors: Vec<u32> = (2..=target / 2)
            .filter(|f| target.is_multiple_of(*f) && target / f >= 2)
            .collect();
        let mut ops = vec![Operation::Subtract];
        if target >= 2 {
            ops.extend([Operation::Add, Operation::Divide]);
        }
        if !factors.is_empty() {
            ops.push(Operation::Multiply);
        }
        let op = ops[self.rng.usize(..ops.len())];
        let (a, b) = match op {
            Operation::Add => {
                let a = self.rng.u32(1..target);
                (a, target - a)
            }
            Operation::Subtract => {
                let b = self.rng.u32(1..=max);
                (target + b, b)
            }
            Operation::Multiply => {
                let a = factors[self.rng.usize(..factors.len())];
                (a, target / a)
            }
            Operation::Divide => {
                let b = self.rng.u32(2..=max);
                (target * b, b)
            }
        };
        Question {
            op,
            a,
            b,
            answer: target,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LEVEL_COUNT;

    const SAMPLES: usize = 3000;

    fn generator(level: u32) -> ComparisonGenerator {
        ComparisonGenerator::with_rng(Level::new(level), fastrand::Rng::with_seed(42))
    }

    fn samples(level: u32) -> Vec<Comparison> {
        let mut gen_ = generator(level);
        (0..SAMPLES).map(|_| gen_.next_comparison()).collect()
    }

    fn numbers(c: &Comparison) -> [Decimal; 2] {
        match (c.left, c.right) {
            (Side::Number(a), Side::Number(b)) => [a, b],
            _ => panic!("expected two numbers: {c:?}"),
        }
    }

    #[test]
    fn decimal_text() {
        assert_eq!(Decimal::new(830, 2).text("."), "8.30");
        assert_eq!(Decimal::new(83, 1).text(","), "8,3");
        assert_eq!(Decimal::new(7, 2).text("."), "0.07");
        assert_eq!(Decimal::new(2045, 3).negated().text("."), "−2.045");
        assert_eq!(Decimal::new(5, 0).text("."), "5");
        assert_eq!(Decimal::new(0, 1).negated().text("."), "0.0");
    }

    #[test]
    fn the_answer_compares_the_values() {
        let number = |digits, places| Side::Number(Decimal::new(digits, places));
        let calculation = Side::Calculation(Question {
            op: Operation::Multiply,
            a: 7,
            b: 8,
            answer: 56,
        });
        let cases = [
            (number(83, 1), number(830, 2), Ordering::Equal),
            (number(83, 1), number(825, 2), Ordering::Greater),
            (number(304, 2), number(34, 1), Ordering::Less),
            (number(-83, 1), number(-825, 2), Ordering::Less),
            (calculation, number(54, 0), Ordering::Greater),
            (calculation, number(560, 1), Ordering::Equal),
        ];
        for (left, right, answer) in cases {
            assert_eq!(Comparison { left, right }.answer(), answer);
        }
    }

    #[test]
    fn text_shows_both_sides() {
        let c = Comparison {
            left: Side::Calculation(Question {
                op: Operation::Multiply,
                a: 7,
                b: 8,
                answer: 56,
            }),
            right: Side::Number(Decimal::new(545, 1)),
        };
        assert_eq!(c.text(","), "7 × 8 ? 54,5");
    }

    #[test]
    fn every_answer_comes_up_about_a_third_of_the_time() {
        for level in 1..=LEVEL_COUNT {
            for answer in [Ordering::Less, Ordering::Equal, Ordering::Greater] {
                let count = samples(level)
                    .iter()
                    .filter(|c| c.answer() == answer)
                    .count();
                let share = count as f64 / SAMPLES as f64;
                assert!(
                    (0.28..0.39).contains(&share),
                    "level {level} {answer:?}: {share}"
                );
            }
        }
    }

    #[test]
    fn level_1_compares_whole_numbers() {
        for c in samples(1) {
            for n in numbers(&c) {
                assert_eq!(n.places, 0, "{c:?}");
                assert!(
                    (SCALE..=i64::from(WHOLE_MAX) * SCALE).contains(&n.thousandths),
                    "{c:?}"
                );
            }
        }
    }

    #[test]
    fn levels_2_and_3_compare_decimals() {
        for (level, max_places) in [(2, 2), (3, 3)] {
            let comparisons = samples(level);
            let mut negative = false;
            for c in &comparisons {
                let [a, b] = numbers(c);
                for n in [a, b] {
                    assert!(n.places <= max_places, "{c:?}");
                    // The places shown are exact.
                    assert_eq!(n.thousandths % 10i64.pow(MAX_PLACES - n.places), 0);
                    negative |= n.thousandths < 0;
                }
                // Equal numbers are written differently.
                if c.answer() == Ordering::Equal {
                    assert_ne!(a.places, b.places, "{c:?}");
                }
            }
            assert_eq!(negative, level == 3, "level {level}");

            // Some questions have the greater number with fewer places.
            assert!(comparisons.iter().any(|c| {
                let [a, b] = numbers(c);
                let (low, high) = if a.thousandths < b.thousandths {
                    (a, b)
                } else {
                    (b, a)
                };
                low.thousandths > 0 && high.places < low.places
            }));
        }
    }

    #[test]
    fn level_4_compares_a_calculation_and_a_number() {
        for c in samples(4) {
            match (c.left, c.right) {
                (Side::Calculation(_), Side::Number(n))
                | (Side::Number(n), Side::Calculation(_)) => {
                    assert_eq!(n.places, 0, "{c:?}");
                    assert!(n.thousandths >= 0, "{c:?}");
                }
                _ => panic!("expected a calculation and a number: {c:?}"),
            }
        }
    }

    #[test]
    fn level_5_compares_two_correct_calculations() {
        for c in samples(5) {
            let (Side::Calculation(a), Side::Calculation(b)) = (c.left, c.right) else {
                panic!("expected two calculations: {c:?}");
            };
            assert_ne!(a, b);
            for q in [a, b] {
                let result = match q.op {
                    Operation::Add => q.a + q.b,
                    Operation::Subtract => q.a - q.b,
                    Operation::Multiply => q.a * q.b,
                    Operation::Divide => {
                        assert_eq!(q.a % q.b, 0, "{q:?}");
                        q.a / q.b
                    }
                };
                assert_eq!(q.answer, result, "{q:?}");
            }
        }
    }

    #[test]
    fn the_same_comparison_never_comes_twice_in_a_row() {
        for level in 1..=LEVEL_COUNT {
            let comparisons = samples(level);
            assert!(
                comparisons.windows(2).all(|w| w[0] != w[1]),
                "level {level}"
            );
        }
    }
}
