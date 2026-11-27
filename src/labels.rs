use gettextrs::{gettext, pgettext};
use i18n_format::i18n_fmt;

use crate::comparison::{self, ComparisonKind, ComparisonKinds};
use crate::fraction::{Ask, FractionQuestion, Shape};
use crate::generator::Question;
use crate::level::Level;
use crate::operation::{Operation, OperationSet};
use crate::practice::{FactStats, OperationStats};
use crate::round::{GameConfig, Mode};

pub fn operation_name(op: Operation) -> String {
    match op {
        Operation::Add => gettext("Addition"),
        Operation::Subtract => gettext("Subtraction"),
        Operation::Multiply => gettext("Multiplication"),
        Operation::Divide => gettext("Division"),
    }
}

/// How the player does with an operation, for example "Multiplication: 86%
/// right, 2.1 s on average, 12 weak facts".
pub fn operation_progress(op: Operation, stats: &OperationStats) -> String {
    let name = operation_name(op);
    if stats.answered == 0 {
        return i18n_fmt! {
            // Translators: {} is the name of an operation, for example
            // "Multiplication: no answers yet".
            i18n_fmt("{}: no answers yet", name)
        };
    }
    let percent = (100 * stats.right + stats.answered / 2) / stats.answered;
    let weak = stats.weak;
    match stats.average_ms {
        Some(ms) => {
            let average = format_average(ms);
            i18n_fmt! {
                // Translators: How the player does with an operation. The
                // first {} is its name, the second {} the percentage of right
                // answers, the third {} the average time of a right answer,
                // for example "2.1 s", and the fourth {} the number of weak
                // facts, which picks the plural form, for example
                // "Multiplication: 86% right, 2.1 s on average, 12 weak facts".
                i18n_nfmt("{}: {}% right, {} on average, {} weak fact", "{}: {}% right, {} on average, {} weak facts", weak, name, percent, average, weak)
            }
        }
        None => i18n_fmt! {
            // Translators: How the player does with an operation with no
            // right answers yet. The first {} is its name, the second {} the
            // percentage of right answers and the third {} the number of weak
            // facts, which picks the plural form, for example
            // "Multiplication: 0% right, 3 weak facts".
            i18n_nfmt("{}: {}% right, {} weak fact", "{}: {}% right, {} weak facts", weak, name, percent, weak)
        },
    }
}

/// How the player does with a fact, for example "7 × 8: 3 right out of 5,
/// 2.4 s on average".
pub fn fact_progress(question: &Question, stats: &FactStats) -> String {
    let fact = question.text();
    let (right, answered) = (stats.right, stats.answered);
    match stats.average_ms {
        Some(ms) => {
            let average = format_average(ms);
            i18n_fmt! {
                // Translators: How the player does with a fact. The first {}
                // is the fact, the second {} the number of right answers,
                // the third {} the number of answers kept, at most 5, which
                // picks the plural form, and the fourth {} the average time
                // of a right answer, for example "7 × 8: 3 right out of 5,
                // 2.4 s on average".
                i18n_nfmt("{}: {} right out of {}, {} on average", "{}: {} right out of {}, {} on average", answered, fact, right, answered, average)
            }
        }
        None => i18n_fmt! {
            // Translators: How the player does with a fact never answered
            // right. The first {} is the fact, the second {} the number of
            // right answers, 0, and the third {} the number of answers kept,
            // at most 5, which picks the plural form, for example "7 × 8: 0
            // right out of 2".
            i18n_nfmt("{}: {} right out of {}", "{}: {} right out of {}", answered, fact, right, answered)
        },
    }
}

