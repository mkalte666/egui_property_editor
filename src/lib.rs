//! # A property editor for egui
//!
//! This crate is a property editor for egui with minimal dependencies.
//!
//! # Usage
//!
//! The `PropertyEditor` itself follows somewhat of a builder pattern, where you create it, then add properties to it, and finally draw it.
//! ```
//! # use egui_property_editor::PropertyEditor;
//! # egui::__run_test_ui(|ui| {
//!     let mut property_1 = false;
//!     let mut property_2 = 123;
//!     PropertyEditor::new("my property editor")
//!         // change some properties
//!         .stripes(true)
//!         .outer_border(true)
//!         // add headlines to sections
//!         .headline("I am a headline")
//!         // Add some properties
//!         .named_property("A checkbox", &mut property_1)
//!         // Property lives off of `Into<Property>`, have a look at the `Property` docs to see what converts seamlessly.
//!         .property(("A number",&mut property_2))
//!         // and finally show it
//!         .show(ui);
//! # });
//! ```
use egui::emath::Align;
use egui::{Align2, Color32, Context, Direction, DragValue, FontId, FontSelection, Grid, Id, Layout, Rect, Response, Sense, Stroke, StrokeKind, TextEdit, TextWrapMode, Ui, UiBuilder, Vec2, Widget, WidgetText};
use std::fmt::{Display, Formatter};
use std::time::Duration;

/// A property editor is a builder that combines multiple `Property`s and drawing-related settings.
///
/// See the crate level documentation for a rough example, functions for details, and `examples/demo.rs` for detailed usage with comments.
pub struct PropertyEditor<'a> {
    /// The id salt to make memory persistent.
    id: Id,
    /// If any added properties have a description, this becomes true and indicates we need the thrid column.
    show_descriptions: bool,
    /// If the grid stripes are shown.
    show_stripes: bool,
    /// If the whole thing gets an outer border.
    group_all: bool,
    /// If this is Some, the grids get a minimum col width
    min_column_width: Option<f32>,
    /// The spacing of the headline entry. Might not be followed 100%.
    headline_spacing: Vec2,
    /// The list of entries used by this editor.
    entries: Vec<EditorLine<'a>>,
}

impl<'a> PropertyEditor<'a> {
    /// Create a new property editor, using `id_source` as a salt for the persistent id.
    pub fn new(id_source: impl Into<Id>) -> Self {
        Self {
            id: id_source.into(),
            show_descriptions: false,
            show_stripes: false,
            group_all: false,
            min_column_width: None,
            headline_spacing: Vec2::new(0.0, 5.0),
            entries: vec![],
        }
    }

    /// Show the property editor, consuming it.
    ///
    /// Will return `true` if all properties validated `Ok(())`, or `false` if one or more shows an error.
    pub fn show(self, ui: &mut Ui) -> bool {
        // Always use this layout, but copy the alignment (so we can be centered as it pleases).
        ui.with_layout(Layout::top_down(ui.layout().horizontal_align()), |ui| {
            self.show_outer(ui)
        })
        .inner
    }

    /// The outer part of show, after things are assured to be in a vertical layout.
    fn show_outer(mut self, ui: &mut Ui) -> bool {
        // should not happen, since show() assures a vertical layout. But who knows, and without all drawing dies.
        debug_assert_eq!(
            ui.layout().main_dir,
            Direction::TopDown,
            "Property editor must be within a top down layout"
        );

        let persistent_id = ui.make_persistent_id(self.id);
        let mut store = PropertyEditorStore::load(ui.ctx(), persistent_id).unwrap_or_default();
        // the property editor is always left to right
        // however its position might vary depending on the layout.
        // The first pass must be left to right though, or we would not know the required size.
        let available_rect = ui.available_rect_before_wrap().intersect(ui.cursor());
        let ui_rect = if store.first_pass {
            available_rect
        } else {
            match ui.layout().cross_align {
                Align::Min => available_rect,
                Align::Center => Rect::from_center_size(
                    available_rect.center(),
                    store.last_width * Vec2::X + available_rect.height() * Vec2::Y,
                ),
                Align::Max => Rect {
                    min: available_rect.min
                        + (available_rect.width()
                            - store.last_width
                            - ui.spacing().window_margin.right as f32)
                            * Vec2::X,
                    max: available_rect.max - ui.spacing().window_margin.right as f32 * Vec2::X,
                },
            }
        };

        // border?
        let inner_rect = if self.group_all {
            ui_rect.shrink(5.0)
        } else {
            ui_rect
        };

        let inner_layout = Layout::top_down(Align::Min);
        let ui_builder = UiBuilder::new().max_rect(inner_rect).layout(inner_layout);
        let mut inner_ui = ui.new_child(ui_builder);
        let validation_result = self.inner_ui(&mut inner_ui);

        let final_inner_rect = inner_ui.min_rect();
        let final_rect = if self.group_all {
            let final_rect = final_inner_rect.expand(5.0);
            ui.painter().rect_stroke(
                final_rect,
                2,
                ui.visuals().window_stroke,
                StrokeKind::Inside,
            );
            final_rect
        } else {
            final_inner_rect
        };
        ui.advance_cursor_after_rect(final_rect);
        
        // sizing pass?
        if store.first_pass || store.last_width != final_rect.width() {
            ui.ctx().request_discard("Property editor size changed");
        }
        store.first_pass = false;
        store.last_width = final_rect.width();
        store.store(ui.ctx(), persistent_id);

        validation_result
    }

