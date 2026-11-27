use gettextrs::gettext;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;

use crate::config;
use crate::practice::FactHistory;
use crate::window::FastmathWindow;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FastmathApplication;

    #[glib::object_subclass]
    impl ObjectSubclass for FastmathApplication {
        const NAME: &'static str = "FastmathApplication";
        type Type = super::FastmathApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for FastmathApplication {}

    impl ApplicationImpl for FastmathApplication {
        fn startup(&self) {
            self.parent_startup();
            let app = self.obj();
            app.setup_actions();
            app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
            app.set_accels_for_action("app.shortcuts", &["<Ctrl>question"]);
            app.set_accels_for_action("window.close", &["<Ctrl>W"]);

            let provider = gtk::CssProvider::new();
            provider.load_from_resource("/io/github/nacho/fastmath/style.css");
            gtk::style_context_add_provider_for_display(
                &gtk::gdk::Display::default().expect("Could not connect to a display"),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        fn activate(&self) {
            let app = self.obj();
            let window = match app.active_window() {
                Some(window) => window,
                None => {
                    let window = FastmathWindow::new(app.upcast_ref());
                    window.load_window_state();
                    window.upcast()
                }
            };
            window.present();
        }
    }

    impl GtkApplicationImpl for FastmathApplication {}
    impl AdwApplicationImpl for FastmathApplication {}
}

glib::wrapper! {
    pub struct FastmathApplication(ObjectSubclass<imp::FastmathApplication>)
        @extends adw::Application, gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl FastmathApplication {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", config::APPLICATION_ID)
            .build()
    }

    fn setup_actions(&self) {
        // The sound effects item in the menu toggles the setting directly.
        let settings = gio::Settings::new(config::APPLICATION_ID);
        self.add_action(&settings.create_action("sound-effects"));

        self.add_action_entries(vec![
            gio::ActionEntry::builder("quit")
                .activate(|app: &Self, _, _| app.quit())
                .build(),
            gio::ActionEntry::builder("shortcuts")
                .activate(|app: &Self, _, _| app.show_shortcuts())
                .build(),
            gio::ActionEntry::builder("about")
                .activate(|app: &Self, _, _| app.show_about())
                .build(),
            gio::ActionEntry::builder("forget-history")
                .activate(|app: &Self, _, _| {
                    Self::forget_history_dialog().present(app.active_window().as_ref());
                })
                .build(),
        ]);
    }

    /// Asks before deleting the practice history.
    pub(crate) fn forget_history_dialog() -> adw::AlertDialog {
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Forget the practice history?"))
            .body(gettext(
                "Practice rounds will no longer know which facts you answered wrong or slowly.",
            ))
            .build();
        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("forget", &gettext("_Forget")),
        ]);
        dialog.set_response_appearance("forget", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.connect_response(Some("forget"), |_, _| FactHistory::forget());
        dialog
    }

    fn show_shortcuts(&self) {
        // The builder needs the types to be registered before it parses.
        adw::ShortcutsDialog::ensure_type();
        adw::ShortcutsSection::ensure_type();
        adw::ShortcutsItem::ensure_type();

        let builder =
            gtk::Builder::from_resource("/io/github/nacho/fastmath/ui/shortcuts_dialog.ui");
        if let Some(dialog) = builder.object::<adw::ShortcutsDialog>("shortcuts_dialog") {
            dialog.present(self.active_window().as_ref());
        }
    }

    fn show_about(&self) {
        let dialog = adw::AboutDialog::builder()
            .application_name(gettext("Fast Math"))
            .application_icon(config::APPLICATION_ID)
            .version(config::VERSION)
            .developer_name("Ignacio Casal Quinteiro")
            .developers(["Ignacio Casal Quinteiro"])
            .license_type(gtk::License::Gpl30)
            .website("https://github.com/nacho/fastmath")
            .issue_url("https://github.com/nacho/fastmath/issues")
            .copyright("© 2026 Ignacio Casal Quinteiro")
            .translator_credits(gettext("translator-credits"))
            .build();
        dialog.add_acknowledgement_section(
            Some(&gettext("Sound Sources")),
            &[
                "Correct and wrong sounds: freedesktop sound theme (CC BY-SA) https://freedesktop.org/wiki/Specifications/sound-theme-spec/",
                "Quiz game music loop BPM 90 by portwain (CC0) https://freesound.org/people/portwain/sounds/220060/",
            ],
        );
        // Link to other apps by the same developer.
        dialog.add_other_app(
            "io.github.nacho.mundi",
            "Mundi",
            &gettext("Learn geography"),
        );
        dialog.add_other_app(
            "io.github.nacho.teclas",
            "Teclas",
            &gettext("Learn to type"),
        );
        dialog.present(self.active_window().as_ref());
    }
}
