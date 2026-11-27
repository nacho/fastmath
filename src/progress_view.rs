use gtk::prelude::*;
use gtk::subclass::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;

use crate::choice::Choice;
use crate::generator::Question;
use crate::labels;
use crate::operation::Operation;
use crate::practice::{FactHistory, weakness_step};

/// The sizes of the tables of products and quotients.
const TABLE_SIZES: [u32; 2] = [12, 20];
/// The largest number of the grids of sums and differences.
const SUM_SIZE: u32 = 20;

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
        #[template_child]
        pub size_group: TemplateChild<adw::ToggleGroup>,
        #[template_child]
        pub fact_grid: TemplateChild<gtk::Grid>,
        #[template_child]
        pub beyond_group: TemplateChild<adw::PreferencesGroup>,
        pub beyond_rows: RefCell<Vec<adw::ActionRow>>,
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
            for group in [&*self.operation_group, &*self.size_group] {
                group.connect_active_name_notify(glib::clone!(
                    #[weak]
                    view,
                    move |_| view.show_operation()
                ));
            }
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

    /// The largest number of the tables of products and quotients.
    fn table_size(&self) -> u32 {
        let name = self.imp().size_group.active_name().unwrap_or_default();
        TABLE_SIZES
            .into_iter()
            .find(|size| name == size.to_string())
            .unwrap_or(TABLE_SIZES[0])
    }

    fn show_operation(&self) {
        let imp = self.imp();
        let op = self.operation();
        let stats = imp.history.borrow().operation_stats(op);
        imp.summary_label
            .set_text(&labels::operation_progress(op, &stats));

        let table = matches!(op, Operation::Multiply | Operation::Divide);
        imp.size_group.set_visible(table);
        let (first, last) = if table {
            (2, self.table_size())
        } else {
            (1, SUM_SIZE)
        };
        self.fill_grid(op, first..=last);
        self.list_beyond(op, last);
    }

    /// Lists the weak facts of `op` that do not fit in a grid up to `max`.
    fn list_beyond(&self, op: Operation, max: u32) {
        let imp = self.imp();
        for row in imp.beyond_rows.take() {
            imp.beyond_group.remove(&row);
        }
        let facts = imp.history.borrow().weak_facts_beyond(op, max);
        imp.beyond_group.set_visible(!facts.is_empty());
        for (question, stats) in facts {
            let row = adw::ActionRow::builder()
                .title(question.solution())
                .subtitle(labels::fact_progress(&question, &stats))
                .build();
            imp.beyond_group.add(&row);
            imp.beyond_rows.borrow_mut().push(row);
        }
    }

    /// The fact in `row` and `column` of the grid of `op`, if any. Quotients
    /// are placed like the products they come from: 56 ÷ 7 is in row 7,
    /// column 8, like 7 × 8. Differences take the column from the row, so
    /// there are none above the diagonal.
    fn fact_at(op: Operation, row: u32, column: u32) -> Option<Question> {
        let (a, b, answer) = match op {
            Operation::Add => (row, column, row + column),
            Operation::Subtract if column > row => return None,
            Operation::Subtract => (row, column, row - column),
            Operation::Multiply => (row, column, row * column),
            Operation::Divide => (row * column, row, column),
        };
        Some(Question { op, a, b, answer })
    }

    /// Fills the grid with the facts of `op` for the numbers in `range`,
    /// coloured by weakness, with the numbers as headers.
    fn fill_grid(&self, op: Operation, range: std::ops::RangeInclusive<u32>) {
        let imp = self.imp();
        let grid = &imp.fact_grid;
        while let Some(child) = grid.first_child() {
            grid.remove(&child);
        }
        let header = |number: u32| {
            let label = gtk::Label::new(Some(&number.to_string()));
            label.add_css_class("caption");
            label.add_css_class("dim-label");
            label.add_css_class("numeric");
            label
        };
        let history = imp.history.borrow();
        for (index, number) in (1..).zip(range.clone()) {
            grid.attach(&header(number), index, 0, 1, 1);
            grid.attach(&header(number), 0, index, 1, 1);
        }
        for (y, row) in (1..).zip(range.clone()) {
            for (x, column) in (1..).zip(range.clone()) {
                let cell = gtk::Label::new(None);
                let Some(question) = Self::fact_at(op, row, column) else {
                    // Keeps the place of a fact that cannot be asked.
                    grid.attach(&cell, x, y, 1, 1);
                    continue;
                };
                cell.add_css_class("fact-cell");
                match history.fact_stats(&question) {
                    Some(stats) => {
                        cell.add_css_class(&format!("fact-weak-{}", weakness_step(stats.weakness)));
                        let description = labels::fact_progress(&question, &stats);
                        cell.set_tooltip_text(Some(&description));
                        cell.set_focusable(true);
                        cell.update_property(&[gtk::accessible::Property::Label(&description)]);
                    }
                    None => cell.add_css_class("fact-unseen"),
                }
                grid.attach(&cell, x, y, 1, 1);
            }
        }
    }
}