    /// Shows the inner ui (i.e inside a possible border) for this.
    ///
    fn inner_ui(&mut self, ui: &mut Ui) -> bool {
        let headline_width = ui.available_width() * 0.9;
        let mut validation_result = true;
        let mut entries = std::mem::take(&mut self.entries).into_iter().peekable();
        let columns = if self.show_descriptions { 3 } else { 2 };
        let mut grid = Grid::new(ui.next_auto_id())
            .striped(self.show_stripes)
            .num_columns(columns);
        if let Some(width) = &self.min_column_width {
            let max_width = ui.available_width() / columns as f32 - ui.spacing().item_spacing.x * columns.saturating_sub(1) as f32;
            let width = width.min(max_width);
            grid = grid.min_col_width(width);
        }
        grid.show(ui, |ui| {
            while let Some(entry) = entries.next() {
                match entry {
                    EditorLine::Headline(line) => {
                        let text_pos = ui.cursor().min + Vec2::Y * ui.spacing().item_spacing.y;
                        let galley =
                            line.into_galley(ui, None, headline_width, FontSelection::Default);
                        ui.allocate_response(
                            Vec2::X * 1.0
                                + Vec2::Y
                                    * (galley.rect.height() + 2.0 * ui.spacing().item_spacing.y),
                            Sense::empty(),
                        );
                        ui.end_row();
                        ui.painter()
                            .galley(text_pos, galley.clone(), ui.visuals().text_color());
                    }
                    EditorLine::Property(p) => {
                        validation_result &= p.draw(ui, self.show_descriptions);
                        // usually id agree, but this is more readable IMO.
                        #[allow(clippy::while_let_loop)]
                        loop {
                            match entries.next_if(|e| matches!(e, EditorLine::Property(_))) {
                                Some(EditorLine::Property(p)) => {
                                    validation_result &= p.draw(ui, self.show_descriptions);
                                }
                                _ => break,
                            }
                        }
                    }
                }
            }
        });

        validation_result
    }

    /// Set to `true` if you want the inner grid to show stripes.
    pub fn stripes(mut self, show_stripes: bool) -> Self {
        self.show_stripes = show_stripes;
        self
    }

    /// Set to `true` if you want to show a border around the whole property editor.
    pub fn outer_border(mut self, outer_border: bool) -> Self {
        self.group_all = outer_border;
        self
    }

    /// Set the headline spacing, that is the distance of the headline to things.
    pub fn headline_spacing(mut self, spacing: impl Into<Vec2>) -> Self {
        self.headline_spacing = spacing.into();
        self
    }

    /// If you set this to some, will provide the inner grid with a minimal col width.
    /// Will look more aligned, but will of course also consume a bit more space.
    pub fn min_col_width(mut self, min_col_width: Option<f32>) -> Self {
        self.min_column_width = min_col_width;
        self
    }

    /// Set to true to always show description column
    pub fn show_descriptions(mut self, show_descriptions: bool) -> Self {
        self.show_descriptions = show_descriptions;
        self
    }

    /// Add a headline.
    ///
    /// As with all content-adding functions, insertion order matters.
    pub fn headline(mut self, text: impl Into<WidgetText>) -> Self {
        self.entries.push(EditorLine::Headline(text.into()));
        self
    }

    /// Add a property and assign it a name.
    ///
    /// As with all content-adding functions, insertion order matters.
    ///
    /// This is the same as creating it first, then calling `Property::name` on it, and then adding it with `PropertyEditor::property`.
    ///
    /// This takes a `Into<Property>`, so look at the `Property` docs to see what is possible.
    pub fn named_property(
        self,
        name: impl Into<WidgetText>,
        property: impl Into<Property<'a>>,
    ) -> Self {
        let property = property.into().name(name);
        self.property(property)
    }

