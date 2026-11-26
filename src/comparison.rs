use std::cmp::Ordering;

use crate::choice::{Choice, ChoiceSet};
use crate::generator::{Generator, Question};
use crate::level::{LEVEL_COUNT, Level};
use crate::operation::{Operation, OperationSet};

/// The largest whole number compared, per level. Levels 4 and 5 go from
/// minus this to this.
const WHOLE_MAX: [u32; LEVEL_COUNT as usize] = [20, 100, 1000, 100, 1000];
/// The most decimal places shown, per level.
const DECIMAL_PLACES: [u32; LEVEL_COUNT as usize] = [1, 2, 3, 2, 3];
/// The arithmetic level of the calculations compared, per level.
const CALCULATION_LEVEL: [u32; LEVEL_COUNT as usize] = [1, 2, 2, 3, 4];
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

/// What a comparison compares.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ComparisonKind {
    WholeNumbers,
    Decimals,
    Calculations,
}

impl Choice for ComparisonKind {
    const ALL: &'static [Self] = &[
        ComparisonKind::WholeNumbers,
        ComparisonKind::Decimals,
        ComparisonKind::Calculations,
    ];

    /// The letter used to store the kind in settings and file names.
    fn id(self) -> char {
        match self {
            ComparisonKind::WholeNumbers => 'w',
            ComparisonKind::Decimals => 'd',
            ComparisonKind::Calculations => 'c',
        }
    }
}

/// A set of comparison kinds, stored as `"wdc"` letters.
pub type ComparisonKinds = ChoiceSet<ComparisonKind>;

/// The largest whole number compared at `level`.
pub fn whole_max(level: Level) -> u32 {
    WHOLE_MAX[level.number() as usize - 1]
}

/// Whether `level` compares negative whole numbers and decimals.
pub fn has_negatives(level: Level) -> bool {
    level.number() >= 4
}

/// The arithmetic level whose operands the calculations use at `level`.
pub fn calculation_level(level: Level) -> Level {
    Level::new(CALCULATION_LEVEL[level.number() as usize - 1])
}

/// Produces random comparisons of the selected kinds for a level:
///
/// | Level | Whole numbers | Decimals                   | Calculations               |
/// |-------|---------------|----------------------------|----------------------------|
/// | 1     | up to 20      | tenths                     | a sum or difference and a number |
/// | 2     | up to 100     | tenths and hundredths      | a calculation and a number |
/// | 3     | up to 1000    | thousandths                | two calculations           |
/// | 4     | −100 to 100   | negatives with hundredths  | two calculations           |
/// | 5     | −1000 to 1000 | negatives with thousandths | two calculations           |
///
/// Calculations use the operands of `calculation_level`. Each answer comes
/// up about a third of the time. Unequal decimals are often built to catch
/// place value mistakes, such as thinking that 8.25 is greater than 8.3
/// because 25 is greater than 3.
pub struct ComparisonGenerator {
    kinds: Vec<ComparisonKind>,
    level: Level,
    rng: fastrand::Rng,
    calculations: Generator,
    last: Option<Comparison>,
}

impl ComparisonGenerator {
    pub fn new(kinds: ComparisonKinds, level: Level) -> Self {
        Self::with_rng(kinds, level, fastrand::Rng::new())
    }

    pub fn with_rng(kinds: ComparisonKinds, level: Level, mut rng: fastrand::Rng) -> Self {
        // An empty set cannot produce questions, so compare everything.
        let kinds = if kinds.is_empty() {
            ComparisonKinds::ALL
        } else {
            kinds
        };
        // A calculation against a number at level 1 is a sum or a
        // difference.
        let ops = if level.number() == 1 {
            OperationSet::from_id_string("as")
        } else {
            OperationSet::ALL
        };
        let calculations = Generator::with_rng(ops, calculation_level(level), rng.fork());
        ComparisonGenerator {
            kinds: kinds.iter().collect(),
            level,
            rng,
            calculations,
            last: None,
        }
    }

    /// Returns the next comparison, never equal to the previous one.
    pub fn next_comparison(&mut self) -> Comparison {
        loop {
            let kind = self.kinds[self.rng.usize(..self.kinds.len())];
            let answer = [Ordering::Less, Ordering::Equal, Ordering::Greater][self.rng.usize(..3)];
            let comparison = self.make(kind, answer);
            if self.last != Some(comparison) {
                self.last = Some(comparison);
                return comparison;
            }
        }
    }

