use gtk::prelude::*;
use gtk::subclass::prelude::*;
use libadwaita as adw;
use libadwaita::subclass::prelude::*;

use crate::config;
use crate::game_view::GameView;
use crate::results_view::ResultsView;
use crate::round::GameConfig;
use crate::start_view::StartView;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/nacho/fastmath/ui/window.ui")]
    pub struct FastmathWindow {
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub start_view: TemplateChild<StartView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FastmathWindow {
        const NAME: &'static str = "FastmathWindow";
        type Type = super::FastmathWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            StartView::ensure_type();
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FastmathWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let window = self.obj();
            self.start_view.connect_start(glib::clone!(
                #[weak]
                window,
                move |start_view| window.play(start_view.game_config())
            ));
        }
    }

    impl WidgetImpl for FastmathWindow {}
    impl WindowImpl for FastmathWindow {}
    impl ApplicationWindowImpl for FastmathWindow {}
    impl AdwApplicationWindowImpl for FastmathWindow {}
}

glib::wrapper! {
    pub struct FastmathWindow(ObjectSubclass<imp::FastmathWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl FastmathWindow {
    pub fn new(app: &adw::Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    /// Starts a round, replacing whatever is on top of the start page.
    fn play(&self, config: GameConfig) {
        let imp = self.imp();
        let game = GameView::new(&config);

        game.connect_finished(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |game| window.show_results(game)
        ));
        game.connect_abandoned(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| window.show_start()
        ));

        imp.navigation_view
            .replace(&[imp.start_view.clone().upcast(), game.upcast()]);
    }

    fn show_results(&self, game: &GameView) {
        let imp = self.imp();
        let results = ResultsView::new(&game.summary());

        results.connect_retry(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |results| window.play(results.config())
        ));
        results.connect_back(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| window.show_start()
        ));

        imp.navigation_view
            .replace(&[imp.start_view.clone().upcast(), results.upcast()]);
    }

    fn show_start(&self) {
        let imp = self.imp();
        imp.navigation_view
            .replace(&[imp.start_view.clone().upcast()]);
    }

    pub fn load_window_state(&self) {
        let settings = gio::Settings::new(&format!("{}.state.window", config::APPLICATION_ID));
        let (width, height) = settings.get::<(i32, i32)>("size");
        self.set_default_size(width, height);

        if settings.boolean("maximized") {
            self.maximize();
        }

        self.connect_maximized_notify(glib::clone!(
            #[strong]
            settings,
            move |window| {
                let _ = settings.set_boolean("maximized", window.is_maximized());
            }
        ));

        let save_size = glib::clone!(
            #[strong]
            settings,
            move |window: &Self| {
                if !window.is_maximized() {
                    let _ = settings.set("size", (window.default_width(), window.default_height()));
                }
            }
        );
        self.connect_default_width_notify(save_size.clone());
        self.connect_default_height_notify(save_size);
    }
}
