use gettextrs::gettext;
use i18n_format::i18n_fmt;

use crate::level::Level;
use crate::operation::{Operation, OperationSet};
use crate::round::GameConfig;

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

/// A level with the range of numbers it uses, for example
/// "Level 2: sums up to 20, tables up to 10".
pub fn level_description(level: Level) -> String {
    let sums = level.max_operand(Operation::Add);
    let tables = level.max_operand(Operation::Multiply);
    i18n_fmt! {
        i18n_fmt("Level {}: sums up to {}, tables up to {}", level.number(), sums, tables)
    }
}

/// A duration as minutes and seconds, for example "2:00".
pub fn format_clock(secs: u32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// Describes what was played, for example
/// "Addition, Division · Level 2 · 2:00".
pub fn config_summary(config: &GameConfig) -> String {
    let level = i18n_fmt! { i18n_fmt("Level {}", config.level.number()) };
    format!(
        "{} · {} · {}",
        operations_summary(config.ops),
        level,
        format_clock(config.duration_secs)
    )
}

/// An average time per answer, for example "2.5 s".
pub fn format_average(average_ms: u64) -> String {
    let secs = format!("{:.1}", average_ms as f64 / 1000.0);
    i18n_fmt! { i18n_fmt("{} s", secs) }
}