    fn make(&mut self, kind: ComparisonKind, answer: Ordering) -> Comparison {
        let equal = answer == Ordering::Equal;
        let (low, high) = match kind {
            ComparisonKind::WholeNumbers => {
                let (low, high) = self.whole_numbers(equal);
                (Side::Number(low), Side::Number(high))
            }
            ComparisonKind::Decimals => {
                let (low, high) = self.decimals(equal);
                (Side::Number(low), Side::Number(high))
            }
            ComparisonKind::Calculations if self.level.number() <= 2 => {
                self.calculation_and_number(equal)
            }
            ComparisonKind::Calculations => self.two_calculations(equal),
        };
        let (left, right) = match answer {
            Ordering::Less => (low, high),
            Ordering::Greater => (high, low),
            Ordering::Equal if self.rng.bool() => (low, high),
            Ordering::Equal => (high, low),
        };
        Comparison { left, right }
    }

    /// Two whole numbers, the second one greater unless `equal`.
    fn whole_numbers(&mut self, equal: bool) -> (Decimal, Decimal) {
        let max = whole_max(self.level);
        let low = self.rng.u32(1..max);
        let high = if equal {
            low
        } else {
            self.rng.u32(low + 1..=(low + max / 5).min(max))
        };
        let pair = (Decimal::new(low.into(), 0), Decimal::new(high.into(), 0));
        self.with_signs(pair, equal)
    }

    /// Makes most pairs negative at the levels with negative numbers: both
    /// numbers, so −54 is less than −45, or only the smaller one.
    fn with_signs(&mut self, (low, high): (Decimal, Decimal), equal: bool) -> (Decimal, Decimal) {
        if !has_negatives(self.level) {
            return (low, high);
        }
        match self.rng.u8(0..10) {
            0..6 => (high.negated(), low.negated()),
            6..8 if !equal && low.thousandths > 0 => (low.negated(), high),
            _ => (low, high),
        }
    }

