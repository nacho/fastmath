use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, graphene, gsk};
use i18n_format::i18n_fmt;
use libadwaita as adw;
use std::f32::consts::{FRAC_PI_2, TAU};

use crate::fraction::{Picture, Shape};

/// The size of one shape, alone or next to another.
const SIZE: i32 = 200;
const SIZE_OF_TWO: i32 = 160;
const SPACING: i32 = 24;

const LINE_WIDTH: f32 = 3.0;
const DASH: [f32; 2] = [6.0, 5.0];

/// A colour from its red, green and blue bytes.
const fn rgb(red: u8, green: u8, blue: u8) -> gdk::RGBA {
    gdk::RGBA::new(
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
        1.0,
    )
}

// GNOME palette colours.
const YELLOW_3: gdk::RGBA = rgb(0xf6, 0xd3, 0x2d);
const YELLOW_5: gdk::RGBA = rgb(0xe5, 0xa5, 0x0a);
const LIGHT_3: gdk::RGBA = rgb(0xde, 0xdd, 0xda);
const LIGHT_5: gdk::RGBA = rgb(0x9a, 0x99, 0x96);
const DARK_2: gdk::RGBA = rgb(0x5e, 0x5c, 0x64);

mod imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::FractionPicture)]
    pub struct FractionPicture {
        #[property(get, set, construct_only)]
        pub picture: Cell<Option<Picture>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FractionPicture {
        const NAME: &'static str = "FastmathFractionPicture";
        type Type = super::FractionPicture;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_accessible_role(gtk::AccessibleRole::Img);
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FractionPicture {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_halign(gtk::Align::Center);
            obj.update_property(&[gtk::accessible::Property::Label(&obj.description())]);

            // Redraw with the colours of the new style.
            adw::StyleManager::default().connect_dark_notify(glib::clone!(
                #[weak]
                obj,
                move |_| obj.queue_draw()
            ));
        }
    }

    impl WidgetImpl for FractionPicture {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let picture = self.obj().shown();
            let wholes = picture.wholes as i32;
            let size = self.obj().shape_size();
            let length = match orientation {
                gtk::Orientation::Horizontal => wholes * size + (wholes - 1) * SPACING,
                _ => size,
            };
            (length, length, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let picture = obj.shown();
            let size = obj.shape_size() as f32;
            let dark = adw::StyleManager::default().is_dark();
            for index in 0..picture.wholes {
                let x = index as f32 * (size + SPACING as f32);
                let bounds = graphene::Rect::new(x, 0.0, size, size);
                match picture.shape {
                    Shape::Pizza => draw_pizza(snapshot, &bounds, &picture, index, dark),
                }
            }
        }
    }
}

glib::wrapper! {
    /// A picture for a fractions question: shapes cut into pieces, some of
    /// them eaten.
    pub struct FractionPicture(ObjectSubclass<imp::FractionPicture>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl FractionPicture {
    pub fn new(picture: &Picture) -> Self {
        glib::Object::builder()
            .property("picture", *picture)
            .build()
    }

    fn shown(&self) -> Picture {
        self.picture().expect("the picture is a construct property")
    }

    fn shape_size(&self) -> i32 {
        if self.shown().wholes > 1 {
            SIZE_OF_TWO
        } else {
            SIZE
        }
    }

    /// What the picture shows, for screen readers.
    pub fn description(&self) -> String {
        let picture = self.shown();
        let (pieces, eaten) = (picture.pieces, picture.shaded);
        match (picture.shape, picture.wholes) {
            (Shape::Pizza, 1) => i18n_fmt! {
                // Translators: Describes a picture for screen readers. The
                // first {} is the number of slices and the second {} the
                // number of eaten slices, which picks the plural form, for
                // example "A pizza in 8 slices, 3 of them eaten".
                i18n_nfmt("A pizza in {} slices, {} of them eaten", "A pizza in {} slices, {} of them eaten", eaten, pieces, eaten)
            },
            (Shape::Pizza, _) => i18n_fmt! {
                // Translators: Describes a picture for screen readers. The
                // first {} is the number of slices of each pizza and the
                // second {} the number of eaten slices of both pizzas, which
                // picks the plural form, for example "Two pizzas in 4 slices
                // each, 7 slices eaten".
                i18n_nfmt("Two pizzas in {} slices each, {} slice eaten", "Two pizzas in {} slices each, {} slices eaten", eaten, pieces, eaten)
            },
        }
    }
}

/// Draws pizza number `index` of `picture` in `bounds`: yellow slices with
/// an orange outline, and eaten ones in grey with a dashed outline.
fn draw_pizza(
    snapshot: &gtk::Snapshot,
    bounds: &graphene::Rect,
    picture: &Picture,
    index: u32,
    dark: bool,
) {
    let center = bounds.center();
    let radius = bounds.width() / 2.0 - LINE_WIDTH;
    let pieces = picture.pieces;
    let eaten = picture.shaded_in(index);
    let sweep = TAU / pieces as f32;

    let (eaten_fill, eaten_line) = if dark {
        (DARK_2, LIGHT_5)
    } else {
        (LIGHT_3, LIGHT_5)
    };
    let solid = gsk::Stroke::new(LINE_WIDTH);
    let dashed = gsk::Stroke::new(LINE_WIDTH);
    dashed.set_dash(&DASH);

    for slice in 0..pieces {
        let start = -FRAC_PI_2 + slice as f32 * sweep;
        let end = start + sweep;
        let builder = gsk::PathBuilder::new();
        builder.move_to(center.x(), center.y());
        builder.line_to(
            center.x() + radius * start.cos(),
            center.y() + radius * start.sin(),
        );
        builder.svg_arc_to(
            radius,
            radius,
            0.0,
            sweep > std::f32::consts::PI,
            true,
            center.x() + radius * end.cos(),
            center.y() + radius * end.sin(),
        );
        builder.close();
        let path = builder.to_path();

        if slice < eaten {
            snapshot.append_fill(&path, gsk::FillRule::Winding, &eaten_fill);
            snapshot.append_stroke(&path, &dashed, &eaten_line);
        } else {
            snapshot.append_fill(&path, gsk::FillRule::Winding, &YELLOW_3);
            snapshot.append_stroke(&path, &solid, &YELLOW_5);
        }
    }
}