pub fn operations_summary(ops: OperationSet) -> String {
    if ops == OperationSet::ALL {
        return gettext("All operations");
    }
    ops.iter()
        .map(operation_name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn comparison_kind_name(kind: ComparisonKind) -> String {
    match kind {
        ComparisonKind::WholeNumbers => gettext("Whole Numbers"),
        ComparisonKind::Decimals => gettext("Decimals"),
        ComparisonKind::Fractions => gettext("Fractions"),
        ComparisonKind::Calculations => gettext("Calculations"),
    }
}

pub fn comparison_kinds_summary(kinds: ComparisonKinds) -> String {
    if kinds == ComparisonKinds::ALL {
        return gettext("All comparisons");
    }
    kinds
        .iter()
        .map(comparison_kind_name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// A level number, for example "Level 2".
pub fn level_name(level: Level) -> String {
    i18n_fmt! {
        // Translators: {} is the difficulty level number, from 1 to 5, for
        // example "Level 2".
        i18n_fmt("Level {}", level.number())
    }
}

/// A level with the numbers it uses, for example
/// "Level 2: sums up to 20, tables up to 10". Comparison levels depend on
/// the kind, see `comparison_kind_description`.
pub fn level_description(mode: Mode, level: Level) -> String {
    let number = level.number();
    match mode {
        Mode::Arithmetic => (),
        Mode::Comparison => return level_name(level),
        Mode::Fractions => {
            return match number {
                1 => i18n_fmt! {
                    // Translators: This describes a difficulty level of the
                    // fractions mode. {} is the level number, for example
                    // "Level 1: halves, thirds and quarters".
                    i18n_fmt("Level {}: halves, thirds and quarters", number)
                },
                2 => i18n_fmt! {
                    // Translators: This describes a difficulty level of the
                    // fractions mode, with fractions like 5/12. {} is the
                    // level number, for example "Level 2: up to twelfths".
                    i18n_fmt("Level {}: up to twelfths", number)
                },
                3 => i18n_fmt! {
                    // Translators: This describes a difficulty level of the
                    // fractions mode, asking which fraction was eaten or is
                    // left. {} is the level number, for example "Level 3:
                    // eaten and left".
                    i18n_fmt("Level {}: eaten and left", number)
                },
                4 => i18n_fmt! {
                    // Translators: This describes a difficulty level of the
                    // fractions mode, where 4/8 must be answered as 1/2. {}
                    // is the level number, for example "Level 4: simplest
                    // form".
                    i18n_fmt("Level {}: simplest form", number)
                },
                _ => i18n_fmt! {
                    // Translators: This describes a difficulty level of the
                    // fractions mode, with answers like 7/4. {} is the level
                    // number, for example "Level 5: more than a whole".
                    i18n_fmt("Level {}: more than a whole", number)
                },
            };
        }
    }
    let sums = level.max_operand(Operation::Add);
    let tables = level.max_operand(Operation::Multiply);
    i18n_fmt! {
        // Translators: This describes a difficulty level. The first {} is
        // the level number, from 1 to 5, the second {} is the largest number
        // used in additions and subtractions, and the third {} is the largest
        // number used in the multiplication tables and divisions, for
        // example "Level 2: sums up to 20, tables up to 10".
        i18n_fmt("Level {}: sums up to {}, tables up to {}", number, sums, tables)
    }
}

/// What a kind of comparison asks at a level, for example "Tenths and
/// hundredths".
pub fn comparison_kind_description(kind: ComparisonKind, level: Level) -> String {
    let number = level.number();
    match kind {
        ComparisonKind::WholeNumbers if comparison::has_negatives(level) => {
            let max = comparison::whole_max(level);
            i18n_fmt! {
                // Translators: This describes the whole numbers compared at a
                // difficulty level. Both {} are the same number, for example
                // "Negative numbers, from −100 to 100". The first one comes
                // after a minus sign.
                i18n_fmt("Negative numbers, from −{} to {}", max, max)
            }
        }
        ComparisonKind::WholeNumbers => {
            let max = comparison::whole_max(level);
            i18n_fmt! {
                // Translators: This describes the whole numbers compared at a
                // difficulty level. {} is the largest number, for example
                // "Up to 20".
                i18n_fmt("Up to {}", max)
            }
        }
        ComparisonKind::Decimals => match number {
            1 => gettext("Tenths"),
            2 => gettext("Tenths and hundredths"),
            3 => gettext("Thousandths"),
            4 => gettext("Negative numbers with hundredths"),
            _ => gettext("Negative numbers with thousandths"),
        },
        ComparisonKind::Fractions => match number {
            1 => gettext("The same denominator"),
            2 => gettext("The same numerator"),
            3 => gettext("Equivalent fractions"),
            4 => gettext("Different denominators"),
            _ => gettext("Improper fractions and mixed numbers"),
        },
        ComparisonKind::Calculations => {
            let operands = comparison::calculation_level(level);
            let sums = operands.max_operand(Operation::Add);
            let tables = operands.max_operand(Operation::Multiply);
            match number {
                1 => i18n_fmt! {
                    // Translators: This describes the calculations compared
                    // at a difficulty level, for example "7 + 2 ? 8". {} is
                    // the largest number added or subtracted, for example
                    // "A sum or a difference and a number, up to 10".
                    i18n_fmt("A sum or a difference and a number, up to {}", sums)
                },
                2 => i18n_fmt! {
                    // Translators: This describes the calculations compared
                    // at a difficulty level, for example "7 × 8 ? 54". The
                    // first {} is the largest number added or subtracted and
                    // the second {} the largest multiplication table, for
                    // example "A calculation and a number, sums up to 20,
                    // tables up to 10".
                    i18n_fmt("A calculation and a number, sums up to {}, tables up to {}", sums, tables)
                },
                _ => i18n_fmt! {
                    // Translators: This describes the calculations compared
                    // at a difficulty level, for example "6 × 9 ? 50 + 4".
                    // The first {} is the largest number added or subtracted
                    // and the second {} the largest multiplication table, for
                    // example "Two calculations, sums up to 50, tables up to
                    // 12".
                    i18n_fmt("Two calculations, sums up to {}, tables up to {}", sums, tables)
                },
            }
        }
    }
}

/// The question of a picture, for example "How much was eaten?".
pub fn fraction_question(question: &FractionQuestion) -> String {
    match (question.picture.shape, question.ask) {
        (Shape::Grid { .. }, Ask::Eaten) => gettext("What fraction is coloured?"),
        (Shape::Grid { .. }, Ask::Left) => gettext("What fraction is not coloured?"),
        (_, Ask::Eaten) => gettext("How much was eaten?"),
        (_, Ask::Left) => gettext("How much is left?"),
    }
}

pub fn decimal_separator() -> String {
    // Translators: The decimal separator, as in 8.25. Translate it as ","
    // if your language writes 8,25.
    pgettext("decimal separator", ".")
}

/// A duration as minutes and seconds, for example "2:00".
pub fn format_clock(secs: u32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// Describes what was played, for example
/// "Addition, Division · Level 2 · 2:00", "Decimals · Level 1 · 2:00" or
/// "Addition · Level 2 · Practice".
pub fn config_summary(config: &GameConfig) -> String {
    let questions = match config.mode {
        Mode::Arithmetic => operations_summary(config.ops),
        Mode::Comparison => comparison_kinds_summary(config.kinds),
        Mode::Fractions => gettext("Fractions"),
    };
    format!(
        "{} · {} · {}",
        questions,
        level_name(config.level),
        if config.is_practice() {
            // Translators: Shown instead of the time limit for a practice
            // round, which has no clock.
            gettext("Practice")
        } else {
            format_clock(config.duration_secs)
        }
    )
}

/// An average time per answer, for example "2.5 s".
pub fn format_average(average_ms: u64) -> String {
    let secs = format!("{:.1}", average_ms as f64 / 1000.0);
    i18n_fmt! {
        // Translators: {} is a number of seconds with one decimal and "s" is
        // the abbreviation of seconds, for example "2.5 s".
        i18n_fmt("{} s", secs)
    }
}
