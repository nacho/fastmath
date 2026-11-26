use gettextrs::{gettext, pgettext};
use i18n_format::i18n_fmt;

use crate::comparison::{self, ComparisonKind, ComparisonKinds};
use crate::level::Level;
use crate::operation::{Operation, OperationSet};
use crate::round::{GameConfig, Mode};

pub fn operation_name(op: Operation) -> String {
    match op {
        Operation::Add => gettext("Addition"),
        Operation::Subtract => gettext("Subtraction"),
        Operation::Multiply => gettext("Multiplication"),
        Operation::Divide => gettext("Division"),
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
    if mode == Mode::Comparison {
        return level_name(level);
    }
    let number = level.number();
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
/// "Addition, Division · Level 2 · 2:00" or "Decimals · Level 1 · 2:00".
pub fn config_summary(config: &GameConfig) -> String {
    let questions = match config.mode {
        Mode::Arithmetic => operations_summary(config.ops),
        Mode::Comparison => comparison_kinds_summary(config.kinds),
    };
    format!(
        "{} · {} · {}",
        questions,
        level_name(config.level),
        format_clock(config.duration_secs)
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
