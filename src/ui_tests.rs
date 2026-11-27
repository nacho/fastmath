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
use std::cmp::Ordering;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::application::FastmathApplication;
use crate::comparison::{ComparisonKind, ComparisonKinds};
use crate::config;
use crate::fraction::{Fraction, Picture, Shape};
use crate::fraction_label::FractionLabel;
use crate::fraction_picture::FractionPicture;
use crate::game_view::{GameSummary, GameView};
use crate::level::Level;
use crate::operation::{Operation, OperationSet};
use crate::practice::FactHistory;
use crate::progress_view::ProgressView;
use crate::results_view::ResultsView;
use crate::round::{Answer, GameConfig, Mode, PRACTICE_QUESTIONS, Prompt, QUESTIONS_PER_ROUND};
use crate::start_view::StartView;
use crate::window::FastmathWindow;

fn config() -> GameConfig {
    GameConfig {
        mode: Mode::Arithmetic,
        ops: OperationSet::ALL,
        kinds: ComparisonKinds::ALL,
        practice: false,
        level: Level::new(1),
        duration_secs: 120,
    }
}

fn answer(view: &GameView, text: &str) {
    let entry = &view.imp().answer_entry;
    entry.set_text(text);
    entry.emit_by_name::<()>("activate", &[]);
}

fn current_answer(view: &GameView) -> Answer {
    let round = view.imp().round.borrow();
    round.as_ref().unwrap().current().unwrap().answer()
}

fn current_number(view: &GameView) -> u32 {
    let Answer::Number(n) = current_answer(view) else {
        panic!("not an arithmetic question");
    };
    n
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

    // Comparisons need no operations, and keep the level.
    imp.add_row.set_active(false);
    imp.subtract_row.set_active(false);
    imp.multiply_row.set_active(false);
    imp.divide_row.set_active(false);
    assert!(!imp.start_button.is_sensitive());
    let arithmetic_level = level_subtitle(&view);
    imp.mode_group.set_active_name(Some("comparison"));
    assert_eq!(settings.string("mode"), "comparison");
    assert_eq!(view.game_config().mode, Mode::Comparison);
    assert!(!imp.operations_group.get_visible());
    assert!(imp.start_button.is_sensitive());
    assert_eq!(imp.level_row.selected(), 3);
    assert_eq!(settings.uint("level"), 4);
    assert_ne!(level_subtitle(&view), arithmetic_level);

    let again: StartView = glib::Object::new();
    assert_eq!(again.game_config().mode, Mode::Comparison);
    assert_eq!(again.imp().level_row.selected(), 3);

    // The comparisons are chosen like the operations, and their rows
    // describe the selected level.
    assert!(imp.comparisons_group.get_visible());
    assert_eq!(view.game_config().kinds, ComparisonKinds::ALL);
    assert_eq!(level_subtitle(&view), "Level 4");
    let level_4 = imp.decimals_row.subtitle().unwrap();
    imp.level_row.set_selected(0);
    assert_ne!(imp.decimals_row.subtitle().unwrap(), level_4);
    imp.level_row.set_selected(3);
    imp.whole_numbers_row.set_active(false);
    imp.fractions_row.set_active(false);
    imp.calculations_row.set_active(false);
    assert_eq!(settings.string("comparisons"), "d");
    imp.decimals_row.set_active(false);
    assert!(!imp.start_button.is_sensitive());
    assert_eq!(settings.string("comparisons"), "d");
    imp.all_comparisons_button
        .emit_by_name::<()>("clicked", &[]);
    assert!(imp.start_button.is_sensitive());
    imp.calculations_row.set_active(false);
    let kinds = view.game_config().kinds;
    assert!(!kinds.contains(ComparisonKind::Calculations));
    assert_eq!(settings.string("comparisons"), "wdf");
    imp.all_comparisons_button
        .emit_by_name::<()>("clicked", &[]);

    // Fractions need no choices.
    imp.mode_group.set_active_name(Some("fractions"));
    assert_eq!(view.game_config().mode, Mode::Fractions);
    assert!(!imp.operations_group.get_visible());
    assert!(!imp.comparisons_group.get_visible());
    imp.add_row.set_active(false);
    imp.subtract_row.set_active(false);
    imp.multiply_row.set_active(false);
    imp.divide_row.set_active(false);
    assert!(imp.start_button.is_sensitive());
    assert_eq!(imp.level_row.selected(), 3);

    imp.mode_group.set_active_name(Some("arithmetic"));
    assert!(imp.operations_group.get_visible());
    assert!(!imp.comparisons_group.get_visible());
    assert!(!imp.start_button.is_sensitive());
    imp.all_button.emit_by_name::<()>("clicked", &[]);

    // Practice is for arithmetic only, and has no time limit.
    assert!(imp.practice_row.get_visible());
    assert!(imp.duration_row.is_sensitive());
    imp.practice_row.set_active(true);
    assert!(settings.boolean("practice"));
    assert!(view.game_config().is_practice());
    assert!(!imp.duration_row.is_sensitive());
    imp.mode_group.set_active_name(Some("comparison"));
    assert!(!imp.practice_row.get_visible());
    assert!(imp.duration_row.is_sensitive());
    assert!(!view.game_config().is_practice());
    imp.mode_group.set_active_name(Some("arithmetic"));
    assert!(!imp.duration_row.is_sensitive());
    imp.practice_row.set_active(false);
    assert!(!settings.boolean("practice"));
}