    /// Two decimals, the second one greater unless `equal`. Equal numbers
    /// are written with a different number of places, as in 8.3 and 8.30.
    fn decimals(&mut self, equal: bool) -> (Decimal, Decimal) {
        let max_places = DECIMAL_PLACES[self.level.number() as usize - 1];
        let max_whole = if self.level.number() <= 2 { 9 } else { 20 };
        let whole = self.rng.i64(0..=max_whole);
        let pair = if equal {
            let short = self.rng.u32(0..max_places);
            let long = self.rng.u32(short + 1..=max_places);
            let digits = whole * 10i64.pow(short) + self.rng.i64(0..10i64.pow(short));
            (
                Decimal::new(digits, short),
                Decimal::new(digits * 10i64.pow(long - short), long),
            )
        } else if max_places == 1 {
            // With only tenths there are no place value traps.
            self.close_decimals(whole, max_places)
        } else {
            match self.rng.u8(0..3) {
                0 => self.fewer_places_but_greater(whole, max_places),
                1 => self.shifted_digits(whole, max_places),
                _ => self.close_decimals(whole, max_places),
            }
        };
        self.with_signs(pair, equal)
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
        let max = calculation_level(self.level).max_operand(Operation::Multiply);
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

    const SAMPLES: usize = 3000;

    fn only(kind: ComparisonKind) -> ComparisonKinds {
        ComparisonKinds::from_id_string("").with(kind, true)
    }

    fn samples(kinds: ComparisonKinds, level: u32) -> Vec<Comparison> {
        let mut gen_ =
            ComparisonGenerator::with_rng(kinds, Level::new(level), fastrand::Rng::with_seed(42));
        (0..SAMPLES).map(|_| gen_.next_comparison()).collect()
    }

    fn numbers(c: &Comparison) -> [Decimal; 2] {
        match (c.left, c.right) {
            (Side::Number(a), Side::Number(b)) => [a, b],
            _ => panic!("expected two numbers: {c:?}"),
        }
    }

    /// The kind a comparison was made for. Whole numbers never have places,
    /// decimals always have them on at least one side.
    fn kind_of(c: &Comparison) -> ComparisonKind {
        match (c.left, c.right) {
            (Side::Number(a), Side::Number(b)) if a.places == 0 && b.places == 0 => {
                ComparisonKind::WholeNumbers
            }
            (Side::Number(_), Side::Number(_)) => ComparisonKind::Decimals,
            _ => ComparisonKind::Calculations,
        }
    }

    fn check_calculation(q: &Question) {
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
        for &kind in ComparisonKind::ALL {
            for level in 1..=LEVEL_COUNT {
                let comparisons = samples(only(kind), level);
                for answer in [Ordering::Less, Ordering::Equal, Ordering::Greater] {
                    let count = comparisons.iter().filter(|c| c.answer() == answer).count();
                    let share = count as f64 / SAMPLES as f64;
                    assert!(
                        (0.28..0.39).contains(&share),
                        "{kind:?} level {level} {answer:?}: {share}"
                    );
                }
            }
        }
    }

    #[test]
    fn whole_numbers_stay_in_range() {
        for level in 1..=LEVEL_COUNT {
            let max = i64::from(whole_max(Level::new(level))) * SCALE;
            let mut negative = false;
            for c in samples(only(ComparisonKind::WholeNumbers), level) {
                for n in numbers(&c) {
                    assert_eq!(n.places, 0, "{c:?}");
                    assert!(n.thousandths.abs() <= max && n.thousandths != 0, "{c:?}");
                    negative |= n.thousandths < 0;
                }
            }
            assert_eq!(negative, level >= 4, "level {level}");
        }
    }

    #[test]
    fn decimals_stay_in_range() {
        for (level, max_places, max_whole) in
            [(1, 1, 9), (2, 2, 9), (3, 3, 20), (4, 2, 20), (5, 3, 20)]
        {
            let comparisons = samples(only(ComparisonKind::Decimals), level);
            let mut negative = false;
            for c in &comparisons {
                let [a, b] = numbers(c);
                for n in [a, b] {
                    assert!(n.places <= max_places, "{c:?}");
                    assert!(n.thousandths.abs() < (max_whole + 1) * SCALE, "{c:?}");
                    // The places shown are exact.
                    assert_eq!(n.thousandths % 10i64.pow(MAX_PLACES - n.places), 0);
                    negative |= n.thousandths < 0;
                }
                assert!(a.places > 0 || b.places > 0, "{c:?}");
                // Equal numbers are written differently.
                if c.answer() == Ordering::Equal {
                    assert_ne!(a.places, b.places, "{c:?}");
                }
            }
            assert_eq!(negative, level >= 4, "level {level}");

            // From level 2, some questions have the greater number with
            // fewer places.
            let trap = comparisons.iter().any(|c| {
                let [a, b] = numbers(c);
                let (low, high) = if a.thousandths < b.thousandths {
                    (a, b)
                } else {
                    (b, a)
                };
                c.answer() != Ordering::Equal && low.thousandths > 0 && high.places < low.places
            });
            assert_eq!(trap, level >= 2, "level {level}");
        }
    }

    #[test]
    fn calculations_follow_the_level() {
        for level in 1..=LEVEL_COUNT {
            let max = calculation_level(Level::new(level)).max_operand(Operation::Add);
            for c in samples(only(ComparisonKind::Calculations), level) {
                match (c.left, c.right) {
                    (Side::Calculation(q), Side::Number(n))
                    | (Side::Number(n), Side::Calculation(q)) => {
                        assert!(level <= 2, "level {level}: {c:?}");
                        assert_eq!(n.places, 0, "{c:?}");
                        assert!(n.thousandths >= 0, "{c:?}");
                        check_calculation(&q);
                        if level == 1 {
                            assert!(matches!(q.op, Operation::Add | Operation::Subtract));
                            assert!(q.a <= max && q.b <= max, "{q:?}");
                        }
                    }
                    (Side::Calculation(a), Side::Calculation(b)) => {
                        assert!(level >= 3, "level {level}: {c:?}");
                        assert_ne!(a, b);
                        check_calculation(&a);
                        check_calculation(&b);
                    }
                    _ => panic!("expected a calculation: {c:?}"),
                }
            }
        }
    }

    #[test]
    fn only_and_all_of_the_selected_kinds_come_up() {
        let all = ComparisonKinds::ALL;
        let some = only(ComparisonKind::WholeNumbers).with(ComparisonKind::Calculations, true);
        let empty = ComparisonKinds::from_id_string("");
        for (set, expected) in [(all, all), (some, some), (empty, all)] {
            for level in 1..=LEVEL_COUNT {
                let seen = samples(set, level)
                    .iter()
                    .fold(ComparisonKinds::from_id_string(""), |seen, c| {
                        seen.with(kind_of(c), true)
                    });
                assert_eq!(seen, expected, "level {level}");
            }
        }
    }

    #[test]
    fn the_same_comparison_never_comes_twice_in_a_row() {
        for &kind in ComparisonKind::ALL {
            for level in 1..=LEVEL_COUNT {
                let comparisons = samples(only(kind), level);
                assert!(
                    comparisons.windows(2).all(|w| w[0] != w[1]),
                    "{kind:?} level {level}"
                );
            }
        }
    }

    #[test]
    fn kinds_use_their_letters() {
        assert_eq!(ComparisonKinds::ALL.to_id_string(), "wdc");
    }
}
