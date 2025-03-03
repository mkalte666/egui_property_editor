//! A demo for the egui_property_editor

use eframe::Frame;
use eframe::emath::Align;
use egui::{CentralPanel, ComboBox, Context, Layout, RichText, ScrollArea};
use egui_property_editor::{
    Property, PropertyEditor, ValidatedProperty, ValidationError, enum_property, unit_enum_property,
};
use std::fmt::Formatter;

/// Entry point for the eframe app
fn main() {
    let n_o = eframe::NativeOptions::default();
    eframe::run_native(
        "Property editor example",
        n_o,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
    .unwrap()
}

/// The eframe app state - contains all the things we wanna modify
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug)]
struct App {
    some_string: String,
    some_other_string: String,
    c: String,
    d: String,
    e: String,
    f: String,
    an_int: i32,
    another_thing: usize,
    a_bool: bool,
    something_optional: Option<String>,
    optional_struct: Option<InnerThingWithDefault>,
    selection: UnitEnum,
    selection2: UnitEnum,
    value_enum: ValueEnum,
    value_enum2: ValueEnum,
}

#[derive(Debug, Default)]
#[allow(clippy::missing_docs_in_private_items)]
struct InnerThingWithDefault {
    string: String,
    number: i32,
}

#[derive(Debug, PartialEq)]
enum UnitEnum {
    OptionA,
    OptionB,
    OptionC,
}

impl std::fmt::Display for UnitEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UnitEnum::OptionA => "Option A",
                UnitEnum::OptionB => "Option B",
                UnitEnum::OptionC => "Option C",
            }
        )
    }
}

#[derive(Debug)]
enum ValueEnum {
    Nothing,
    Something(i32),
    More { a: i32, b: i32 },
}

impl std::fmt::Display for ValueEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueEnum::Nothing => write!(f, "Nothing"),
            ValueEnum::Something(val) => write!(f, "Something: {val}"),
            ValueEnum::More { .. } => write!(f, "More"),
        }
    }
}

impl App {
    /// Create a new editor
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        Self::default()
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            some_string: "I am a string".to_string(),
            some_other_string: "I am another string".to_string(),
            c: "".to_string(),
            d: "".to_string(),
            e: "".to_string(),
            f: "".to_string(),
            an_int: 123,
            another_thing: 0,
            a_bool: false,
            something_optional: None,
            optional_struct: None,
            selection: UnitEnum::OptionA,
            selection2: UnitEnum::OptionA,
            value_enum: ValueEnum::Nothing,
            value_enum2: ValueEnum::Nothing,
        }
    }
}

