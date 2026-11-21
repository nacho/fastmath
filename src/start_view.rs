use gtk::prelude::*;
use gtk::subclass::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use std::sync::OnceLock;

use crate::config;
use crate::labels;
use crate::lib::level::{LEVEL_COUNT, Level};
use crate::lib::operation::{Operation, OperationSet};
use crate::lib::round::GameConfig;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use std::cell::OnceCell;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/nacho/fastmath/ui/start_view.ui")]
    pub struct StartView {
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

    fn setup(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(config::APPLICATION_ID);
        let _ = imp.settings.set(settings.clone());

        let ops = OperationSet::from_id_string(&settings.string("operations"));
        for (op, row) in self.operation_rows() {
            row.set_active(ops.contains(op));
            row.connect_active_notify(glib::clone!(
                #[weak(rename_to = this)]
                self,
                move |_| this.operations_changed()
            ));
        }
        self.operations_changed();

        imp.all_button.connect_clicked(glib::clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                for (_, row) in this.operation_rows() {
                    row.set_active(true);
                }
            }
        ));

        let levels: Vec<String> = (1..=LEVEL_COUNT)
            .map(|n| labels::level_description(Level::new(n)))
            .collect();
        let levels: Vec<&str> = levels.iter().map(String::as_str).collect();
        imp.level_row
            .set_model(Some(&gtk::StringList::new(&levels)));
        imp.level_row
            .set_selected(Level::new(settings.uint("level")).number() - 1);
        imp.level_row.connect_selected_notify(glib::clone!(
            #[weak(rename_to = this)]
            self,
            move |row| {
                let _ = this.settings().set_uint("level", row.selected() + 1);
            }
        ));

        settings
            .bind("duration-secs", &*imp.duration_row, "value")
            .build();

        imp.start_button.connect_clicked(glib::clone!(
            #[weak(rename_to = this)]
            self,
            move |_| this.emit_by_name::<()>("start", &[])
        ));
    }

    /// The operations currently selected.
    fn operations(&self) -> OperationSet {
        self.operation_rows()
            .into_iter()
            .fold(OperationSet::from_id_string(""), |set, (op, row)| {
                set.with(op, row.is_active())
            })
    }

    /// Saves the selected operations. Starting needs at least one.
    fn operations_changed(&self) {
        let ops = self.operations();
        if !ops.is_empty() {
            let _ = self
                .settings()
                .set_string("operations", &ops.to_id_string());
        }
        self.imp().start_button.set_sensitive(!ops.is_empty());
    }

    /// What the player has chosen to play.
    pub fn game_config(&self) -> GameConfig {
        let settings = self.settings();
        GameConfig {
            ops: self.operations(),
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
