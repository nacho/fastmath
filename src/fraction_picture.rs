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

/// The colours of the pictures, the `fraction-*` named colours of
/// style.css.
#[derive(Clone, Copy, Debug)]
struct Colors {
    pizza: gdk::RGBA,
    pizza_edge: gdk::RGBA,
    chocolate: gdk::RGBA,
    chocolate_edge: gdk::RGBA,
    eaten: gdk::RGBA,
    eaten_edge: gdk::RGBA,
    grid: gdk::RGBA,
    grid_blank: gdk::RGBA,
    grid_edge: gdk::RGBA,
}

impl Default for Colors {
    /// Grey everywhere, until the colours are looked up.
    fn default() -> Self {
        let grey = gdk::RGBA::new(0.5, 0.5, 0.5, 1.0);
        Colors {
            pizza: grey,
            pizza_edge: grey,
            chocolate: grey,
            chocolate_edge: grey,
            eaten: grey,
            eaten_edge: grey,
            grid: grey,
            grid_blank: grey,
            grid_edge: grey,
        }
    }
}

mod imp {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::FractionPicture)]
    pub struct FractionPicture {
        #[property(get, set, construct_only)]
        pub picture: Cell<Option<Picture>>,
        pub(super) colors: Cell<Colors>,
        /// The StyleManager handlers, disconnected on dispose.
        pub style_handlers: RefCell<Vec<glib::SignalHandlerId>>,
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

            // The new style's CSS is only loaded after the notification, so
            // look the colours up again once idle.
            let refresh = glib::clone!(
                #[weak]
                obj,
                move |_: &adw::StyleManager| {
                    glib::idle_add_local_once(glib::clone!(
                        #[weak]
                        obj,
                        move || {
                            obj.imp().cache_colors();
                            obj.queue_draw();
                        }
                    ));
                }
            );
            let style = adw::StyleManager::default();
            *self.style_handlers.borrow_mut() = vec![
                style.connect_dark_notify(refresh.clone()),
                style.connect_high_contrast_notify(refresh),
            ];
            glib::idle_add_local_once(glib::clone!(
                #[weak]
                obj,
                move || {
                    obj.imp().cache_colors();
                    obj.queue_draw();
                }
            ));
        }

        fn dispose(&self) {
            let style = adw::StyleManager::default();
            for handler in self.style_handlers.take() {
                style.disconnect(handler);
            }
        }
    }

    impl FractionPicture {
        pub(super) fn cache_colors(&self) {
            #[allow(deprecated)]
            let ctx = self.obj().style_context();
            let lookup = |name: &str| {
                #[allow(deprecated)]
                ctx.lookup_color(name)
                    .unwrap_or(gdk::RGBA::new(0.5, 0.5, 0.5, 1.0))
            };
            self.colors.set(Colors {
                pizza: lookup("fraction-pizza-color"),
                pizza_edge: lookup("fraction-pizza-edge-color"),
                chocolate: lookup("fraction-chocolate-color"),
                chocolate_edge: lookup("fraction-chocolate-edge-color"),
                eaten: lookup("fraction-eaten-color"),
                eaten_edge: lookup("fraction-eaten-edge-color"),
                grid: lookup("fraction-grid-color"),
                grid_blank: lookup("fraction-grid-blank-color"),
                grid_edge: lookup("fraction-grid-edge-color"),
            });
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
            let colors = self.colors.get();
            for index in 0..picture.wholes {
                let x = index as f32 * (size + SPACING as f32);
                let bounds = graphene::Rect::new(x, 0.0, size, size);
                match picture.shape {
                    Shape::Pizza => draw_pizza(snapshot, &bounds, &picture, index, &colors),
                    Shape::Bar { rows, cols } => draw_bar(
                        snapshot,
                        &bounds,
                        rows,
                        cols,
                        picture.shaded_in(index),
                        &colors,
                    ),
                    Shape::Grid { rows, cols } => draw_grid(
                        snapshot,
                        &bounds,
                        rows,
                        cols,
                        picture.shaded_in(index),
                        &colors,
                    ),
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
            (Shape::Bar { .. }, _) => i18n_fmt! {
                // Translators: Describes a picture for screen readers. The
                // first {} is the number of squares and the second {} the
                // number of eaten squares, which picks the plural form, for
                // example "A chocolate bar of 8 squares, 3 of them eaten".
                i18n_nfmt("A chocolate bar of {} squares, {} of them eaten", "A chocolate bar of {} squares, {} of them eaten", eaten, pieces, eaten)
            },
            (Shape::Grid { .. }, _) => i18n_fmt! {
                // Translators: Describes a picture for screen readers. The
                // first {} is the number of squares and the second {} the
                // number of coloured squares, which picks the plural form,
                // for example "A grid of 8 squares, 3 of them coloured".
                i18n_nfmt("A grid of {} squares, {} of them coloured", "A grid of {} squares, {} of them coloured", eaten, pieces, eaten)
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

/// Draws pizza number `index` of `picture` in `bounds`: slices with a
/// solid outline, and eaten ones with a dashed outline.
fn draw_pizza(
    snapshot: &gtk::Snapshot,
    bounds: &graphene::Rect,
    picture: &Picture,
    index: u32,
    colors: &Colors,
) {
    let center = bounds.center();
    let radius = bounds.width() / 2.0 - LINE_WIDTH;
    let pieces = picture.pieces;
    let eaten = picture.shaded_in(index);
    let sweep = TAU / pieces as f32;

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
            snapshot.append_fill(&path, gsk::FillRule::Winding, &colors.eaten);
            snapshot.append_stroke(&path, &dashed, &colors.eaten_edge);
        } else {
            snapshot.append_fill(&path, gsk::FillRule::Winding, &colors.pizza);
            snapshot.append_stroke(&path, &solid, &colors.pizza_edge);
        }
    }
}

/// The square cells of a `rows` × `cols` shape, centred in `bounds`, row by
/// row.
fn cells(bounds: &graphene::Rect, rows: u32, cols: u32) -> Vec<graphene::Rect> {
    let side = (bounds.width() - 2.0 * LINE_WIDTH) / rows.max(cols) as f32;
    let left = bounds.x() + (bounds.width() - side * cols as f32) / 2.0;
    let top = bounds.y() + (bounds.height() - side * rows as f32) / 2.0;
    (0..rows)
        .flat_map(|row| {
            (0..cols).map(move |col| {
                graphene::Rect::new(
                    left + col as f32 * side,
                    top + row as f32 * side,
                    side,
                    side,
                )
            })
        })
        .collect()
}

/// A rounded square path inside `cell`, `inset` from its edges.
fn square(cell: &graphene::Rect, inset: f32, radius: f32) -> gsk::Path {
    let rect = cell.inset_r(inset, inset);
    let corner = graphene::Size::new(radius, radius);
    let rounded = gsk::RoundedRect::new(rect, corner, corner, corner, corner);
    let builder = gsk::PathBuilder::new();
    builder.add_rounded_rect(&rounded);
    builder.to_path()
}

/// Draws a chocolate bar: squares with darker edges, and eaten ones with a
/// dashed outline.
fn draw_bar(
    snapshot: &gtk::Snapshot,
    bounds: &graphene::Rect,
    rows: u32,
    cols: u32,
    eaten: u32,
    colors: &Colors,
) {
    let solid = gsk::Stroke::new(LINE_WIDTH);
    let dashed = gsk::Stroke::new(LINE_WIDTH);
    dashed.set_dash(&DASH);
    for (index, cell) in cells(bounds, rows, cols).iter().enumerate() {
        let path = square(cell, LINE_WIDTH, 4.0);
        if (index as u32) < eaten {
            snapshot.append_fill(&path, gsk::FillRule::Winding, &colors.eaten);
            snapshot.append_stroke(&path, &dashed, &colors.eaten_edge);
        } else {
            snapshot.append_fill(&path, gsk::FillRule::Winding, &colors.chocolate);
            snapshot.append_stroke(&path, &solid, &colors.chocolate_edge);
        }
    }
}

/// Draws a grid of squares, coloured or blank, with solid outlines.
fn draw_grid(
    snapshot: &gtk::Snapshot,
    bounds: &graphene::Rect,
    rows: u32,
    cols: u32,
    coloured: u32,
    colors: &Colors,
) {
    let stroke = gsk::Stroke::new(LINE_WIDTH / 1.5);
    for (index, cell) in cells(bounds, rows, cols).iter().enumerate() {
        let path = square(cell, 0.0, 0.0);
        let fill = if (index as u32) < coloured {
            &colors.grid
        } else {
            &colors.grid_blank
        };
        snapshot.append_fill(&path, gsk::FillRule::Winding, fill);
        snapshot.append_stroke(&path, &stroke, &colors.grid_edge);
    }
}