fn a_practice_round_has_no_clock_and_20_questions() {
    let config = GameConfig {
        practice: true,
        ..config()
    };
    let view = GameView::new(&config);
    let finished = watch_finished(&view);
    let imp = view.imp();
    view.start_clock();
    assert!(!imp.time_label.get_visible());
    assert_eq!(imp.counter_label.text(), "1 / 20");

    // Time never runs out.
    imp.started_at
        .set(Some(Instant::now() - Duration::from_secs(3600)));
    assert_eq!(view.tick(), glib::ControlFlow::Continue);
    assert_eq!(finished.get(), 0);

    // A wrong answer shows the right one until the next answer.
    assert!(imp.feedback_label.get_visible());
    assert_eq!(imp.feedback_label.opacity(), 0.0);
    let Some(Prompt::Arithmetic(missed)) = imp.round.borrow().as_ref().unwrap().current().copied()
    else {
        panic!("not an arithmetic question");
    };
    answer(&view, &(missed.answer + 1).to_string());
    assert_eq!(imp.feedback_label.opacity(), 1.0);
    assert_eq!(imp.feedback_label.text(), missed.solution());

    for _ in 1..PRACTICE_QUESTIONS {
        assert_eq!(finished.get(), 0);
        let correct = current_number(&view);
        answer(&view, &correct.to_string());
        assert_eq!(imp.feedback_label.opacity(), 0.0);
    }
    assert_eq!(finished.get(), 1);
    let summary = view.summary();
    assert_eq!(summary.result.correct, PRACTICE_QUESTIONS - 1);
    assert_eq!(summary.result.unanswered, 0);

    // No leaderboard and no name entry, but the facts to practise.
    let results = ResultsView::new(&summary);
    let imp = results.imp();
    assert!(!imp.leaderboard_group.get_visible());
    assert!(!imp.save_group.get_visible());
    assert!(imp.config_label.text().ends_with("Practice"));
    assert!(imp.missed_group.get_visible());
    let row = imp
        .missed_group
        .row(0)
        .and_downcast::<adw::ActionRow>()
        .unwrap();
    assert_eq!(row.title(), missed.solution());
    assert!(imp.missed_group.row(1).is_none());

    let perfect = GameSummary {
        missed: Vec::new(),
        ..summary
    };
    let results = ResultsView::new(&perfect);
    assert_eq!(
        results.imp().missed_group.description().unwrap(),
        "No mistakes"
    );
}

fn level_subtitle(view: &StartView) -> String {
    let row = &view.imp().level_row;
    row.selected_item()
        .and_downcast::<gtk::StringObject>()
        .unwrap()
        .string()
        .into()
}

/// The widgets showing the current question.
fn question_parts(view: &GameView) -> Vec<gtk::Widget> {
    let question_box = &view.imp().question_box;
    std::iter::successors(question_box.first_child(), |w| w.next_sibling()).collect()
}

