use gettextrs::gettext;
use gtk::gdk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use std::cmp::Ordering;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::G_LOG_DOMAIN;
use crate::comparison::Part;
use crate::fraction_label::FractionLabel;
use crate::fraction_picture::FractionPicture;
use crate::labels;
use crate::practice::FactHistory;
use crate::round::{Answer, AnswerResult, Answered, GameConfig, Mode, Prompt, Round, RoundResult};
use crate::sound_player::SoundPlayer;

/// How often the clock and the progress bar are refreshed.
const TICK: Duration = Duration::from_millis(100);
/// How long the answer entry or button shows whether the answer was right.
const FLASH: Duration = Duration::from_millis(400);

/// What was played and how it went.
#[derive(Clone, Copy, Debug, glib::Boxed)]
#[boxed_type(name = "FastmathGameSummary", nullable)]
pub struct GameSummary {
    pub config: GameConfig,
    pub result: RoundResult,
}

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use std::cell::{Cell, RefCell};

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/nacho/fastmath/ui/game_view.ui")]
    #[properties(wrapper_type = super::GameView)]
    pub struct GameView {
        #[template_child]
        pub time_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub counter_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub question_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub answer_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub compare_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub fraction_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub numerator_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub denominator_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub less_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub equal_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub greater_button: TemplateChild<gtk::Button>,

        #[property(get, set, construct_only)]
        pub config: Cell<Option<GameConfig>>,
        pub round: RefCell<Option<Round>>,
        /// When the clock started, moved forward by the time spent paused.
        pub started_at: Cell<Option<Instant>>,
        pub paused_at: Cell<Option<Instant>>,
        pub done: Cell<bool>,
        pub timer: RefCell<Option<glib::SourceId>>,
        pub flash: RefCell<Option<glib::SourceId>>,
        pub sound_player: SoundPlayer,
        /// The answers to arithmetic facts, for practice rounds.
        pub history: RefCell<Option<FactHistory>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GameView {
        const NAME: &'static str = "FastmathGameView";
        type Type = super::GameView;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("game.abandon", None, |view, _, _| {
                view.confirm_abandon();
            });
            klass.add_binding_action(gdk::Key::Escape, gdk::ModifierType::empty(), "game.abandon");

            klass.install_action(
                "game.compare",
                Some(glib::VariantTy::STRING),
                |view, _, target| {
                    let order = match target.and_then(|t| t.str()) {
                        Some("less") => Ordering::Less,
                        Some("equal") => Ordering::Equal,
                        Some("greater") => Ordering::Greater,
                        _ => return,
                    };
                    view.submit(Answer::Order(order));
                },
            );
            for (key, order) in [
                (gdk::Key::less, Ordering::Less),
                (gdk::Key::equal, Ordering::Equal),
                (gdk::Key::greater, Ordering::Greater),
            ] {
                klass.add_binding(key, gdk::ModifierType::empty(), move |view| {
                    if view.game_config().mode != Mode::Comparison {
                        return glib::Propagation::Proceed;
                    }
                    view.submit(Answer::Order(order));
                    glib::Propagation::Stop
                });
            }
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for GameView {
        fn constructed(&self) {
            self.parent_constructed();
            let view = self.obj();

            let config = view.game_config();
            *self.round.borrow_mut() = Some(Round::new(&config));
            if config.mode == Mode::Arithmetic {
                *self.history.borrow_mut() = Some(FactHistory::load());
            }
            self.answer_entry
                .set_visible(config.mode == Mode::Arithmetic);
            self.compare_box
                .set_visible(config.mode == Mode::Comparison);
            self.fraction_box
                .set_visible(config.mode == Mode::Fractions);
            if config.mode == Mode::Fractions {
                self.question_box
                    .set_orientation(gtk::Orientation::Vertical);
            }
            view.update_clock(Duration::ZERO);
            view.show_question();

            // Only digits can be typed. A "/" in the numerator moves on to
            // the denominator, taking any digits after it along.
            for entry in [
                &*self.answer_entry,
                &*self.numerator_entry,
                &*self.denominator_entry,
            ] {
                entry.connect_changed(glib::clone!(
                    #[weak]
                    view,
                    move |entry| {
                        let text = entry.text();
                        let imp = view.imp();
                        let (before, after) = match text.split_once('/') {
                            Some((before, after)) if entry == &*imp.numerator_entry => {
                                (before, Some(after))
                            }
                            _ => (text.as_str(), None),
                        };
                        let digits: String = before.chars().filter(char::is_ascii_digit).collect();
                        if digits != text.as_str() {
                            entry.set_text(&digits);
                            entry.set_position(-1);
                        }
                        if let Some(after) = after {
                            let denominator = &imp.denominator_entry;
                            let digits: String =
                                after.chars().filter(char::is_ascii_digit).collect();
                            if !digits.is_empty() {
                                denominator.set_text(&digits);
                            }
                            denominator.grab_focus();
                            denominator.set_position(-1);
                        }
                    }
                ));
            }
            self.answer_entry.connect_activate(glib::clone!(
                #[weak]
                view,
                move |entry| {
                    if let Ok(answer) = entry.text().parse::<u32>() {
                        view.submit(Answer::Number(answer));
                    }
                }
            ));
            // Enter in the numerator moves on to an empty denominator.
            self.numerator_entry.connect_activate(glib::clone!(
                #[weak]
                view,
                move |_| {
                    let denominator = &view.imp().denominator_entry;
                    if denominator.text().is_empty() {
                        denominator.grab_focus();
                    } else {
                        view.submit_fraction();
                    }
                }
            ));
            self.denominator_entry.connect_activate(glib::clone!(
                #[weak]
                view,
                move |_| view.submit_fraction()
            ));
        }

        fn dispose(&self) {
            self.obj().stop_timer();
            if let Some(source) = self.flash.take() {
                source.remove();
            }
            self.sound_player.stop_music();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("finished").build(),
                    Signal::builder("abandoned").build(),
                ]
            })
        }
    }

    impl WidgetImpl for GameView {}

    impl NavigationPageImpl for GameView {
        // The clock starts once the page is on screen, so the transition
        // does not eat into the player's time.
        fn shown(&self) {
            self.parent_shown();
            let view = self.obj();
            view.focus_answer();
            if self.started_at.get().is_none() && !self.done.get() {
                view.start_clock();
            }
        }
    }
}

