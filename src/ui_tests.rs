// Tests that need a display. They build the widgets and drive a round, but
// never show a window. Run them from a desktop session with:
//
//   glib-compile-schemas <dir>   # a directory with the gschema.xml in it
//   GSETTINGS_BACKEND=memory GSETTINGS_SCHEMA_DIR=<dir> \
//     cargo test -- --ignored --test-threads=1
//
// Everything runs in one test: the widgets start clocks on the default main
// context, which only one thread can own.

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::config;
use crate::game_view::GameView;
use crate::level::Level;
use crate::operation::{Operation, OperationSet};
use crate::results_view::ResultsView;
use crate::round::{GameConfig, QUESTIONS_PER_ROUND};
use crate::start_view::StartView;

fn config() -> GameConfig {
    GameConfig {
        ops: OperationSet::ALL,
        level: Level::new(1),
        duration_secs: 120,
    }
}

fn answer(view: &GameView, text: &str) {
    let entry = &view.imp().answer_entry;
    entry.set_text(text);
    entry.emit_by_name::<()>("activate", &[]);
}

fn current_answer(view: &GameView) -> u32 {
    let round = view.imp().round.borrow();
    round.as_ref().unwrap().current().unwrap().answer
}

fn watch_finished(view: &GameView) -> Rc<Cell<u32>> {
    let finished = Rc::new(Cell::new(0));
    view.connect_finished({
        let finished = finished.clone();
        move |_| finished.set(finished.get() + 1)
    });
    finished
}

fn start_view_keeps_the_choices(settings: &gio::Settings) {
    let view: StartView = glib::Object::new();
    let imp = view.imp();

    assert_eq!(view.game_config().ops, OperationSet::ALL);
    assert!(imp.start_button.is_sensitive());

    imp.subtract_row.set_active(false);
    imp.divide_row.set_active(false);
    let ops = view.game_config().ops;
    assert!(ops.contains(Operation::Add) && ops.contains(Operation::Multiply));
    assert!(!ops.contains(Operation::Subtract) && !ops.contains(Operation::Divide));
    assert_eq!(settings.string("operations"), "am");

    // Without operations there is nothing to play, and the last choice that
    // had some is kept.
    imp.add_row.set_active(false);
    assert_eq!(settings.string("operations"), "m");
    imp.multiply_row.set_active(false);
    assert!(!imp.start_button.is_sensitive());
    assert_eq!(settings.string("operations"), "m");

    imp.all_button.emit_by_name::<()>("clicked", &[]);
    assert_eq!(view.game_config().ops, OperationSet::ALL);
    assert!(imp.start_button.is_sensitive());

    imp.level_row.set_selected(3);
    imp.duration_row.set_value(45.0);
    let config = view.game_config();
    assert_eq!(config.level.number(), 4);
    assert_eq!(config.duration_secs, 45);
    assert_eq!(settings.uint("level"), 4);
    assert_eq!(settings.uint("duration-secs"), 45);

    // A new view starts from the saved choices.
    let again: StartView = glib::Object::new();
    assert_eq!(again.game_config().level.number(), 4);
    assert_eq!(again.imp().level_row.selected(), 3);
    assert_eq!(again.game_config().duration_secs, 45);
}

fn a_full_round_ends_after_60_answers() {
    let view = GameView::new(&config());
    let finished = watch_finished(&view);
    let imp = view.imp();
    view.start_clock();

    // Only digits can be typed.
    imp.answer_entry.set_text("1a2 3");
    assert_eq!(imp.answer_entry.text(), "123");
    imp.answer_entry.set_text("");

    // Nothing happens without an answer.
    imp.answer_entry.emit_by_name::<()>("activate", &[]);
    assert_eq!(imp.round.borrow().as_ref().unwrap().answered(), 0);

    let wrong = current_answer(&view) + 1;
    answer(&view, &wrong.to_string());
    assert_eq!(imp.answer_entry.text(), "");
    assert_eq!(imp.counter_label.text(), "2 / 60");

    for _ in 1..QUESTIONS_PER_ROUND {
        assert_eq!(finished.get(), 0);
        let correct = current_answer(&view);
        answer(&view, &correct.to_string());
    }

    assert_eq!(finished.get(), 1);
    let result = view.summary().result;
    assert_eq!(result.wrong, 1);
    assert_eq!(result.correct, QUESTIONS_PER_ROUND - 1);
    assert_eq!(result.unanswered, 0);

    // Answers after the end are ignored.
    answer(&view, "1");
    assert_eq!(finished.get(), 1);
}

