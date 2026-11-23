use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use i18n_format::i18n_fmt;
use libadwaita as adw;
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use std::sync::OnceLock;

use crate::config;
use crate::game_view::GameSummary;
use crate::labels;
use crate::leaderboard::{Leaderboard, LeaderboardEntry};
use crate::round::GameConfig;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use std::cell::{Cell, RefCell};

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/nacho/fastmath/ui/results_view.ui")]
    #[properties(wrapper_type = super::ResultsView)]
    pub struct ResultsView {
        #[template_child]
        pub score_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub config_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub correct_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub wrong_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub unanswered_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub average_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub save_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub name_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub leaderboard_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub retry_button: TemplateChild<gtk::Button>,

        #[property(get, set, construct_only)]
        pub summary: Cell<Option<GameSummary>>,
        pub leaderboard: RefCell<Leaderboard>,
        pub rows: RefCell<Vec<adw::ActionRow>>,
        pub saved: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ResultsView {
        const NAME: &'static str = "FastmathResultsView";
        type Type = super::ResultsView;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ResultsView {
        fn constructed(&self) {
            self.parent_constructed();
            let view = self.obj();

            *self.leaderboard.borrow_mut() = Leaderboard::load(&view.game_summary().config);
            view.show_summary();
            view.show_leaderboard(None);
            view.offer_to_save();

            self.back_button.connect_clicked(glib::clone!(
                #[weak]
                view,
                move |_| view.emit_by_name::<()>("back", &[])
            ));
            self.retry_button.connect_clicked(glib::clone!(
                #[weak]
                view,
                move |_| view.emit_by_name::<()>("retry", &[])
            ));
            self.name_row.connect_apply(glib::clone!(
                #[weak]
                view,
                move |_| view.save_score()
            ));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("retry").build(),
                    Signal::builder("back").build(),
                ]
            })
        }
    }

    impl WidgetImpl for ResultsView {}
    impl NavigationPageImpl for ResultsView {}
}

glib::wrapper! {
    pub struct ResultsView(ObjectSubclass<imp::ResultsView>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ResultsView {
    pub fn new(summary: &GameSummary) -> Self {
        glib::Object::builder()
            .property("summary", *summary)
            .build()
    }

    fn game_summary(&self) -> GameSummary {
        self.summary().expect("the summary is a construct property")
    }

    pub fn config(&self) -> GameConfig {
        self.game_summary().config
    }

    fn average_ms(&self) -> Option<u64> {
        self.game_summary()
            .result
            .average_time
            .map(|time| time.as_millis() as u64)
    }

    fn show_summary(&self) {
        let imp = self.imp();
        let GameSummary { config, result } = self.game_summary();

        imp.score_label.set_text(&i18n_fmt! {
            i18n_nfmt("{} correct answer", "{} correct answers", result.correct, result.correct)
        });
        imp.config_label.set_text(&labels::config_summary(&config));
        imp.correct_label.set_text(&result.correct.to_string());
        imp.wrong_label.set_text(&result.wrong.to_string());
        imp.unanswered_label
            .set_text(&result.unanswered.to_string());
        imp.average_label.set_text(
            &self
                .average_ms()
                .map_or_else(|| gettext("—"), labels::format_average),
        );
    }

    /// Shows the name entry when the score earns a place in the leaderboard.
    fn offer_to_save(&self) {
        let imp = self.imp();
        let result = self.game_summary().result;
        let qualifies = result.correct > 0
            && imp
                .leaderboard
                .borrow()
                .qualifies(result.correct, self.average_ms().unwrap_or(0));
        if !qualifies {
            return;
        }

        let settings = gio::Settings::new(config::APPLICATION_ID);
        let mut name = settings.string("player-name").to_string();
        if name.is_empty() {
            let real_name = glib::real_name().to_string_lossy().into_owned();
            if real_name != "Unknown" {
                name = real_name;
            }
        }
        imp.name_row.set_text(&name);
        imp.save_group.set_visible(true);
    }

    fn save_score(&self) {
        let imp = self.imp();
        let name = imp.name_row.text().trim().to_string();
        if name.is_empty() || imp.saved.replace(true) {
            return;
        }

        let result = self.game_summary().result;
        let rank = {
            let mut leaderboard = imp.leaderboard.borrow_mut();
            let rank = leaderboard.insert(LeaderboardEntry {
                name: name.clone(),
                correct: result.correct,
                average_ms: self.average_ms().unwrap_or(0),
            });
            leaderboard.save(&self.game_summary().config);
            rank
        };

        let settings = gio::Settings::new(config::APPLICATION_ID);
        let _ = settings.set_string("player-name", &name);

        imp.save_group.set_visible(false);
        self.show_leaderboard(Some(rank));
    }

    /// Fills the leaderboard group, highlighting the entry at `highlight`.
    fn show_leaderboard(&self, highlight: Option<usize>) {
        let imp = self.imp();

        for row in imp.rows.take() {
            imp.leaderboard_group.remove(&row);
        }

        let leaderboard = imp.leaderboard.borrow();
        imp.leaderboard_group
            .set_description(Some(&if leaderboard.entries.is_empty() {
                gettext("No scores yet with these settings")
            } else {
                gettext("Best scores with these settings")
            }));

        let mut rows = imp.rows.borrow_mut();
        for (index, entry) in leaderboard.entries.iter().enumerate() {
            let rank = gtk::Label::new(Some(&(index + 1).to_string()));
            rank.add_css_class("numeric");
            rank.add_css_class("dim-label");

            let average = labels::format_average(entry.average_ms);
            let row = adw::ActionRow::builder()
                .title(&entry.name)
                .use_markup(false)
                .subtitle(i18n_fmt! {
                    i18n_fmt("{} correct, {} per answer", entry.correct, average)
                })
                .build();
            row.add_prefix(&rank);
            if Some(index) == highlight {
                row.add_css_class("accent");
            }
            imp.leaderboard_group.add(&row);
            rows.push(row);
        }
    }

    pub fn connect_retry<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("retry", false, move |values| {
            f(&values[0].get::<Self>().expect("signal emitter"));
            None
        })
    }

    pub fn connect_back<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("back", false, move |values| {
            f(&values[0].get::<Self>().expect("signal emitter"));
            None
        })
    }
}
