use gettextrs::{gettext, pgettext};
use i18n_format::i18n_fmt;

use crate::comparison::WHOLE_MAX;
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

/// A level with the numbers it uses, for example
/// "Level 2: sums up to 20, tables up to 10".
pub fn level_description(mode: Mode, level: Level) -> String {
    let number = level.number();
    if mode == Mode::Comparison {
        return match number {
            1 => i18n_fmt! {
                // Translators: This describes a difficulty level of the
                // comparisons. The first {} is the level number and the
                // second {} the largest number, for example "Level 1: whole
                // numbers up to 100".
                i18n_fmt("Level {}: whole numbers up to {}", number, WHOLE_MAX)
            },
            2 => i18n_fmt! {
                // Translators: This describes a difficulty level of the
                // comparisons, with numbers like 8.3 and 8.25. {} is the
                // level number, for example "Level 2: tenths and hundredths".
                i18n_fmt("Level {}: tenths and hundredths", number)
            },
            3 => i18n_fmt! {
                // Translators: This describes a difficulty level of the
                // comparisons, with numbers like 2.045 and −2.5. {} is the
                // level number, for example "Level 3: thousandths and
                // negative numbers".
                i18n_fmt("Level {}: thousandths and negative numbers", number)
            },
            4 => i18n_fmt! {
                // Translators: This describes a difficulty level of the
                // comparisons, with questions like "7 × 8 ? 54". {} is the
                // level number, for example "Level 4: a calculation and a
                // number".
                i18n_fmt("Level {}: a calculation and a number", number)
            },
            _ => i18n_fmt! {
                // Translators: This describes a difficulty level of the
                // comparisons, with questions like "6 × 9 ? 50 + 4". {} is
                // the level number, for example "Level 5: two calculations".
                i18n_fmt("Level {}: two calculations", number)
            },
        };
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
/// "Addition, Division · Level 2 · 2:00" or "Comparison · Level 1 · 2:00".
pub fn config_summary(config: &GameConfig) -> String {
    let level = i18n_fmt! {
        // Translators: {} is the difficulty level number, from 1 to 5, for
        // example "Level 2".
        i18n_fmt("Level {}", config.level.number())
    };
    let questions = match config.mode {
        Mode::Arithmetic => operations_summary(config.ops),
        Mode::Comparison => gettext("Comparison"),
    };
    format!(
        "{} · {} · {}",
        questions,
        level,
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
