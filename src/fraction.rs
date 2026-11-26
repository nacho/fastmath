use std::cmp::Ordering;

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