    /// Add a property.
    ///
    /// As with all content-adding functions, insertion order matters.
    ///
    /// This takes a `Into<Property>`, so look at the `Property` docs to see what is possible.
    pub fn property(mut self, property: impl Into<Property<'a>>) -> Self {
        let property = property.into();
        self.show_descriptions = self.show_descriptions || property.description.is_some();
        self.entries.push(EditorLine::Property(property));
        self
    }

    /// Adds a property for an `Option<T>`.
    ///
    /// This function does two things:
    ///   * Create a checkbox, that sets the `Option<T>` to Some or None, using `default` to init.
    ///   * If the property is `Some`, calls `property_cb` with `&mut T`, which then can return one or more `Property`s in a `PropertyList`.
    ///
    /// To nest this, you can also create an optional property directly using `Property::new_optional`.
    ///
    /// For a detailed usage example, look at `examples/demo.rs`.
    pub fn optional_property<T>(
        self,
        name: impl Into<WidgetText>,
        value: &'a mut Option<T>,
        default: T,
        property_cb: impl FnOnce(&Ui, &'a mut T) -> PropertyList<'a> + 'a,
    ) -> Self {
        self.property(Property::new_optional(
            name,
            None::<&str>,
            value,
            default,
            property_cb,
        ))
    }

    /// Same as `Property::optional_property`, but with `T` implementing `Default` and using that instead of manually specifying that.
    pub fn optional_property_default<T: Default>(
        self,
        name: impl Into<WidgetText>,
        value: &'a mut Option<T>,
        property_cb: impl FnOnce(&Ui, &'a mut T) -> PropertyList<'a> + 'a,
    ) -> Self {
        self.property(Property::new_optional(
            name,
            None::<&str>,
            value,
            T::default(),
            property_cb,
        ))
    }
}

/// The internal storage the differentiates actual properties from section headlines
enum EditorLine<'a> {
    /// A headline, i.e. a section introducing text
    Headline(WidgetText),
    /// The actual property contents
    Property(Property<'a>),
}

/// The persistent memory needed to draw this whole thing
#[derive(Debug, Clone, Default)]
struct PropertyEditorStore {
    /// False on the first pass, used for discarding
    first_pass: bool,
    /// Used for ui allocation.
    last_width: f32,
}

impl PropertyEditorStore {
    /// Loads from temp storage
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data(|d| d.get_temp(id))
    }

    /// Stores to temp storage
    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}

/// The simpler of the two callback types for custom widget drawing.
///
/// Takes a &mut ui, returns a widget response.
pub type PropertyWidgetFn<'a> = dyn FnOnce(&mut Ui) -> Response + 'a;

/// The more complex, somewhat internal, drawing function.
///
/// Do not use if you can avoid it.
///
/// Takes all Property data and draws the thing. Do not forget to call `ui.next_row()` :)
pub type PropertyDrawFn<'a> = dyn FnOnce(
        &mut Ui,
        Option<WidgetText>,
        Option<WidgetText>,
        Result<(), ValidationError>,
        bool,
    ) -> bool
    + 'a;

