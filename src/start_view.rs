use gtk::prelude::*;
use gtk::subclass::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use std::sync::OnceLock;

use crate::comparison::{ComparisonKind, ComparisonKinds};
use crate::config;
use crate::labels;
use crate::level::{LEVEL_COUNT, Level};
use crate::operation::{Operation, OperationSet};
use crate::round::{GameConfig, Mode};

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use std::cell::OnceCell;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/nacho/fastmath/ui/start_view.ui")]
    pub struct StartView {
        #[template_child]
        pub mode_group: TemplateChild<adw::ToggleGroup>,
        #[template_child]
        pub operations_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub add_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub subtract_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub multiply_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub divide_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub all_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub comparisons_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub whole_numbers_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub decimals_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub decimals_symbol: TemplateChild<gtk::Label>,
        #[template_child]
        pub fractions_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub calculations_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub all_comparisons_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub level_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub duration_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub start_button: TemplateChild<gtk::Button>,
        pub settings: OnceCell<gio::Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StartView {
        const NAME: &'static str = "FastmathStartView";
        type Type = super::StartView;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for StartView {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("start").build()])
        }
    }

    impl WidgetImpl for StartView {}
    impl NavigationPageImpl for StartView {}
}

glib::wrapper! {
    pub struct StartView(ObjectSubclass<imp::StartView>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl StartView {
    fn settings(&self) -> &gio::Settings {
        self.imp().settings.get().expect("settings are set up")
    }

    fn operation_rows(&self) -> [(Operation, &adw::SwitchRow); 4] {
        let imp = self.imp();
        [
            (Operation::Add, &*imp.add_row),
            (Operation::Subtract, &*imp.subtract_row),
            (Operation::Multiply, &*imp.multiply_row),
            (Operation::Divide, &*imp.divide_row),
        ]
    }

    fn comparison_rows(&self) -> [(ComparisonKind, &adw::SwitchRow); 4] {
        let imp = self.imp();
        [
            (ComparisonKind::WholeNumbers, &*imp.whole_numbers_row),
            (ComparisonKind::Decimals, &*imp.decimals_row),
            (ComparisonKind::Fractions, &*imp.fractions_row),
            (ComparisonKind::Calculations, &*imp.calculations_row),
        ]
    }

    fn setup(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(config::APPLICATION_ID);
        let _ = imp.settings.set(settings.clone());

        let ops = OperationSet::from_id_string(&settings.string("operations"));
        let operation_rows = self
            .operation_rows()
            .map(|(op, row)| (ops.contains(op), row));
        let kinds = ComparisonKinds::from_id_string(&settings.string("comparisons"));
        let comparison_rows = self
            .comparison_rows()
            .map(|(kind, row)| (kinds.contains(kind), row));
        for (active, row) in operation_rows.into_iter().chain(comparison_rows) {
            row.set_active(active);
            row.connect_active_notify(glib::clone!(
                #[weak(rename_to = this)]
                self,
                move |_| this.choices_changed()
            ));
        }

        imp.all_button.connect_clicked(glib::clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                for (_, row) in this.operation_rows() {
                    row.set_active(true);
                }
            }
        ));
        imp.all_comparisons_button.connect_clicked(glib::clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                for (_, row) in this.comparison_rows() {
                    row.set_active(true);
                }
            }
        ));

        imp.decimals_symbol
            .set_label(&format!("0{}5", labels::decimal_separator()));

        imp.level_row.connect_selected_notify(glib::clone!(
            #[weak(rename_to = this)]
            self,
            move |row| {
                let _ = this.settings().set_uint("level", row.selected() + 1);
                this.update_comparison_descriptions();
            }
        ));

        settings
            .bind("duration-secs", &*imp.duration_row, "value")
            .build();

        settings
            .bind("mode", &*imp.mode_group, "active-name")
            .build();
        imp.mode_group.connect_active_name_notify(glib::clone!(
            #[weak(rename_to = this)]
            self,
            move |_| this.mode_changed()
        ));
        self.mode_changed();

        imp.start_button.connect_clicked(glib::clone!(
            #[weak(rename_to = this)]
            self,
            move |_| this.emit_by_name::<()>("start", &[])
        ));
    }

    fn mode(&self) -> Mode {
        Mode::from_id(&self.imp().mode_group.active_name().unwrap_or_default())
    }

    /// Shows the choices of the selected mode.
    fn mode_changed(&self) {
        let imp = self.imp();
        let mode = self.mode();
        imp.operations_group.set_visible(mode == Mode::Arithmetic);
        imp.comparisons_group.set_visible(mode == Mode::Comparison);

        // Changing the model selects the first level, so keep the saved one.
        let level = Level::new(self.settings().uint("level"));
        let levels: Vec<String> = (1..=LEVEL_COUNT)
            .map(|n| labels::level_description(mode, Level::new(n)))
            .collect();
        let levels: Vec<&str> = levels.iter().map(String::as_str).collect();
        imp.level_row
            .set_model(Some(&gtk::StringList::new(&levels)));
        imp.level_row.set_selected(level.number() - 1);

        self.choices_changed();
    }

    /// Describes what each kind of comparison asks at the selected level.
    fn update_comparison_descriptions(&self) {
        let level = Level::new(self.imp().level_row.selected() + 1);
        for (kind, row) in self.comparison_rows() {
            row.set_subtitle(&labels::comparison_kind_description(kind, level));
        }
    }

    /// The operations currently selected.
    fn operations(&self) -> OperationSet {
        self.operation_rows()
            .into_iter()
            .fold(OperationSet::from_id_string(""), |set, (op, row)| {
                set.with(op, row.is_active())
            })
    }

    /// The comparison kinds currently selected.
    fn comparison_kinds(&self) -> ComparisonKinds {
        self.comparison_rows()
            .into_iter()
            .fold(ComparisonKinds::from_id_string(""), |set, (kind, row)| {
                set.with(kind, row.is_active())
            })
    }

    /// Saves the selected operations and comparisons. A round needs at
    /// least one of those of its mode.
    fn choices_changed(&self) {
        let settings = self.settings();
        let ops = self.operations();
        if !ops.is_empty() {
            let _ = settings.set_string("operations", &ops.to_id_string());
        }
        let kinds = self.comparison_kinds();
        if !kinds.is_empty() {
            let _ = settings.set_string("comparisons", &kinds.to_id_string());
        }
        let can_start = match self.mode() {
            Mode::Arithmetic => !ops.is_empty(),
            Mode::Comparison => !kinds.is_empty(),
        };
        self.imp().start_button.set_sensitive(can_start);
    }

    /// What the player has chosen to play.
    pub fn game_config(&self) -> GameConfig {
        let settings = self.settings();
        GameConfig {
            mode: self.mode(),
            ops: self.operations(),
            kinds: self.comparison_kinds(),
            level: Level::new(settings.uint("level")),
            duration_secs: settings.uint("duration-secs"),
        }
    }

    pub fn connect_start<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("start", false, move |values| {
            let this = values[0].get::<Self>().expect("signal emitter");
            f(&this);
            None
        })
    }
}
