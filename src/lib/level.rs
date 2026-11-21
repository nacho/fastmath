use super::operation::Operation;

pub const LEVEL_COUNT: u32 = 5;

/// Largest operand of addition and subtraction, per level.
const SUM_MAX: [u32; LEVEL_COUNT as usize] = [10, 20, 50, 100, 1000];
/// Largest factor and divisor of multiplication and division, per level.
const TABLE_MAX: [u32; LEVEL_COUNT as usize] = [5, 10, 12, 15, 20];

/// A difficulty level, from 1 to `LEVEL_COUNT`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Level(u32);

impl Level {
    /// Builds a level, clamping `number` into the valid range.
    pub fn new(number: u32) -> Self {
        Level(number.clamp(1, LEVEL_COUNT))
    }

    pub fn number(self) -> u32 {
        self.0
    }

    /// The largest operand `op` uses at this level.
    pub fn max_operand(self, op: Operation) -> u32 {
        let index = (self.0 - 1) as usize;
        match op {
            Operation::Add | Operation::Subtract => SUM_MAX[index],
            Operation::Multiply | Operation::Divide => TABLE_MAX[index],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clamps() {
        assert_eq!(Level::new(0).number(), 1);
        assert_eq!(Level::new(3).number(), 3);
        assert_eq!(Level::new(99).number(), LEVEL_COUNT);
    }

    #[test]
    fn ranges_grow_with_the_level() {
        for op in Operation::ALL {
            let maxes: Vec<u32> = (1..=LEVEL_COUNT)
                .map(|n| Level::new(n).max_operand(op))
                .collect();
            assert!(maxes.windows(2).all(|w| w[0] < w[1]), "{op:?}: {maxes:?}");
        }
    }
}