/// Plays a comparison round of `kinds`, checking that fractions are shown
/// stacked when `fractions` and as text otherwise.
fn a_comparison_round_is_answered_with_buttons(kinds: ComparisonKinds, fractions: bool) {
    let config = GameConfig {
        mode: Mode::Comparison,
        kinds,
        level: Level::new(2),
        ..config()
    };
    let view = GameView::new(&config);
    // Buttons only find their actions inside a window. It is never shown.
    let navigation = adw::NavigationView::new();
    navigation.push(&view);
    let window = gtk::Window::new();
    window.set_child(Some(&navigation));
    let finished = watch_finished(&view);
    let imp = view.imp();
    assert!(!imp.answer_entry.get_visible());
    assert!(imp.compare_box.get_visible());
    view.start_clock();

    let check_parts = || {
        let parts = question_parts(&view);
        assert_eq!(parts.len(), 3);
        let middle = parts[1].downcast_ref::<gtk::Label>().unwrap();
        assert_eq!(middle.text(), "?");
        for side in [&parts[0], &parts[2]] {
            assert_eq!(side.is::<FractionLabel>(), fractions);
        }
    };
    check_parts();

    let button = |order| match order {
        Ordering::Less => &imp.less_button,
        Ordering::Equal => &imp.equal_button,
        Ordering::Greater => &imp.greater_button,
    };
    let Answer::Order(order) = current_answer(&view) else {
        panic!("not a comparison");
    };
    let wrong = if order == Ordering::Equal {
        Ordering::Greater
    } else {
        Ordering::Equal
    };
    button(wrong).emit_clicked();
    assert!(button(wrong).has_css_class("error"));
    assert_eq!(imp.counter_label.text(), "2 / 60");

    for _ in 1..QUESTIONS_PER_ROUND {
        assert_eq!(finished.get(), 0);
        let Answer::Order(order) = current_answer(&view) else {
            panic!("not a comparison");
        };
        check_parts();
        button(order).emit_clicked();
    }

    assert_eq!(finished.get(), 1);
    let result = view.summary().result;
    assert_eq!(result.wrong, 1);
    assert_eq!(result.correct, QUESTIONS_PER_ROUND - 1);
    window.destroy();
}