/// An editable property.
///
/// A property can have a name and a description. It is drawn in a single row in a grid.
///
/// # Into property
///
/// There is an implementation of `From<T> for Property` for more or less all basic types, and `String`.
///   * `&mut u8,i8,..,f32,f64` integer and floating point types will become a `DragValue`.
///   * `&mut String` will become a single line edit.
///   * `&mut bool` will become a checkbox.
///
/// Additionally, two element tuples `(N, T)` where `N` is a `Into<WidgetText>` and `T` is a `Into<Property>` are equivalent to
/// ```ignore
///     let t = todo!("from somewhere");
///     t.into().name(n.into());
/// ```
/// And the same is true for three element tuples, with `(N,T,D)`, where `D` is an `Into<WidgetText>` for a description.
///
/// So you can do things like
/// ```ignore
///    PropertyEditor::new("editor")
///      .property(("name",&mut something))
///      .property(("else",&mut something_else, "description of else"));
/// ```
///
/// # Custom Widgets
///
/// If you want to create a property with a custom widget, you can create it with `Property::from_widget_fn`.
///
/// Note that the widget fn needs to follow `PropertyWidgetFn` (which is `FnOnce(&mut egui::Ui) -> egui::Response`).
///
/// You should only add *one* ui widget. If you need more, group them using layouts. The returned response is used to draw error highlighting.
///
/// # Custom Draw
///
/// **This is not recommended!**
///
/// You can also create a property with complete custom drawing.
///
/// This is mainly exposed due to the enum macros making use of it, but may be useful to some users as well.
///
/// In the end, the drawing is just a Boxed function that captures the mut ref to what will be modified.
///
/// It uses `PropertyDrawFn` - which has the following signature:
///```ignore
///pub type PropertyDrawFn<'a> = dyn FnOnce(
///         &mut Ui,
///         Option<WidgetText>,
///         Option<WidgetText>,
///         Result<(), ValidationError>,
///         bool,
///     ) -> bool
///     + 'a
///```
///
/// The first parameters is the target ui, followed by an optional name, description, the result of the validation (see `Validation` above), and bool indicating if there is a thrid column for the description in the first place.
/// The return value is "did validation succeed or not".
///
/// You need to
///   * Check if the name exists, and otherwise draw it empty.
///   * Draw your widget.
///   * Check if there is a description column. If yes, check if the description is there, or draw it empty. If not, there is no thrid column.
///   * `ui.end_row()` **is your responsibility when providing a custom draw function.**
///   * In the end, you must return true of false, indicating if the validation result is ok or not.
///
/// The end-row-point is why this function is a bit of a (visual) footgun. However, it allows you too add additional lines after this.
/// The enum macros and `Property::new_optional` use this to add content after initial combo boxes or checkboxes.
///
/// For detailed use, i recommend reading the source.
pub struct Property<'a> {
    /// The name of the property
    name: Option<WidgetText>,
    /// The description of this property
    description: Option<WidgetText>,
    /// The dynamic drawing function that will eventually be consumed to draw this property
    draw_fn: Box<PropertyDrawFn<'a>>,
    /// The result of a validation operation. Will usually be `Ok(())`, except if the `Property` is created out of a `ValidatedProperty`.
    validation_result: Result<(), ValidationError>,
}

impl<'a> Property<'a> {
    /// Create a new property from a callback that adds a widget to Ui, and returns the response of it.
    pub fn from_widget_fn(cb: impl FnOnce(&mut Ui) -> Response + 'a) -> Self {
        Self {
            name: None,
            description: None,
            draw_fn: Box::new(|ui, name, descr, valid, draw_descr| {
                default_property_draw_fn(ui, name, descr, valid, draw_descr, Box::new(cb))
            }),
            validation_result: Ok(()),
        }
    }

    /// Create a widget, but use custom drawing instead. Read the top level comment of `Property` (and, honestly, the source code of this file) for a detailed use of this.
    ///
    /// You probably do not wanna use this.
    pub fn from_custom_draw_fn(cb: Box<PropertyDrawFn<'a>>) -> Self {
        Self {
            name: None,
            description: None,
            draw_fn: cb,
            validation_result: Ok(()),
        }
    }

    /// For an `Option<T>`, create a new `Property` with a checkbox.
    ///
    /// If the checkbox is ticked, and thus `Option<T>` is `Some`, call `property_cb` with the inner `&mut T`.
    ///
    /// `property_cb` then returns a `PropertyList` - a `Vec<Property>` with zero or more properties for T.
    pub fn new_optional<T>(
        name: impl Into<WidgetText>,
        description: Option<impl Into<WidgetText>>,
        value: &'a mut Option<T>,
        default: T,
        property_cb: impl FnOnce(&Ui, &'a mut T) -> PropertyList<'a> + 'a,
    ) -> Self {
        let custom_draw_fn = move |ui: &mut Ui, name, description, _, draw_descr| -> bool {
            let mut cb = value.is_some();
            if let Some(name) = name {
                ui.label(name);
            } else {
                ui.label("");
            }
            ui.checkbox(&mut cb, "");
            if draw_descr {
                if let Some(description) = description {
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        ui.style_mut().wrap_mode = Some(TextWrapMode::Wrap);
                        ui.label(description)
                    });
                } else {
                    ui.label("");
                }
            }
            ui.end_row();

            if cb != value.is_some() {
                if cb {
                    *value = Some(default);
                } else {
                    *value = None;
                }
            }
            let mut inner_validation_result = true;
            if let Some(val) = value {
                for p in property_cb(ui, val) {
                    inner_validation_result &= p.draw(ui, draw_descr);
                }
            }

            inner_validation_result
        };
        Self {
            name: Some(name.into()),
            description: description.map(|x| x.into()),
            draw_fn: Box::new(custom_draw_fn),
            validation_result: Ok(()),
        }
    }

    /// Builder-style function to set the name of this property.
    pub fn name(self, name: impl Into<WidgetText>) -> Self {
        Self {
            name: Some(name.into()),
            ..self
        }
    }

    /// Builder-style function to set the description of this property.
    pub fn description(self, description: impl Into<WidgetText>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }

    /// Draw this property. Usually, you would not want to call this.
    /// Here be dragons etc.
    ///
    /// The only really valid place to call this in your code is if you have a custom drawing function, and let it produce additional properties.
    /// You would then want to draw these after your initial `ui.end_row()`. See the `Property` docs for this as well.
    pub fn draw(self, ui: &mut Ui, draw_description: bool) -> bool {
        (self.draw_fn)(
            ui,
            self.name,
            self.description,
            self.validation_result,
            draw_description,
        )
    }
}