glib::wrapper! {
    pub struct GameView(ObjectSubclass<imp::GameView>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl GameView {
    pub fn new(config: &GameConfig) -> Self {
        glib::Object::builder().property("config", *config).build()
    }

    fn game_config(&self) -> GameConfig {
        self.config().expect("the config is a construct property")
    }

    fn total_time(&self) -> Duration {
        Duration::from_secs(self.game_config().duration_secs.into())
    }

    /// The time elapsed since the round started, not counting pauses.
    fn elapsed(&self) -> Duration {
        let imp = self.imp();
        let now = imp.paused_at.get().unwrap_or_else(Instant::now);
        imp.started_at
            .get()
            .map_or(Duration::ZERO, |start| now.saturating_duration_since(start))
    }

    pub(crate) fn start_clock(&self) {
        let imp = self.imp();
        imp.started_at.set(Some(Instant::now()));
        imp.sound_player.play_music();
        self.start_timer();
    }

    fn start_timer(&self) {
        let source = glib::timeout_add_local(
            TICK,
            glib::clone!(
                #[weak(rename_to = view)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || view.tick()
            ),
        );
        *self.imp().timer.borrow_mut() = Some(source);
    }

    fn stop_timer(&self) {
        if let Some(source) = self.imp().timer.take() {
            source.remove();
        }
    }

    pub(crate) fn tick(&self) -> glib::ControlFlow {
        let elapsed = self.elapsed();
        if elapsed >= self.total_time() {
            // The source is removed by returning Break.
            self.imp().timer.take();
            self.update_clock(self.total_time());
            self.end_round();
            return glib::ControlFlow::Break;
        }
        self.update_clock(elapsed);
        glib::ControlFlow::Continue
    }

    fn update_clock(&self, elapsed: Duration) {
        let imp = self.imp();
        let total = self.total_time();
        let remaining = total.saturating_sub(elapsed);
        let remaining_secs = remaining.as_secs_f64().ceil() as u32;
        imp.time_label
            .set_text(&labels::format_clock(remaining_secs));
        imp.progress_bar
            .set_fraction(remaining.as_secs_f64() / total.as_secs_f64());
    }

    fn show_question(&self) {
        let imp = self.imp();
        let round = imp.round.borrow();
        let Some(round) = round.as_ref() else {
            return;
        };
        if let Some(question) = round.current() {
            while let Some(child) = imp.question_box.first_child() {
                imp.question_box.remove(&child);
            }
            let comparison = matches!(question, Prompt::Comparison(_));
            for part in question.parts(&labels::decimal_separator()) {
                let widget: gtk::Widget = match part {
                    Part::Text(text) => {
                        let label = gtk::Label::new(Some(&text));
                        label.add_css_class("question");
                        label.add_css_class("numeric");
                        if comparison {
                            label.add_css_class("comparison");
                        }
                        label.upcast()
                    }
                    Part::Fraction(fraction) => FractionLabel::new(&fraction).upcast(),
                    Part::Picture(question) => {
                        imp.question_box
                            .append(&FractionPicture::new(&question.picture));
                        let label = gtk::Label::new(Some(&labels::fraction_question(&question)));
                        label.set_wrap(true);
                        label.set_justify(gtk::Justification::Center);
                        label.add_css_class("title-2");
                        label.upcast()
                    }
                };
                imp.question_box.append(&widget);
            }
            let number = round.answered() + 1;
            imp.counter_label
                .set_text(&format!("{} / {}", number, round.limit()));
        }
    }

    /// Gives the keyboard focus to the answer entry, the numerator, or the
    /// middle button of a comparison so that the keys reach the page.
    fn focus_answer(&self) {
        let imp = self.imp();
        match self.game_config().mode {
            Mode::Arithmetic => imp.answer_entry.grab_focus(),
            Mode::Comparison => imp.equal_button.grab_focus(),
            Mode::Fractions => imp.numerator_entry.grab_focus(),
        };
    }

    /// Submits the fraction in the entries, once both have a number.
    fn submit_fraction(&self) {
        let imp = self.imp();
        if let (Ok(n), Ok(d)) = (
            imp.numerator_entry.text().parse::<u32>(),
            imp.denominator_entry.text().parse::<u32>(),
        ) {
            self.submit(Answer::Fraction(n, d));
        }
    }

    fn submit(&self, answer: Answer) {
        let imp = self.imp();
        if imp.done.get() || imp.started_at.get().is_none() || imp.paused_at.get().is_some() {
            return;
        }

        let elapsed = self.elapsed();
        let (question, answered, finished) = {
            let mut round = imp.round.borrow_mut();
            let Some(round) = round.as_mut() else {
                return;
            };
            let question = round.current().copied();
            let answered = round.submit(answer, elapsed);
            (question, answered, round.is_finished())
        };
        let Some(Answered { result, time }) = answered else {
            return;
        };
        if let (Some(history), Some(Prompt::Arithmetic(question))) =
            (imp.history.borrow_mut().as_mut(), question)
        {
            history.record(&question, result == AnswerResult::Correct, time);
        }

        // Show the result on what the player used to answer.
        let widgets: Vec<gtk::Widget> = match answer {
            Answer::Number(_) => {
                imp.answer_entry.set_text("");
                vec![imp.answer_entry.clone().upcast()]
            }
            Answer::Order(Ordering::Less) => vec![imp.less_button.clone().upcast()],
            Answer::Order(Ordering::Equal) => vec![imp.equal_button.clone().upcast()],
            Answer::Order(Ordering::Greater) => vec![imp.greater_button.clone().upcast()],
            Answer::Fraction(..) => {
                imp.numerator_entry.set_text("");
                imp.denominator_entry.set_text("");
                imp.numerator_entry.grab_focus();
                vec![
                    imp.numerator_entry.clone().upcast(),
                    imp.denominator_entry.clone().upcast(),
                ]
            }
        };
        match result {
            AnswerResult::Correct => {
                imp.sound_player.play_correct();
                self.flash(widgets, "success");
            }
            AnswerResult::Wrong => {
                imp.sound_player.play_wrong();
                self.flash(widgets, "error");
            }
        }

        if finished {
            self.end_round();
        } else {
            self.show_question();
        }
    }

    /// Highlights `widgets`, the entries or a button, for a moment with a
    /// style class.
    fn flash(&self, widgets: Vec<gtk::Widget>, class: &'static str) {
        let imp = self.imp();
        if let Some(source) = imp.flash.take() {
            source.remove();
        }
        let all: [&gtk::Widget; 6] = [
            imp.answer_entry.upcast_ref(),
            imp.less_button.upcast_ref(),
            imp.equal_button.upcast_ref(),
            imp.greater_button.upcast_ref(),
            imp.numerator_entry.upcast_ref(),
            imp.denominator_entry.upcast_ref(),
        ];
        for w in all {
            w.remove_css_class("success");
            w.remove_css_class("error");
        }
        for w in &widgets {
            w.add_css_class(class);
        }

        let source = glib::timeout_add_local_once(
            FLASH,
            glib::clone!(
                #[weak(rename_to = view)]
                self,
                move || {
                    view.imp().flash.take();
                    for w in &widgets {
                        w.remove_css_class(class);
                    }
                }
            ),
        );
        *imp.flash.borrow_mut() = Some(source);
    }

    /// Ends the round and tells the window to show the results.
    fn end_round(&self) {
        let imp = self.imp();
        if imp.done.replace(true) {
            return;
        }
        self.stop_timer();
        imp.sound_player.stop_music();
        if let Some(round) = imp.round.borrow_mut().as_mut() {
            round.finish();
        }
        self.save_history();
        glib::debug!("Round finished: {:?}", self.summary().result);
        self.set_can_pop(true);
        self.emit_by_name::<()>("finished", &[]);
    }

    fn confirm_abandon(&self) {
        let imp = self.imp();
        if imp.done.get() || imp.started_at.get().is_none() || imp.paused_at.get().is_some() {
            return;
        }

        // The clock stops while the dialog is open.
        imp.paused_at.set(Some(Instant::now()));
        self.stop_timer();
        imp.sound_player.stop_music();

        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Abandon this round?"))
            .body(gettext("Your progress in this round will be lost."))
            .build();
        dialog.add_responses(&[
            ("cancel", &gettext("_Keep Playing")),
            ("abandon", &gettext("_Abandon")),
        ]);
        dialog.set_response_appearance("abandon", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = view)]
                self,
                move |_, response| {
                    if response == "abandon" {
                        view.abandon();
                    } else {
                        view.resume();
                    }
                }
            ),
        );
        dialog.present(Some(self));
    }

    fn resume(&self) {
        let imp = self.imp();
        if let (Some(start), Some(paused_at)) = (imp.started_at.get(), imp.paused_at.take()) {
            imp.started_at.set(Some(start + paused_at.elapsed()));
        }
        imp.sound_player.play_music();
        self.start_timer();
        self.focus_answer();
    }

    /// Saves the answers of the round, also when it is abandoned.
    fn save_history(&self) {
        if let Some(history) = self.imp().history.borrow().as_ref() {
            history.save();
        }
    }

    fn abandon(&self) {
        let imp = self.imp();
        imp.done.set(true);
        self.save_history();
        self.stop_timer();
        imp.sound_player.stop_music();
        self.set_can_pop(true);
        self.emit_by_name::<()>("abandoned", &[]);
    }

    /// The configuration and the result of the round.
    pub fn summary(&self) -> GameSummary {
        let round = self.imp().round.borrow();
        GameSummary {
            config: self.game_config(),
            result: round
                .as_ref()
                .expect("the round is set in constructed()")
                .result(),
        }
    }

    pub fn connect_finished<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("finished", false, move |values| {
            f(&values[0].get::<Self>().expect("signal emitter"));
            None
        })
    }

    pub fn connect_abandoned<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("abandoned", false, move |values| {
            f(&values[0].get::<Self>().expect("signal emitter"));
            None
        })
    }
}