impl eframe::App for App {
    /// Update fn, as usual
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            // The demo will likely grow larger than the screen size, this adds scrolling.
            ScrollArea::vertical().show(ui, |ui| {
                // A property editor is created with new.
                // You then operate on it to create all the things to show.
                // This is the basic way - when you embedded this in a ui group yourself, and/or do not care about element sizes.
                // It is not the prettyest however.
                let first_valid = PropertyEditor::new("editor left")
                    // Have a look at the doc for all functions like this
                    .stripes(true)
                    .headline("Should be left")
                    // There are many ways to create a property.
                    // Most functions take an Into<Property>, so you can hand it things where there is a default implementation.
                    .named_property("A string", &mut self.some_string)
                    .named_property("Some other String", &mut self.some_other_string)
                    .headline("Another headline")
                    .named_property("C String", &mut self.c)
                    .named_property("D String", &mut self.d)
                    // Validation is done with a callback. As of writing this there aren't too many variants for the ValidationError.
                    // This is because you quite likely will want to write custom messages anyway.
                    .named_property(
                        "E String",
                        ValidatedProperty::new(&mut self.e, |val| {
                            if val.to_lowercase() == **val {
                                Ok(())
                            } else {
                                Err(ValidationError::CustomWithMessage(
                                    "Everything should be lowercase".to_string(),
                                ))
                            }
                        }),
                    )
                    // This exists
                    .named_property("F String", ValidatedProperty::unvalidated(&mut self.f))
                    // This functions shows, and returns true if all validations passed, or false if one or more failed.
                    // This will not give you *what* failed - since its a visual thing first, add a hint.
                    .show(ui);
                if first_valid {
                    ui.label("First property editor was valid");
                } else {
                    ui.label("Validation failed somewhere in the first property editor");
                }

                ui.separator();
                ui.vertical_centered(|ui| {
                    // The editor can handle center and right justified layouts, though it itself will always be top-down.
                    ui.strong("Same thing again, but centered");
                    // Same things as before...
                    let second_valid = PropertyEditor::new("editor center")
                        .stripes(true)
                        .outer_border(true)
                        .min_col_width(Some(125.0))
                        .headline("Should be left")
                        .named_property("A string", &mut self.some_string)
                        .named_property("Some other String", &mut self.some_other_string)
                        .headline(RichText::new("Another headline").strong())
                        // Note that tuples with (Into<WidgetText>,Into<Property>) and (Into<WidgetText>,Into<Property>,Into<WidgetText>)
                        // also impl Into<Property>, and add name and/or description. That will become important later!
                        .property(("C String", &mut self.c))
                        .property(("D String", &mut self.d, "The string with name d"))
                        // You could also do a custom widget. Note that the callback needs to return the Response of the widget.
                        .property((
                            "E String",
                            Property::from_widget_fn(|ui| ui.text_edit_multiline(&mut self.e)),
                            "with custom widget!",
                        ))
                        .named_property("F String", &mut self.f)
                        .headline("Numbers work as well, and can be validated")
                        // ... with a few more types ...
                        .named_property("An int", &mut self.an_int)
                        .named_property(
                            "Another thing",
                            ValidatedProperty::new(&mut self.another_thing, |val| {
                                if **val == 0 {
                                    Err(ValidationError::OutOfRange)
                                } else {
                                    Ok(())
                                }
                            }),
                        )
                        .headline("Optional things exist")
                        // Now this is a bit more complicated.
                        // Options can be added, with an additional checkbox to say if they are there or not.
                        // This is becomes important that tuples implement Into<Property>: You want to return a PropertyList (aka Vec<Property<'a>>).
                        // And this is much nicer than manually building everything with chain operators.
                        .optional_property(
                            "Include another string?",
                            &mut self.something_optional,
                            String::new(),
                            |s| [("Optional String", s).into()].into(),
                        )
                        // And all normal features can be used inside. You cam these, though not with the PropertyEditor convencience functions.
                        // Look at the `Property::new_optional` function, which PropertyEditor calls for this.
                        .optional_property_default(
                            "This one options a struct",
                            &mut self.optional_struct,
                            |s| {
                                [
                                    ("member string", &mut s.string).into(),
                                    (
                                        "member number, validated",
                                        ValidatedProperty::new(&mut s.number, |val| {
                                            if **val == 0 {
                                                Err(ValidationError::CustomWithMessage(
                                                    "No zero allowed".to_string(),
                                                ))
                                            } else {
                                                Ok(())
                                            }
                                        }),
                                    )
                                        .into(),
                                ]
                                .into()
                            },
                        )
                        // Next up: Enums. Unit enums are same as everywhere - rather simple combo boxes. You can just add that manually...
                        .property((
                            "enum, manually drawn",
                            Property::from_widget_fn(|ui| {
                                ComboBox::new("manually drawn combo", "")
                                    .selected_text(self.selection.to_string())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.selection,
                                            UnitEnum::OptionA,
                                            UnitEnum::OptionA.to_string(),
                                        );
                                        ui.selectable_value(
                                            &mut self.selection,
                                            UnitEnum::OptionB,
                                            UnitEnum::OptionB.to_string(),
                                        );
                                        ui.selectable_value(
                                            &mut self.selection,
                                            UnitEnum::OptionC,
                                            UnitEnum::OptionC.to_string(),
                                        );
                                    })
                                    .response
                            }),
                        ))
                        // However, as long as the enum implements Display, you can use this helper
                        .property((
                            "enum, with helper",
                            unit_enum_property!(
                                self.selection2,
                                UnitEnum::OptionA,
                                UnitEnum::OptionB,
                                UnitEnum::OptionC
                            ),
                        ))
                        // While you can manually do value enums, id not.
                        // You *still* need to impl display for this, or provide a display fn.
                        .property((
                            "value enum",
                            enum_property!(self.value_enum,
                                ValueEnum::Nothing => {
                                    default: ValueEnum::Nothing;
                                    properties: {
                                        vec![]
                                    };
                                },
                                ValueEnum::Something(val) => {
                                    default: ValueEnum::Something(123);
                                    properties: {
                                        // this block eventually just hands you val.
                                        // PropertyList is our goal here
                                        // You will get mut refs here, so this is fine:
                                        vec![("The value of Something",val).into()]
                                    }
                                },
                                ValueEnum::More{a,b} => {
                                    default: ValueEnum::More{a: 12, b: 34};
                                    properties: {[
                                        ("a",a).into(),
                                        ("b",b,"b descr").into(),
                                    ].into()}
                                }
                            ),
                        ))
                        // So for example, ValueEnums Display impl includes Somethings number. Dont want that?
                        .property((
                            "custom display",
                            enum_property!(
                                self.value_enum2,
                                custom_to_string,
                                ValueEnum::Nothing => {
                                    default: ValueEnum::Nothing;
                                    properties: {
                                        vec![]
                                    }
                                },
                                ValueEnum::Something(val) => {
                                    default: ValueEnum::Something(4321);
                                    properties: {
                                        [("something",val).into()].into()
                                    }
                                }
                                // you can also leave out properties. They will then not appear.
                            ),
                        ))
                        .show(ui);
                    if second_valid {
                        ui.label("Second property editor was valid");
                    } else {
                        ui.label("Validation failed somewhere in the second property editor");
                    }
                });
                // Finally, another one, just to show (and test) that it goes on the right as well.
                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                    ui.strong("And on the right ");
                    let _third_valid = PropertyEditor::new("editor right")
                        .stripes(true)
                        .outer_border(true)
                        .min_col_width(Some(125.0))
                        .headline("Should still be on the left")
                        .named_property("A string", &mut self.some_string)
                        .named_property("Some other String", &mut self.some_other_string)
                        .headline("Another headline")
                        .named_property("A Bool", &mut self.a_bool)
                        .show(ui);
                });
            });
        });
    }
}

fn custom_to_string(val: &ValueEnum) -> String {
    match val {
        ValueEnum::Something(_) => "Something".to_string(),
        rest => rest.to_string(),
    }
}