/// An alias for `Vec<Property>` to recue `<>` in my code just a bit.
pub type PropertyList<'a> = Vec<Property<'a>>;

/// Should validation fail, these are the ways it will do so.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// A generic out of range message will be shown
    OutOfRange,
    /// A custom message
    CustomWithMessage(String),
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::OutOfRange => {
                write!(f, "The value is out of range.")
            }
            ValidationError::CustomWithMessage(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

/// The callback type used by validation callbacks.
pub type ValidationCb<'a, T> = dyn FnOnce(&T) -> Result<(), ValidationError> + 'a;

/// The helper struct for property validation.
///
/// See the `Validation` section inside the docs of `Property` and have a look at `examples/demo.rs` for a usage example.
pub struct ValidatedProperty<'a, T> {
    /// The value to be validated
    value: T,
    /// The callback that validates `value`
    validation_cb: Box<ValidationCb<'a, T>>,
}

impl<'a, T: 'a> ValidatedProperty<'a, T> {
    /// Creates a new, *unvalidated* property. It is unclear as to why you would use ValidatedProperty for that.
    pub fn unvalidated(value: T) -> Self {
        Self {
            value,
            validation_cb: Box::new(default_validation_cb::<T>),
        }
    }

    /// Creates a new validated property.
    ///
    /// The `validation_cb` is immediately called to populate the inner result.
    ///
    /// You can then transform this into a normal property with a custom widget function using `ValidatedProperty::with_widget_cb`.
    ///
    /// There is an `From<T> for Property` implementation for `ValidatedProperty`, as long T has that as well.
    /// So, this will work:
    ///
    /// ```
    /// # use egui_property_editor::{PropertyEditor, ValidatedProperty, ValidationError};
    /// # egui::__run_test_ui(|ui| {
    ///     let mut value = 123;
    ///     let _are_all_ok = PropertyEditor::new("editor")
    ///     .property(
    ///         ("this one is validated",
    ///         ValidatedProperty::new(&mut value, |val| {
    ///             // double deref due to this being a ref to a &mut
    ///             if **val == 0 {
    ///                Err((ValidationError::OutOfRange))
    ///             } else {
    ///                 Ok(())
    ///             }
    ///         })))
    ///     .show(ui);
    /// # })
    /// ```
    pub fn new(
        value: T,
        validation_cb: impl FnOnce(&T) -> Result<(), ValidationError> + 'a,
    ) -> Self {
        Self {
            value,
            validation_cb: Box::new(validation_cb),
        }
    }

    /// Turns this `ValidatedProperty` into a `Property` with a custom widget.
    pub fn with_widget_cb(self, cb: impl FnOnce(T, &mut Ui) -> Response + 'a) -> Property<'a> {
        Property {
            validation_result: (self.validation_cb)(&self.value),
            ..Property::from_widget_fn(|ui| cb(self.value, ui))
        }
    }
}

impl<'a> From<&'a mut String> for Property<'a> {
    fn from(value: &'a mut String) -> Self {
        Self::from_widget_fn(|ui| {
            ui.add(
                TextEdit::singleline(value)
                    .min_size(Vec2::X * 125.0)
                    .clip_text(true),
            )
        })
    }
}

/// A helper macro to add `From<T>` to `Property` for primitive types that allow `egui::DragValue`.
macro_rules! numeric_impl {
    ($t:ty) => {
        impl<'a> From<&'a mut $t> for Property<'a>
        {
            fn from(value: &'a mut $t) -> Self {
                Self::from_widget_fn(|ui| {
                    ui.add(DragValue::new(value))
                })
            }
        }
    };
    ($t:tt,$($rest:tt)*) => {
        numeric_impl!($t);
        numeric_impl!($($rest)*);
    }
}