fn a_full_round_ends_after_60_answers() {
    let view = GameView::new(&config());
    let finished = watch_finished(&view);
    let imp = view.imp();
    view.start_clock();

    // Only practice shows the right answer after a wrong one.
    assert!(!imp.feedback_label.get_visible());

    // Only digits can be typed.
    imp.answer_entry.set_text("1a2 3");
    assert_eq!(imp.answer_entry.text(), "123");
    imp.answer_entry.set_text("");

    // Nothing happens without an answer.
    imp.answer_entry.emit_by_name::<()>("activate", &[]);
    assert_eq!(imp.round.borrow().as_ref().unwrap().answered(), 0);

    let wrong = current_number(&view) + 1;
    answer(&view, &wrong.to_string());
    assert_eq!(imp.answer_entry.text(), "");
    assert_eq!(imp.counter_label.text(), "2 / 60");

    for _ in 1..QUESTIONS_PER_ROUND {
        assert_eq!(finished.get(), 0);
        let correct = current_number(&view);
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
        let correct = current_number(&view);
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
        let correct = current_number(&view);
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

    imp.name_row.set_text(" ");
    assert!(!imp.save_button.is_sensitive());
    imp.name_row.set_text("Ana");
    assert!(imp.save_button.is_sensitive());
    imp.save_button.emit_clicked();
    assert!(!imp.save_group.is_visible());
    assert_eq!(imp.rows.borrow().len(), 1);
    assert_eq!(results.config(), config);

    // A second view of the same configuration finds the saved score.
    let again = ResultsView::new(&view.summary());
    let imp = again.imp();
    assert_eq!(imp.rows.borrow().len(), 1);
    assert!(imp.save_group.is_visible());

    // The remembered name can be saved without editing it, with the button
    // or with Enter.
    assert_eq!(imp.name_row.text(), "Ana");
    assert!(imp.save_button.is_sensitive());
    imp.name_row.emit_by_name::<()>("entry-activated", &[]);
    assert!(!imp.save_group.is_visible());
    assert_eq!(imp.rows.borrow().len(), 2);
}

fn a_fractions_round_is_answered_in_the_entries() {
    let config = GameConfig {
        mode: Mode::Fractions,
        level: Level::new(2),
        ..config()
    };
    let view = GameView::new(&config);
    // Focus only moves inside a window. It is never shown.
    let navigation = adw::NavigationView::new();
    navigation.push(&view);
    let window = gtk::Window::new();
    window.set_child(Some(&navigation));
    let finished = watch_finished(&view);
    let imp = view.imp();
    assert!(imp.fraction_box.get_visible());
    assert!(!imp.answer_entry.get_visible() && !imp.compare_box.get_visible());
    view.start_clock();

    let has_focus = |entry: &gtk::Entry| {
        GtkWindowExt::focus(&window)
            .is_some_and(|w| w.is_ancestor(entry) || w == *entry.upcast_ref::<gtk::Widget>())
    };
    let parts = question_parts(&view);
    assert_eq!(parts.len(), 2);
    assert!(parts[0].is::<FractionPicture>());
    assert!(parts[1].is::<gtk::Label>());

    // "/" moves on to the denominator, taking the digits after it along.
    imp.numerator_entry.grab_focus();
    imp.numerator_entry.set_text("1a/");
    assert_eq!(imp.numerator_entry.text(), "1");
    assert!(has_focus(&imp.denominator_entry));
    imp.numerator_entry.set_text("3/4");
    assert_eq!(imp.numerator_entry.text(), "3");
    assert_eq!(imp.denominator_entry.text(), "4");

    // Enter in the numerator moves on to an empty denominator.
    imp.denominator_entry.set_text("");
    imp.numerator_entry.emit_by_name::<()>("activate", &[]);
    assert!(has_focus(&imp.denominator_entry));
    assert_eq!(imp.round.borrow().as_ref().unwrap().answered(), 0);

    let answer = |n: u32, d: u32| {
        imp.numerator_entry.set_text(&n.to_string());
        imp.denominator_entry.set_text(&d.to_string());
        imp.denominator_entry.emit_by_name::<()>("activate", &[]);
    };
    let Answer::Fraction(n, d) = current_answer(&view) else {
        panic!("not a fraction question");
    };
    answer(n + d, d);
    assert!(imp.numerator_entry.has_css_class("error"));
    assert!(imp.denominator_entry.has_css_class("error"));
    assert_eq!(imp.numerator_entry.text(), "");
    assert!(has_focus(&imp.numerator_entry));
    assert_eq!(imp.counter_label.text(), "2 / 60");

    for i in 1..QUESTIONS_PER_ROUND {
        assert_eq!(finished.get(), 0);
        let Answer::Fraction(n, d) = current_answer(&view) else {
            panic!("not a fraction question");
        };
        // Level 2 accepts equivalent fractions.
        let k = if i % 2 == 0 { 1 } else { 2 };
        answer(k * n, k * d);
    }

    assert_eq!(finished.get(), 1);
    let result = view.summary().result;
    assert_eq!(result.wrong, 1);
    assert_eq!(result.correct, QUESTIONS_PER_ROUND - 1);
    window.destroy();
}

fn fraction_pictures_describe_themselves() {
    let picture = FractionPicture::new(&Picture {
        shape: Shape::Pizza,
        wholes: 1,
        pieces: 8,
        shaded: 3,
    });
    assert_eq!(
        picture.description(),
        "A pizza in 8 slices, 3 of them eaten"
    );
    let (width, ..) = picture.measure(gtk::Orientation::Horizontal, -1);
    assert!(width > 0 && width <= 360);

    let two = FractionPicture::new(&Picture {
        shape: Shape::Pizza,
        wholes: 2,
        pieces: 4,
        shaded: 7,
    });
    assert_eq!(
        two.description(),
        "Two pizzas in 4 slices each, 7 slices eaten"
    );
    let (width, ..) = two.measure(gtk::Orientation::Horizontal, -1);
    assert!(width <= 360, "{width}");

    let bar = FractionPicture::new(&Picture {
        shape: Shape::Bar { rows: 2, cols: 4 },
        wholes: 1,
        pieces: 8,
        shaded: 3,
    });
    assert_eq!(
        bar.description(),
        "A chocolate bar of 8 squares, 3 of them eaten"
    );
    let grid = FractionPicture::new(&Picture {
        shape: Shape::Grid { rows: 3, cols: 4 },
        wholes: 1,
        pieces: 12,
        shaded: 1,
    });
    assert_eq!(
        grid.description(),
        "A grid of 12 squares, 1 of them coloured"
    );
}

fn fraction_labels_show_and_describe_the_fraction() {
    let label = FractionLabel::new(&Fraction::new(3, 4));
    let imp = label.imp();
    assert_eq!(imp.numerator_label.text(), "3");
    assert_eq!(imp.denominator_label.text(), "4");
    assert!(!imp.whole_label.get_visible());

    let mixed = FractionLabel::new(&Fraction {
        whole: 1,
        numerator: 3,
        denominator: 4,
    });
    assert!(mixed.imp().whole_label.get_visible());
    assert_eq!(mixed.imp().whole_label.text(), "1");
}

/// Answers `dialog` as if the player picked `response`.
fn respond(dialog: &adw::AlertDialog, response: &str) {
    let signal = glib::subclass::SignalId::lookup("response", adw::AlertDialog::static_type())
        .expect("the response signal");
    dialog.emit_with_details::<()>(signal, glib::Quark::from_str(response), &[&response]);
}

fn forgetting_the_history_asks_first(data_dir: &std::path::Path) {
    let path = data_dir.join("fastmath/facts.json");
    assert!(path.exists());
    respond(&FastmathApplication::forget_history_dialog(), "cancel");
    assert!(path.exists());
    respond(&FastmathApplication::forget_history_dialog(), "forget");
    assert!(!path.exists());
    assert_eq!(FactHistory::load().len(), 0);
}

/// Saves a small known history: a wrong and a slow 7 × 8, a fast 3 × 4.
fn save_known_history() {
    let mut history = FactHistory::load();
    let q = |op, a, b, answer| crate::generator::Question { op, a, b, answer };
    history.record(
        &q(Operation::Multiply, 7, 8, 56),
        false,
        Duration::from_millis(9000),
    );
    history.record(
        &q(Operation::Multiply, 8, 7, 56),
        true,
        Duration::from_millis(3000),
    );
    history.record(
        &q(Operation::Multiply, 3, 4, 12),
        true,
        Duration::from_millis(1000),
    );
    history.record(
        &q(Operation::Multiply, 3, 4, 12),
        true,
        Duration::from_millis(1000),
    );
    history.save();
}

fn progress_shows_the_history() {
    // Without answers there is nothing to show.
    FactHistory::forget();
    let view = ProgressView::new();
    let imp = view.imp();
    assert_eq!(imp.stack.visible_child_name().unwrap(), "empty");
    assert!(!imp.operation_group.get_visible());

    save_known_history();
    let view = ProgressView::new();
    let imp = view.imp();
    assert_eq!(imp.stack.visible_child_name().unwrap(), "progress");
    assert_eq!(
        imp.summary_label.text(),
        "Multiplication: 75% right, 1.7 s on average, 1 weak fact"
    );
    imp.operation_group.set_active_name(Some("a"));
    assert_eq!(imp.summary_label.text(), "Addition: no answers yet");

    // The menu item opens it.
    let window: FastmathWindow = glib::Object::new();
    WidgetExt::activate_action(&window, "win.progress", None).unwrap();
    let page = window.imp().navigation_view.visible_page().unwrap();
    assert!(page.is::<ProgressView>());
    window.destroy();
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
    assert_eq!(FactHistory::load().len(), 0);
    a_full_round_ends_after_60_answers();
    // Arithmetic rounds keep the answers of each fact, comparisons don't.
    let facts = FactHistory::load().len();
    assert!(facts > 0);
    assert!(data_dir.join("fastmath/facts.json").exists());
    a_comparison_round_is_answered_with_buttons(ComparisonKinds::from_id_string("wd"), false);
    a_comparison_round_is_answered_with_buttons(ComparisonKinds::from_id_string("f"), true);
    assert_eq!(FactHistory::load().len(), facts);
    a_round_ends_when_the_time_is_up();
    a_practice_round_has_no_clock_and_20_questions();
    results_keep_a_leaderboard();
    a_fractions_round_is_answered_in_the_entries();
    fraction_pictures_describe_themselves();
    fraction_labels_show_and_describe_the_fraction();
    forgetting_the_history_asks_first(&data_dir);
    progress_shows_the_history();
    the_shortcuts_dialog_loads();

    let _ = std::fs::remove_dir_all(&data_dir);
}
