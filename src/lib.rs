use egui::emath::Align;
use egui::{
    Align2, Color32, Context, Direction, DragValue, FontId, Grid, Id, Layout, Rect, Response,
    Stroke, StrokeKind, Ui, UiBuilder, Vec2, WidgetText,
};
use std::fmt::{Display, Formatter};

pub struct PropertyEditor<'a> {
    id: Id,
    show_descriptions: bool,
    show_stripes: bool,
    group_all: bool,
    min_column_width: Option<f32>,
    headline_spacing: Vec2,
    entries: Vec<EditorLine<'a>>,
}

impl<'a> PropertyEditor<'a> {
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

    pub fn show(mut self, ui: &mut Ui) -> bool {
        let mut store = PropertyEditorStore::load(ui.ctx(), self.id).unwrap_or_default();
        // the property editor is always left to right
        // however its position might vary depending on the layout.
        // The first pass must be left to right though, or we would not know the required size.
        let available_rect = ui.max_rect().intersect(ui.cursor());
        let ui_rect = if store.first_pass {
            available_rect
        } else {
            debug_assert_eq!(
                ui.layout().main_dir,
                Direction::TopDown,
                "Property editor must be within a top down layout"
            );
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
        store.store(ui.ctx(), self.id);

        validation_result
    }

    fn inner_ui(&mut self, ui: &mut Ui) -> bool {
        let mut validation_result = true;
        let mut entries = std::mem::take(&mut self.entries).into_iter().peekable();
        while let Some(entry) = entries.next() {
            match entry {
                EditorLine::Headline(line) => {
                    ui.add_space(self.headline_spacing.y);
                    ui.horizontal(|ui| {
                        ui.add_space(self.headline_spacing.x);
                        ui.label(line.clone());
                    });
                }
                EditorLine::Property(p) => {
                    let columns = if self.show_descriptions { 3 } else { 2 };
                    let grid = Grid::new(ui.next_auto_id())
                        .striped(self.show_stripes)
                        .num_columns(columns);
                    let grid = if let Some(width) = &self.min_column_width {
                        grid.min_col_width(*width)
                    } else {
                        grid
                    };
                    grid.show(ui, |ui| {
                        validation_result = validation_result & p.draw(ui, self.show_descriptions);
                        loop {
                            match entries.next_if(|e| matches!(e, EditorLine::Property(_))) {
                                Some(EditorLine::Property(p)) => {
                                    validation_result =
                                        validation_result & p.draw(ui, self.show_descriptions);
                                }
                                _ => break,
                            }
                        }
                    });
                }
            }
        }

        validation_result
    }

    pub fn stripes(mut self, show_stripes: bool) -> Self {
        self.show_stripes = show_stripes;
        self
    }

    pub fn outer_border(mut self, outer_border: bool) -> Self {
        self.group_all = outer_border;
        self
    }

    pub fn min_col_width(mut self, min_col_width: Option<f32>) -> Self {
        self.min_column_width = min_col_width;
        self
    }
    pub fn headline(mut self, text: impl Into<WidgetText>) -> Self {
        self.entries.push(EditorLine::Headline(text.into()));
        self
    }

    pub fn named_property(
        self,
        name: impl Into<WidgetText>,
        property: impl Into<Property<'a>>,
    ) -> Self {
        let property = property.into().name(name);
        self.property(property)
    }

    pub fn property(mut self, property: impl Into<Property<'a>>) -> Self {
        let property = property.into();
        self.show_descriptions = self.show_descriptions || property.description.is_some();
        self.entries.push(EditorLine::Property(property));
        self
    }

    pub fn optional_property<T>(
        self,
        name: impl Into<WidgetText>,
        value: &'a mut Option<T>,
        default: T,
        property_cb: impl FnOnce(&'a mut T) -> PropertyList<'a> + 'a,
    ) -> Self {
        self.property(Property::new_optional(
            name,
            None::<&str>,
            value,
            default,
            property_cb,
        ))
    }

    pub fn optional_property_default<T: Default>(
        self,
        name: impl Into<WidgetText>,
        value: &'a mut Option<T>,
        property_cb: impl FnOnce(&'a mut T) -> PropertyList<'a> + 'a,
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

enum EditorLine<'a> {
    Headline(WidgetText),
    Property(Property<'a>),
}

#[derive(Debug, Clone, Default)]
struct PropertyEditorStore {
    first_pass: bool,
    last_width: f32,
}

impl PropertyEditorStore {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data(|d| d.get_temp(id))
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}

type PropertyWidgetFn<'a> = dyn FnOnce(&mut Ui) -> Response + 'a;
type PropertyDrawFn<'a> = dyn FnOnce(
        &mut Ui,
        Option<WidgetText>,
        Option<WidgetText>,
        Result<(), ValidationError>,
        bool,
    ) -> bool
    + 'a;

pub struct Property<'a> {
    name: Option<WidgetText>,
    description: Option<WidgetText>,
    draw_fn: Box<PropertyDrawFn<'a>>,
    validation_result: Result<(), ValidationError>,
}

impl<'a> Property<'a> {
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

    pub fn new_optional<T>(
        name: impl Into<WidgetText>,
        description: Option<impl Into<WidgetText>>,
        value: &'a mut Option<T>,
        default: T,
        property_cb: impl FnOnce(&'a mut T) -> PropertyList<'a> + 'a,
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
                    ui.label(description);
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
                for p in property_cb(val) {
                    inner_validation_result = inner_validation_result & p.draw(ui, draw_descr);
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

    pub fn name(self, name: impl Into<WidgetText>) -> Self {
        Self {
            name: Some(name.into()),
            ..self
        }
    }

    pub fn description(self, description: impl Into<WidgetText>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }

    fn draw(self, ui: &mut Ui, draw_description: bool) -> bool {
        (self.draw_fn)(
            ui,
            self.name,
            self.description,
            self.validation_result,
            draw_description,
        )
    }
}

type PropertyList<'a> = Vec<Property<'a>>;

#[derive(Debug)]
pub enum ValidationError {
    OutOfRange,
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

pub struct ValidatedProperty<'a, T> {
    value: T,
    validation_cb: Box<dyn FnOnce(&T) -> Result<(), ValidationError> + 'a>,
}

impl<'a, T: 'a> ValidatedProperty<'a, T> {
    /// Creates a new, *unvalidated* property. It is unclear as to why you would use ValidatedProperty for that.
    pub fn unvalidated(value: T) -> Self {
        Self {
            value,
            validation_cb: Box::new(default_validation_cb::<T>),
        }
    }

    pub fn new(
        value: T,
        validation_cb: impl FnOnce(&T) -> Result<(), ValidationError> + 'a,
    ) -> Self {
        Self {
            value,
            validation_cb: Box::new(validation_cb),
        }
    }

    pub fn with_widget_cb(self, cb: impl FnOnce(T, &mut Ui) -> Response + 'a) -> Property<'a> {
        Property {
            validation_result: (self.validation_cb)(&self.value),
            ..Property::from_widget_fn(|ui| cb(self.value, ui))
        }
    }
}

impl<'a> From<&'a mut String> for Property<'a> {
    fn from(value: &'a mut String) -> Self {
        Self::from_widget_fn(|ui| ui.text_edit_singleline(value))
    }
}

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

numeric_impl!(u8, i8, u16, i16, u32, i32, u64, i64, usize, f32, f64);

impl<'a> From<&'a mut bool> for Property<'a> {
    fn from(value: &'a mut bool) -> Self {
        Self::from_widget_fn(|ui| ui.checkbox(value, ""))
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

fn default_validation_cb<T>(_val: &T) -> Result<(), ValidationError> {
    Ok(())
}

fn default_property_draw_fn<'a>(
    ui: &mut Ui,
    name: Option<WidgetText>,
    description: Option<WidgetText>,
    validation_result: Result<(), ValidationError>,
    draw_description: bool,
    widget_cb: Box<PropertyWidgetFn<'a>>,
) -> bool {
    if let Some(name) = name {
        ui.label(name);
    } else {
        ui.label("");
    }

    let resp = widget_cb(ui);

    if draw_description {
        if let Some(description) = description {
            ui.label(description);
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