fn a_round_ends_when_the_time_is_up() {
    let view = GameView::new(&config());
    let finished = watch_finished(&view);
    view.start_clock();

    for _ in 0..3 {
        let correct = current_answer(&view);
        answer(&view, &correct.to_string());
    }
    assert_eq!(view.tick(), glib::ControlFlow::Continue);
    assert_eq!(finished.get(), 0);

    view.imp()
        .started_at
        .set(Some(Instant::now() - Duration::from_secs(121)));
    assert_eq!(view.tick(), glib::ControlFlow::Break);
    assert_eq!(finished.get(), 1);
    assert_eq!(view.imp().time_label.text(), "0:00");

    let result = view.summary().result;
    assert_eq!(result.correct, 3);
    assert_eq!(result.unanswered, QUESTIONS_PER_ROUND - 3);
    assert!(result.average_time.is_some());
}

fn results_keep_a_leaderboard() {
    let config = config();
    let view = GameView::new(&config);
    view.start_clock();
    for _ in 0..4 {
        let correct = current_answer(&view);
        answer(&view, &correct.to_string());
    }
    view.imp()
        .started_at
        .set(Some(Instant::now() - Duration::from_secs(121)));
    view.tick();

    let results = ResultsView::new(&view.summary());
    let imp = results.imp();
    assert_eq!(imp.correct_label.text(), "4");
    assert_eq!(imp.unanswered_label.text(), "56");
    assert!(imp.save_group.is_visible());
    assert!(imp.rows.borrow().is_empty());

    imp.name_row.set_text("Ana");
    imp.name_row.emit_by_name::<()>("apply", &[]);
    assert!(!imp.save_group.is_visible());
    assert_eq!(imp.rows.borrow().len(), 1);
    assert_eq!(results.config(), config);

    // A second view of the same configuration finds the saved score.
    let again = ResultsView::new(&view.summary());
    assert_eq!(again.imp().rows.borrow().len(), 1);
    assert!(again.imp().save_group.is_visible());
}

fn the_shortcuts_dialog_loads() {
    adw::ShortcutsDialog::ensure_type();
    adw::ShortcutsSection::ensure_type();
    adw::ShortcutsItem::ensure_type();
    let builder = gtk::Builder::from_resource("/io/github/nacho/fastmath/ui/shortcuts_dialog.ui");
    assert!(
        builder
            .object::<adw::ShortcutsDialog>("shortcuts_dialog")
            .is_some()
    );
}

#[test]
#[ignore = "needs a display"]
fn ui_smoke() {
    let data_dir = std::env::temp_dir().join(format!("fastmath-ui-test-{}", std::process::id()));
    // SAFETY: nothing else is running yet.
    unsafe { std::env::set_var("XDG_DATA_HOME", &data_dir) };

    gtk::init().expect("a display is needed");
    adw::init().expect("libadwaita");
    gio::resources_register_include!("resources.gresource").expect("resources");

    let settings = gio::Settings::new(config::APPLICATION_ID);
    settings.set_boolean("sound-effects", false).unwrap();

    start_view_keeps_the_choices(&settings);
    a_full_round_ends_after_60_answers();
    a_round_ends_when_the_time_is_up();
    results_keep_a_leaderboard();
    the_shortcuts_dialog_loads();

    let _ = std::fs::remove_dir_all(&data_dir);
}
