use gettextrs::gettext;
use gtk::gdk;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use i18n_format::i18n_fmt;
use libadwaita as adw;
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::G_LOG_DOMAIN;
use crate::round::{AnswerResult, GameConfig, Round, RoundResult};
use crate::sound_player::SoundPlayer;

/// How often the clock and the progress bar are refreshed.
const TICK: Duration = Duration::from_millis(100);
/// How long the answer entry shows whether the answer was right.
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
        pub question_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub answer_entry: TemplateChild<gtk::Entry>,

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

            *self.round.borrow_mut() = Some(Round::new(&view.game_config()));
            view.update_clock(Duration::ZERO);
            view.show_question();

            // Only digits can be typed.
            self.answer_entry.connect_changed(|entry| {
                let text = entry.text();
                let digits: String = text.chars().filter(char::is_ascii_digit).collect();
                if digits != text.as_str() {
                    entry.set_text(&digits);
                    entry.set_position(-1);
                }
            });
            self.answer_entry.connect_activate(glib::clone!(
                #[weak]
                view,
                move |_| view.submit_answer()
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
            self.answer_entry.grab_focus();
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
            .set_text(&crate::labels::format_clock(remaining_secs));
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
            imp.question_label.set_text(&question.text());
            let number = round.answered() + 1;
            imp.counter_label
                .set_text(&i18n_fmt! { i18n_fmt("{} / {}", number, round.limit()) });
        }
    }

    fn submit_answer(&self) {
        let imp = self.imp();
        if imp.done.get() || imp.started_at.get().is_none() || imp.paused_at.get().is_some() {
            return;
        }
        let Ok(answer) = imp.answer_entry.text().parse::<u32>() else {
            return;
        };

        let elapsed = self.elapsed();
        let (result, finished) = {
            let mut round = imp.round.borrow_mut();
            let Some(round) = round.as_mut() else {
                return;
            };
            let result = round.submit(answer, elapsed);
            (result, round.is_finished())
        };
        let Some(result) = result else {
            return;
        };

        imp.answer_entry.set_text("");
        match result {
            AnswerResult::Correct => {
                imp.sound_player.play_correct();
                self.flash_entry("success");
            }
            AnswerResult::Wrong => {
                imp.sound_player.play_wrong();
                self.flash_entry("error");
            }
        }

        if finished {
            self.end_round();
        } else {
            self.show_question();
        }
    }

    /// Highlights the answer entry for a moment with a style class.
    fn flash_entry(&self, class: &'static str) {
        let imp = self.imp();
        if let Some(source) = imp.flash.take() {
            source.remove();
        }
        imp.answer_entry.remove_css_class("success");
        imp.answer_entry.remove_css_class("error");
        imp.answer_entry.add_css_class(class);

        let source = glib::timeout_add_local_once(
            FLASH,
            glib::clone!(
                #[weak(rename_to = view)]
                self,
                move || {
                    let imp = view.imp();
                    imp.flash.take();
                    imp.answer_entry.remove_css_class(class);
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
        imp.answer_entry.grab_focus();
    }

    fn abandon(&self) {
        let imp = self.imp();
        imp.done.set(true);
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
