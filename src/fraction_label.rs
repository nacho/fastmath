use gtk::prelude::*;
use gtk::subclass::prelude::*;
use i18n_format::i18n_fmt;

use crate::fraction::Fraction;

mod imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/nacho/fastmath/ui/fraction_label.ui")]
    #[properties(wrapper_type = super::FractionLabel)]
    pub struct FractionLabel {
        #[template_child]
        pub whole_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub numerator_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub denominator_label: TemplateChild<gtk::Label>,

        #[property(get, set, construct_only)]
        pub fraction: Cell<Option<Fraction>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FractionLabel {
        const NAME: &'static str = "FastmathFractionLabel";
        type Type = super::FractionLabel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.set_layout_manager_type::<gtk::BoxLayout>();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FractionLabel {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            if let Some(layout) = obj.layout_manager().and_downcast::<gtk::BoxLayout>() {
                layout.set_spacing(6);
            }

            let fraction = obj
                .fraction()
                .expect("the fraction is a construct property");
            let (whole, numerator, denominator) =
                (fraction.whole, fraction.numerator, fraction.denominator);
            self.numerator_label.set_label(&numerator.to_string());
            self.denominator_label.set_label(&denominator.to_string());
            let description = if whole > 0 {
                self.whole_label.set_label(&whole.to_string());
                self.whole_label.set_visible(true);
                i18n_fmt! {
                    // Translators: How a screen reader reads a mixed number.
                    // The first {} is the whole part, the second {} the
                    // numerator and the third {} the denominator, for example
                    // "1 and 3 over 4" for 1 3/4.
                    i18n_fmt("{} and {} over {}", whole, numerator, denominator)
                }
            } else {
                i18n_fmt! {
                    // Translators: How a screen reader reads a fraction. The
                    // first {} is the numerator and the second {} the
                    // denominator, for example "3 over 4" for 3/4.
                    i18n_fmt("{} over {}", numerator, denominator)
                }
            };
            obj.update_property(&[gtk::accessible::Property::Label(&description)]);
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for FractionLabel {}
}

glib::wrapper! {
    /// A fraction shown with the numerator over the denominator, and the
    /// whole part of a mixed number to the left.
    pub struct FractionLabel(ObjectSubclass<imp::FractionLabel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl FractionLabel {
    pub fn new(fraction: &Fraction) -> Self {
        glib::Object::builder()
            .property("fraction", *fraction)
            .build()
    }
}