numeric_impl!(u8, i8, u16, i16, u32, i32, u64, i64, usize, isize, f32, f64);

impl<'a> From<&'a mut bool> for Property<'a> {
    fn from(value: &'a mut bool) -> Self {
        Self::from_widget_fn(|ui| ui.checkbox(value, ""))
    }
}

impl<'a> From<&'a mut Duration> for Property<'a> {
    fn from(value: &'a mut Duration) -> Self {
        Self::from_widget_fn(|ui| {
            let mut secs = value.as_secs_f64();
            let step_size = if secs < 60.0 {
                if secs == 0.0 {
                    1.0
                } else if secs > 1e-9 {
                    10.0f64.powf(secs.log10().floor())
                } else {
                    1e-9
                }
            } else if secs < 60.0 * 60.0 {
                60.0
            } else if secs < 60.0 * 60.0 * 24.0 {
                60.0 * 60.0
            } else {
                60.0 * 60.0 * 24.0
            };
            // adjust for speed
            let speed = step_size / 25.0;
            let resp = DragValue::new(&mut secs)
                .speed(speed)
                .max_decimals(3)
                .range(0.0..=f64::MAX)
                .custom_formatter(|val, range| {
                    let decimals = range.max().unwrap_or(3);
                    // seconds and below
                    if val < 60.0 {
                        let (multiplier, unit) = if val == 0.0 {
                            (1.0, "s")
                        } else if val < 1e-6 {
                            (1e9, "ns")
                        } else if val < 1e-3 {
                            (1e6, "µs")
                        } else if val < 1.0 {
                            (1e3, "ms")
                        } else {
                            (1.0, "s")
                        };
                        format!(
                            "{value:.prec$} {unit}",
                            value = val * multiplier,
                            prec = decimals
                        )
                    } else if val < 60.0 * 60.0 {
                        let minutes = value.as_secs() / 60;
                        let secs = value.as_secs_f64() % 60.0;
                        format!("{minutes:0>2}:{secs:.2}")
                    } else if val < 60.0 * 60.0 * 24.0 {
                        let hours = value.as_secs() / (60 * 60);
                        let minutes = (value.as_secs() % (60 * 60)) / 60;
                        let secs = value.as_secs() % 60;
                        format!("{hours:0>2}:{minutes:0>2}:{secs:0>2}")
                    } else {
                        let days = value.as_secs() / (60 * 60 * 24);
                        let hours = (value.as_secs() % (60 * 60 * 24)) / (60 * 60);
                        let minutes = (value.as_secs() % (60 * 60)) / 60;
                        let secs = value.as_secs() % 60;
                        format!("{days:0>2}:{hours:0>2}:{minutes:0>2}:{secs:0>2}")
                    }
                })
                .custom_parser(|s| {
                    // simple case: just a number
                    s.parse::<f64>().ok().or_else(|| {
                        // case two: number + unit
                        let unit_split_pos = s.find(|s: char| {
                            (!s.is_ascii_digit() && s != 'e' && s != '-' && s != '.')
                                || s.is_whitespace()
                        });
                        let result = if let Some((left, right)) =
                            unit_split_pos.and_then(|pos| s.split_at_checked(pos))
                        {
                            let num = left.trim();
                            let unit = right.trim().to_lowercase();
                            num.parse::<f64>().ok().and_then(|num| match unit.as_str() {
                                "ns" => Some(num * 1e-9),
                                "us" | "µs" => Some(num * 1e-6),
                                "ms" => Some(num * 1e-3),
                                "" | "s" => Some(num),
                                "m" | "min" | "minutes" => Some(num * 60.0),
                                "h" | "hour" | "hours" => Some(num * 60.0 * 60.0),
                                "d" | "day" | "days" => Some(num * 60.0 * 60.0 * 24.0),
                                _ => None,
                            })
                        } else {
                            None
                        };
                        // last attempt: a:b:c format
                        result.or_else(|| {
                            let splits: Vec<_> = s.split(":").collect();
                            let (d, h, m, secs) = match splits.len() {
                                2 => (None, None, Some(splits[0]), Some(splits[1])),
                                3 => (None, Some(splits[0]), Some(splits[1]), Some(splits[2])),
                                4 => (
                                    Some(splits[0]),
                                    Some(splits[1]),
                                    Some(splits[2]),
                                    Some(splits[3]),
                                ),
                                _ => return None,
                            };
                            let seconds_d = if let Some(d) = d {
                                d.parse::<f64>().ok()? * 60.0 * 60.0 * 24.0
                            } else {
                                0.0
                            };
                            let seconds_h = if let Some(h) = h {
                                h.parse::<f64>().ok()? * 60.0 * 60.0
                            } else {
                                0.0
                            };
                            let seconds_m = if let Some(m) = m {
                                m.parse::<f64>().ok()? * 60.0
                            } else {
                                0.0
                            };
                            let seconds = if let Some(secs) = secs {
                                secs.parse::<f64>().ok()?
                            } else {
                                0.0
                            };
                            Some(seconds_d + seconds_h + seconds_m + seconds)
                        })
                    })
                })
                .ui(ui)
                .on_hover_text("Both d:m:h:s and <value> <unit> (such as 1h, 10s, 5ms) are valid.");
            *value = Duration::from_secs_f64(secs);
            resp
        })
    }
}

