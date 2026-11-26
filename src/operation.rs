use crate::choice::{Choice, ChoiceSet};

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
}

impl Choice for Operation {
    const ALL: &'static [Self] = &Operation::ALL;

    /// The letter used to store the operation in settings and file names.
    fn id(self) -> char {
        match self {
            Operation::Add => 'a',
            Operation::Subtract => 's',
            Operation::Multiply => 'm',
            Operation::Divide => 'd',
        }
    }
}

/// A set of operations, stored as `"asmd"` letters.
pub type OperationSet = ChoiceSet<Operation>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operations_use_their_letters() {
        assert_eq!(OperationSet::ALL.to_id_string(), "asmd");
        let set = OperationSet::from_id_string("dm");
        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            [Operation::Multiply, Operation::Divide]
        );
    }
}
