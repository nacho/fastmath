/// One of the four arithmetic operations the game can ask about.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Operation {
    pub const ALL: [Operation; 4] = [
        Operation::Add,
        Operation::Subtract,
        Operation::Multiply,
        Operation::Divide,
    ];

    /// The symbol shown between the operands.
    pub fn symbol(self) -> &'static str {
        match self {
            Operation::Add => "+",
            Operation::Subtract => "−",
            Operation::Multiply => "×",
            Operation::Divide => "÷",
        }
    }

    /// The letter used to store the operation in settings and file names.
    fn id(self) -> char {
        match self {
            Operation::Add => 'a',
            Operation::Subtract => 's',
            Operation::Multiply => 'm',
            Operation::Divide => 'd',
        }
    }

    fn bit(self) -> u8 {
        1 << (self as u8)
    }
}

/// A set of operations, stored as a bitmask.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct OperationSet(u8);

impl OperationSet {
    pub const ALL: OperationSet = OperationSet(0b1111);

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn contains(self, op: Operation) -> bool {
        self.0 & op.bit() != 0
    }

    pub fn with(self, op: Operation, enabled: bool) -> Self {
        if enabled {
            OperationSet(self.0 | op.bit())
        } else {
            OperationSet(self.0 & !op.bit())
        }
    }

    pub fn iter(self) -> impl Iterator<Item = Operation> {
        Operation::ALL
            .into_iter()
            .filter(move |op| self.contains(*op))
    }

    pub fn len(self) -> usize {
        self.iter().count()
    }

    /// The set as a string of operation letters, for example `"am"`.
    pub fn to_id_string(self) -> String {
        self.iter().map(Operation::id).collect()
    }

    /// Parses the output of `to_id_string`, ignoring unknown letters.
    pub fn from_id_string(s: &str) -> Self {
        Operation::ALL
            .into_iter()
            .filter(|op| s.contains(op.id()))
            .fold(OperationSet(0), |set, op| set.with(op, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_string_round_trips_for_every_subset() {
        for bits in 0..16u8 {
            let set = OperationSet(bits);
            assert_eq!(OperationSet::from_id_string(&set.to_id_string()), set);
        }
    }

    #[test]
    fn id_string_is_ordered_and_ignores_unknown_letters() {
        assert_eq!(OperationSet::ALL.to_id_string(), "asmd");
        assert_eq!(OperationSet::from_id_string("xdz a"), {
            OperationSet(0)
                .with(Operation::Add, true)
                .with(Operation::Divide, true)
        });
        assert!(OperationSet::from_id_string("").is_empty());
    }

    #[test]
    fn with_adds_and_removes() {
        let set = OperationSet::ALL.with(Operation::Multiply, false);
        assert!(!set.contains(Operation::Multiply));
        assert_eq!(set.len(), 3);
        assert!(set.with(Operation::Multiply, true) == OperationSet::ALL);
    }
}