impl<'a, 'b, T> From<ValidatedProperty<'a, T>> for Property<'b>
where
    Property<'b>: From<T>,
{
    fn from(value: ValidatedProperty<'a, T>) -> Self {
        Self {
            validation_result: (value.validation_cb)(&value.value),
            ..<Property<'b> as From<T>>::from(value.value)
        }
    }
}

impl<'a, T, N> From<(N, T)> for Property<'a>
where
    T: Into<Property<'a>>,
    N: Into<WidgetText>,
{
    fn from(value: (N, T)) -> Self {
        value.1.into().name(value.0)
    }
}

impl<'a, T, N, D> From<(N, T, D)> for Property<'a>
where
    T: Into<Property<'a>>,
    N: Into<WidgetText>,
    D: Into<WidgetText>,
{
    fn from(value: (N, T, D)) -> Self {
        value.1.into().name(value.0).description(value.2)
    }
}

/// If you have an Option<String>, and want empty strings to result in None, you can use this helper wrapper.
pub struct EmptyStringIsNone<'a>(pub &'a mut Option<String>);

impl<'a> From<EmptyStringIsNone<'a>> for Property<'a> {
    fn from(value: EmptyStringIsNone<'a>) -> Self {
        Property::from_widget_fn(|ui| {
            let (response, empty) = {
                let name = value.0.get_or_insert("".to_string());
                (
                    TextEdit::singleline(name).clip_text(true).ui(ui),
                    name.is_empty(),
                )
            };
            if empty {
                *value.0 = None;
            }
            response
        })
    }
}

/// The default validation function just validates to `Ok(())`
fn default_validation_cb<T>(_val: &T) -> Result<(), ValidationError> {
    Ok(())
}

/// To reduce generated code, this is the default drawing of the widgets, as a free function.
fn default_property_draw_fn(
    ui: &mut Ui,
    name: Option<WidgetText>,
    description: Option<WidgetText>,
    validation_result: Result<(), ValidationError>,
    draw_description: bool,
    widget_cb: Box<PropertyWidgetFn<'_>>,
) -> bool {
    if let Some(name) = name {
        ui.label(name);
    } else {
        ui.label("");
    }

    let resp = widget_cb(ui);

    if draw_description {
        if let Some(description) = description {
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.style_mut().wrap_mode = Some(TextWrapMode::Wrap);
                ui.label(description)
            });
        } else {
            ui.label("");
        }
    }

    ui.end_row();

    match validation_result {
        Err(e) => {
            ui.painter().rect_stroke(
                resp.interact_rect,
                1,
                Stroke::new(2.0, Color32::DARK_RED),
                StrokeKind::Outside,
            );
            ui.painter().text(
                resp.interact_rect.right_center(),
                Align2::RIGHT_CENTER,
                "?",
                FontId::monospace(resp.interact_rect.height() * 0.9),
                Color32::DARK_RED,
            );
            resp.on_hover_text(e.to_string());
            false
        }
        Ok(_) => true,
    }
}

