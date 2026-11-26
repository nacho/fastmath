use std::cmp::Ordering;

use crate::level::Level;

/// A fraction with an optional whole part, such as 3/4 or 1 3/4.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, glib::Boxed)]
#[boxed_type(name = "FastmathFraction", nullable)]
pub struct Fraction {
    pub whole: u32,
    pub numerator: u32,
    pub denominator: u32,
}

impl Fraction {
    pub fn new(numerator: u32, denominator: u32) -> Self {
        Fraction {
            whole: 0,
            numerator,
            denominator,
        }
    }

    /// The fraction as numerator and denominator of its whole value, so
    /// 1 3/4 is 7/4.
    pub fn improper(&self) -> (u64, u64) {
        let denominator = u64::from(self.denominator);
        (
            u64::from(self.whole) * denominator + u64::from(self.numerator),
            denominator,
        )
    }

    /// The same value with the smallest numerator and denominator and no
    /// whole part, so 1 2/4 is 3/2.
    pub fn simplest(&self) -> Fraction {
        let (n, d) = self.improper();
        let divisor = gcd(n, d).max(1);
        Fraction::new((n / divisor) as u32, (d / divisor) as u32)
    }

    /// Compares the values, so 1/2 and 2/4 are equal.
    pub fn cmp_value(&self, other: &Fraction) -> Ordering {
        let (a, b) = self.improper();
        let (c, d) = other.improper();
        (a * d).cmp(&(c * b))
    }

    /// The fraction as text, for example "3/4" or "1 3/4".
    #[cfg(test)]
    pub fn text(&self) -> String {
        match (self.whole, self.numerator) {
            (0, n) => format!("{n}/{}", self.denominator),
            (w, 0) => w.to_string(),
            (w, n) => format!("{w} {n}/{}", self.denominator),
        }
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// What a picture shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Shape {
    Pizza,
}

/// A picture of `wholes` shapes, each cut into `pieces`. The first `shaded`
/// pieces are eaten, counting across the shapes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, glib::Boxed)]
#[boxed_type(name = "FastmathPicture", nullable)]
pub struct Picture {
    pub shape: Shape,
    pub wholes: u32,
    pub pieces: u32,
    pub shaded: u32,
}

impl Picture {
    /// How many pieces of the whole number `index` are shaded.
    pub fn shaded_in(&self, index: u32) -> u32 {
        self.shaded
            .saturating_sub(index * self.pieces)
            .min(self.pieces)
    }
}

/// Whether the question is about the shaded part or the rest.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Ask {
    Eaten,
    Left,
}

/// A question asking which fraction of a picture was eaten or is left.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FractionQuestion {
    pub picture: Picture,
    pub ask: Ask,
    /// Whether only the simplest form is accepted, 1/2 and not 2/4.
    pub simplest_only: bool,
}

impl FractionQuestion {
    /// The answer in its simplest form, which is always accepted.
    pub fn answer(&self) -> Fraction {
        let p = self.picture;
        let numerator = match self.ask {
            Ask::Eaten => p.shaded,
            Ask::Left => p.wholes * p.pieces - p.shaded,
        };
        Fraction::new(numerator, p.pieces).simplest()
    }

    pub fn is_correct(&self, numerator: u32, denominator: u32) -> bool {
        if denominator == 0 {
            return false;
        }
        let given = Fraction::new(numerator, denominator);
        given.cmp_value(&self.answer()) == Ordering::Equal
            && (!self.simplest_only || given == given.simplest())
    }
}

/// Produces fraction questions for a level:
///
/// 1. a pizza in halves, thirds or quarters, asking what was eaten,
/// 2. up to twelfths,
/// 3. asking what was eaten or what is left,
/// 4. only the simplest form is accepted,
/// 5. two pizzas with more than one eaten, in simplest form.
pub struct FractionGenerator {
    level: Level,
    rng: fastrand::Rng,
    last: Option<FractionQuestion>,
}

impl FractionGenerator {
    pub fn new(level: Level) -> Self {
        Self::with_rng(level, fastrand::Rng::new())
    }

    pub fn with_rng(level: Level, rng: fastrand::Rng) -> Self {
        FractionGenerator {
            level,
            rng,
            last: None,
        }
    }

    /// Returns the next question, never equal to the previous one.
    pub fn next_question(&mut self) -> FractionQuestion {
        loop {
            let question = self.make();
            if self.last != Some(question) {
                self.last = Some(question);
                return question;
            }
        }
    }

