use std::marker::PhantomData;

/// Something the player can switch on or off, such as an operation. Each
/// choice is stored in settings and file names as one letter.
pub trait Choice: Copy + PartialEq + 'static {
    /// Every choice, in the order of their letters in `to_id_string`. At
    /// most 8.
    const ALL: &'static [Self];

    fn id(self) -> char;
}

/// A set of choices, stored as a bitmask.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ChoiceSet<T> {
    bits: u8,
    choices: PhantomData<T>,
}

impl<T: Choice> ChoiceSet<T> {
    pub const ALL: Self = ChoiceSet {
        bits: ((1u16 << T::ALL.len()) - 1) as u8,
        choices: PhantomData,
    };

    fn bit(choice: T) -> u8 {
        let index = T::ALL
            .iter()
            .position(|c| *c == choice)
            .expect("every choice is in ALL");
        1 << index
    }

    pub fn is_empty(self) -> bool {
        self.bits == 0
    }

    pub fn contains(self, choice: T) -> bool {
        self.bits & Self::bit(choice) != 0
    }

    pub fn with(self, choice: T, enabled: bool) -> Self {
        let bits = if enabled {
            self.bits | Self::bit(choice)
        } else {
            self.bits & !Self::bit(choice)
        };
        ChoiceSet { bits, ..self }
    }

    pub fn iter(self) -> impl Iterator<Item = T> {
        T::ALL.iter().copied().filter(move |c| self.contains(*c))
    }

    pub fn len(self) -> usize {
        self.iter().count()
    }

    /// The set as a string of letters, for example `"am"`.
    pub fn to_id_string(self) -> String {
        self.iter().map(T::id).collect()
    }

    /// Parses the output of `to_id_string`, ignoring unknown letters.
    pub fn from_id_string(s: &str) -> Self {
        T::ALL.iter().filter(|c| s.contains(c.id())).fold(
            ChoiceSet {
                bits: 0,
                choices: PhantomData,
            },
            |set, c| set.with(*c, true),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
    enum Fruit {
        Apple,
        Banana,
        Cherry,
    }

    impl Choice for Fruit {
        const ALL: &'static [Self] = &[Fruit::Apple, Fruit::Banana, Fruit::Cherry];

        fn id(self) -> char {
            match self {
                Fruit::Apple => 'a',
                Fruit::Banana => 'b',
                Fruit::Cherry => 'c',
            }
        }
    }

    type Fruits = ChoiceSet<Fruit>;

    /// Every subset of the fruits.
    fn subsets() -> Vec<Fruits> {
        (0..8u8)
            .map(|bits| {
                Fruit::ALL
                    .iter()
                    .enumerate()
                    .fold(Fruits::from_id_string(""), |set, (i, f)| {
                        set.with(*f, bits & (1 << i) != 0)
                    })
            })
            .collect()
    }

    #[test]
    fn id_string_round_trips_for_every_subset() {
        let subsets = subsets();
        for set in &subsets {
            assert_eq!(Fruits::from_id_string(&set.to_id_string()), *set);
        }
        // They are all different.
        for (i, a) in subsets.iter().enumerate() {
            assert!(subsets[i + 1..].iter().all(|b| a != b));
        }
    }

    #[test]
    fn id_string_is_ordered_and_ignores_unknown_letters() {
        assert_eq!(Fruits::ALL.to_id_string(), "abc");
        assert_eq!(
            Fruits::from_id_string("xcz a"),
            Fruits::from_id_string("")
                .with(Fruit::Apple, true)
                .with(Fruit::Cherry, true)
        );
        assert!(Fruits::from_id_string("").is_empty());
    }

    #[test]
    fn with_adds_and_removes() {
        let set = Fruits::ALL.with(Fruit::Banana, false);
        assert!(!set.contains(Fruit::Banana));
        assert_eq!(set.len(), 2);
        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            [Fruit::Apple, Fruit::Cherry]
        );
        assert!(set.with(Fruit::Banana, true) == Fruits::ALL);
    }
}