/// A helper macro to generate a property for a unit enum.
///
/// The rough syntax to use here is
///
/// `unit_enum_property!(<variable>, <variant to include 1>, <variant to include 2>, ...);`
///
/// This will generate a `Property` that you can then use as expected.
///
/// # Example usage
/// ```
/// # use egui_property_editor::unit_enum_property;
///
/// enum UnitEnum {
///     A,
///     B
/// }
///
/// impl std::fmt::Display for UnitEnum {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         match self {
///             UnitEnum::A => write!(f,"A"),
///             UnitEnum::B => write!(f,"B"),
///         }
///     }
/// }
///
/// let mut my_enum = UnitEnum::A;
/// let _property = egui_property_editor::unit_enum_property!(my_enum,
///     UnitEnum::A,
///     UnitEnum::B,
/// );
/// ```
#[macro_export]
macro_rules! unit_enum_property {
    ($value:expr, $($name:path),+ $(,)?) => {
         unit_enum_property!(@inner $value, $([$name]),*)
    };
    (@inner $value:expr, $([$($name_tt:tt)*]),*) => {
        $crate::Property::from_widget_fn(|ui| {
            // we need a way to arbitrarily take both references and values.
            use std::ops::DerefMut;
            let mut value = &mut $value;
            let value = value.deref_mut();
            egui::ComboBox::new(ui.next_auto_id(),"")
            .selected_text(value.to_string())
            .show_ui(ui,|ui| {
                $(
                    if ui.selectable_label(matches!(value,$($name_tt)*),$($name_tt)*.to_string()).clicked() {
                        *value = $($name_tt)*;
                    };
                )*
            }).response
        })
    };
}

/// enum_property is unit_enum_propertys big brother.
/// You can use it to generate properties on value enums.
///
/// # Custom display function
/// If your enum implements `std::fmt::Display`, and you like its output, you can omit the second argument.
/// However, if you want to change that, you can use a custom display function.
/// This can either be a reference to a trait method, or a free function.
///
/// i.e. `std::string::ToString::to_string` is fine, `my_string_fn` is fine, however `|x| x.my_fn()` would not be.
///
/// Take a look at `examples/demo.rs`, where i tried to show why you'd wanna use that.
///
/// # Example with syntax explaination
/// ```rust
/// # use egui_property_editor::enum_property;
/// enum Your {
///     Variant1,
///     Variant(i32,i32),
///     Named{fields: i32}
/// }
///
/// # impl std::fmt::Display for Your {
/// # fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #    write!(f,"")
/// # }
/// # }
///
/// let mut my_enum = Your::Variant1;
/// let property = enum_property!(
///     my_enum, // variable to use
///     // You could add a display function here.
///     // Note that you cannot add a closure, it must be a free function. See notes above.
///     // `my_example_string_formatter`,
///     Your::Variant1 => {
///         default: Your::Variant1; // Value to use when picking this in the dropdown. Will not overwrite if it already matches.
///         properties: {
///             // Generate a PropertyList here
///             vec![] // Empty is fine
///         }; // This ; is optional
///     }, // Comma is required, sorry.
///     // it hands you the fields as &mut
///     Your::Variant(with,fields) => {
///         default: Your::Variant(1,2);
///         properties: {
///             [
///                 ("with",with).into(),
///                 ("fields",fields).into(),
///             ].into()
///         }
///     }
/// );
/// ```
#[macro_export]
macro_rules! enum_property {
    ($value:expr, $($name:pat => {
        default: $default:expr;
        properties: $property_block:block$(;)?
    }
    ),+ $(,)*) => {
        enum_property!($value, std::string::ToString::to_string, $($name => {
            default: $default;
            properties: $property_block;
        },)+)
    };
    ($value:expr, $display_fn:expr, $($name:pat => {
        default: $default:expr;
        properties: $property_block:block$(;)?
    }
    ),+ $(,)*) => {
        $crate::Property::from_custom_draw_fn(Box::new(|ui,name,description,validation_result,include_description| {
            // we need a way to arbitrarily take both references and values.
            use std::ops::DerefMut;
            let mut value = &mut $value;
            let value = value.deref_mut();
            if let Some(name) = name {
                ui.label(name);
            } else {
                ui.label("");
            }

            egui::ComboBox::new(ui.next_auto_id(),"")
            .selected_text($display_fn(value))
            .show_ui(ui,|ui| {
                $(
                    {
                        let checked = match value {
                            #[allow(unused)]
                            $name => true,
                            _ => false,
                        };
                        let name = $display_fn(&$default);
                        if ui.selectable_label(checked,name).clicked() {
                            // do not reset if we click on an already clicked one
                            if !checked {
                                *value = $default
                            }
                        }
                    }
                )*
            });

            if include_description {
                if let Some(description) = description {
                    ui.label(description);
                } else {
                    ui.label("");
                }
            }
            ui.end_row();

            let p_list : $crate::PropertyList = match value {
                $($name => $property_block)*
                _ => vec![],
            };

            let mut valid = true;
            for property in p_list {
                valid &= property.draw(ui,include_description);
            }

            valid
        }))
    };
}