    fn make(&mut self) -> FractionQuestion {
        let level = self.level.number();
        let (wholes, pieces) = match level {
            1 => (1, self.rng.u32(2..=4)),
            2..=4 => (1, self.rng.u32(2..=12)),
            _ => (2, self.rng.u32(2..=4)),
        };
        let shaded = if level == 5 {
            // More than one whole and less than two.
            self.rng.u32(pieces + 1..2 * pieces)
        } else if level == 4 && self.rng.bool() {
            // Half of the time the answer can be simplified, so the
            // simplest form matters: 4/8 is 1/2.
            let factors: Vec<u32> = (2..pieces).filter(|f| pieces.is_multiple_of(*f)).collect();
            match factors.as_slice() {
                [] => self.rng.u32(1..pieces),
                factors => {
                    let f = factors[self.rng.usize(..factors.len())];
                    f * self.rng.u32(1..pieces / f)
                }
            }
        } else {
            self.rng.u32(1..pieces)
        };
        let ask = if (3..=4).contains(&level) && self.rng.bool() {
            Ask::Left
        } else {
            Ask::Eaten
        };
        FractionQuestion {
            picture: Picture {
                shape: Shape::Pizza,
                wholes,
                pieces,
                shaded,
            },
            ask,
            simplest_only: level >= 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_compare_across_forms() {
        let half = Fraction::new(1, 2);
        assert_eq!(half.cmp_value(&Fraction::new(2, 4)), Ordering::Equal);
        assert_eq!(
            Fraction::new(1, 3).cmp_value(&Fraction::new(1, 4)),
            Ordering::Greater
        );
        assert_eq!(
            Fraction::new(2, 3).cmp_value(&Fraction::new(3, 4)),
            Ordering::Less
        );
        let mixed = Fraction {
            whole: 1,
            numerator: 3,
            denominator: 4,
        };
        assert_eq!(mixed.improper(), (7, 4));
        assert_eq!(Fraction::new(7, 4).cmp_value(&mixed), Ordering::Equal);
        assert_eq!(Fraction::new(8, 4).cmp_value(&mixed), Ordering::Greater);
    }

    #[test]
    fn simplest_form() {
        assert_eq!(Fraction::new(4, 8).simplest(), Fraction::new(1, 2));
        assert_eq!(Fraction::new(3, 4).simplest(), Fraction::new(3, 4));
        assert_eq!(Fraction::new(6, 4).simplest(), Fraction::new(3, 2));
        let mixed = Fraction {
            whole: 1,
            numerator: 2,
            denominator: 4,
        };
        assert_eq!(mixed.simplest(), Fraction::new(3, 2));
    }

    fn question(pieces: u32, shaded: u32, ask: Ask, simplest_only: bool) -> FractionQuestion {
        FractionQuestion {
            picture: Picture {
                shape: Shape::Pizza,
                wholes: 1,
                pieces,
                shaded,
            },
            ask,
            simplest_only,
        }
    }

    #[test]
    fn answers_accept_equivalents_unless_simplest_only() {
        let q = question(8, 4, Ask::Eaten, false);
        assert_eq!(q.answer(), Fraction::new(1, 2));
        assert!(q.is_correct(4, 8));
        assert!(q.is_correct(1, 2));
        assert!(q.is_correct(2, 4));
        assert!(!q.is_correct(3, 8));
        assert!(!q.is_correct(4, 0));
        assert!(!q.is_correct(0, 0));

        let q = question(8, 4, Ask::Eaten, true);
        assert!(q.is_correct(1, 2));
        assert!(!q.is_correct(4, 8));

        let q = question(8, 3, Ask::Left, false);
        assert_eq!(q.answer(), Fraction::new(5, 8));
        assert!(q.is_correct(5, 8) && q.is_correct(10, 16));
        assert!(!q.is_correct(3, 8));
    }

    #[test]
    fn improper_answers() {
        let q = FractionQuestion {
            picture: Picture {
                shape: Shape::Pizza,
                wholes: 2,
                pieces: 4,
                shaded: 6,
            },
            ask: Ask::Eaten,
            simplest_only: true,
        };
        assert_eq!(q.answer(), Fraction::new(3, 2));
        assert!(q.is_correct(3, 2));
        assert!(!q.is_correct(6, 4));
        assert_eq!(q.picture.shaded_in(0), 4);
        assert_eq!(q.picture.shaded_in(1), 2);
    }

    const SAMPLES: usize = 2000;

    fn samples(level: u32) -> Vec<FractionQuestion> {
        let mut gen_ = FractionGenerator::with_rng(Level::new(level), fastrand::Rng::with_seed(9));
        (0..SAMPLES).map(|_| gen_.next_question()).collect()
    }

    #[test]
    fn questions_follow_the_level() {
        for level in 1..=5 {
            let questions = samples(level);
            for q in &questions {
                let p = q.picture;
                match level {
                    1 => assert!((2..=4).contains(&p.pieces), "{q:?}"),
                    2..=4 => assert!((2..=12).contains(&p.pieces), "{q:?}"),
                    _ => assert!((2..=4).contains(&p.pieces), "{q:?}"),
                }
                if level < 5 {
                    assert_eq!(p.wholes, 1, "{q:?}");
                    assert!(p.shaded >= 1 && p.shaded < p.pieces, "{q:?}");
                } else {
                    assert_eq!(p.wholes, 2, "{q:?}");
                    assert!(p.shaded > p.pieces && p.shaded < 2 * p.pieces, "{q:?}");
                }
                assert_eq!(q.simplest_only, level >= 4, "{q:?}");
                assert!(q.is_correct(q.answer().numerator, q.answer().denominator));
            }
            let left = questions.iter().any(|q| q.ask == Ask::Left);
            assert_eq!(left, (3..=4).contains(&level), "level {level}");
            assert!(questions.windows(2).all(|w| w[0] != w[1]));
        }

        // At level 4 the simplest form often differs from what the picture
        // shows.
        let simplified = samples(4)
            .iter()
            .filter(|q| q.answer().denominator != q.picture.pieces)
            .count();
        assert!(simplified > SAMPLES / 3, "{simplified}");
    }

    #[test]
    fn text() {
        assert_eq!(Fraction::new(3, 4).text(), "3/4");
        let mixed = Fraction {
            whole: 1,
            numerator: 3,
            denominator: 4,
        };
        assert_eq!(mixed.text(), "1 3/4");
    }
}
