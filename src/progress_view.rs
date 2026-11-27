use gtk::prelude::*;
use gtk::subclass::prelude::*;
use libadwaita as adw;
use libadwaita::subclass::prelude::*;

use crate::choice::Choice;
use crate::labels;
use crate::operation::Operation;
use crate::practice::FactHistory;

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/nacho/fastmath/ui/progress_view.ui")]
    pub struct ProgressView {
        #[template_child]
        pub operation_group: TemplateChild<adw::ToggleGroup>,
        #[template_child]
        pub add_toggle: TemplateChild<adw::Toggle>,
        #[template_child]
        pub subtract_toggle: TemplateChild<adw::Toggle>,
        #[template_child]
        pub multiply_toggle: TemplateChild<adw::Toggle>,
        #[template_child]
        pub divide_toggle: TemplateChild<adw::Toggle>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub summary_label: TemplateChild<gtk::Label>,
        pub history: RefCell<FactHistory>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProgressView {
        const NAME: &'static str = "FastmathProgressView";
        type Type = super::ProgressView;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProgressView {
        fn constructed(&self) {
            self.parent_constructed();
            let view = self.obj();

            let history = FactHistory::load();
            let empty = history.is_empty();
            *self.history.borrow_mut() = history;
            self.stack
                .set_visible_child_name(if empty { "empty" } else { "progress" });
            self.operation_group.set_visible(!empty);

            for (op, toggle) in view.operation_toggles() {
                toggle.set_tooltip(&labels::operation_name(op));
            }
            self.operation_group
                .connect_active_name_notify(glib::clone!(
                    #[weak]
                    view,
                    move |_| view.show_operation()
                ));
            view.show_operation();
        }
    }

    impl WidgetImpl for ProgressView {}
    impl NavigationPageImpl for ProgressView {}
}

glib::wrapper! {
    /// How the player does with each arithmetic fact, from the practice
    /// history.
    pub struct ProgressView(ObjectSubclass<imp::ProgressView>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ProgressView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn operation_toggles(&self) -> [(Operation, &adw::Toggle); 4] {
        let imp = self.imp();
        [
            (Operation::Add, &*imp.add_toggle),
            (Operation::Subtract, &*imp.subtract_toggle),
            (Operation::Multiply, &*imp.multiply_toggle),
            (Operation::Divide, &*imp.divide_toggle),
        ]
    }

    /// The operation selected in the header bar.
    fn operation(&self) -> Operation {
        let name = self.imp().operation_group.active_name().unwrap_or_default();
        Operation::ALL
            .into_iter()
            .find(|op| name.starts_with(op.id()))
            .unwrap_or(Operation::Multiply)
    }

    fn show_operation(&self) {
        let imp = self.imp();
        let op = self.operation();
        let stats = imp.history.borrow().operation_stats(op);
        imp.summary_label
            .set_text(&labels::operation_progress(op, &stats));
    }
}
