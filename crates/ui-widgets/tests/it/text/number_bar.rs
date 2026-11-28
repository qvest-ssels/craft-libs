//! An options bar's number field: arrow keys add steps to the value as it is (`bar_steps`), and only the side label
//! scrubs (`label_scrub_only`). Plain fields keep the grid steps and the body scrub.

use craft_ui::contract::{Bind, Show as _, ShowContainer as _, SideLabel};
use craft_ui::form::Field;
use craft_ui::testing::pointer::moved;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::NumberField;
use egui::{CursorIcon, Key, Modifiers, vec2};
use std::cell::Cell;
use std::ops::RangeInclusive;
use std::rc::Rc;

/// How the field under test is set up.
#[derive(Clone, Copy)]
struct Setup {
    step: f64,
    range: (f64, f64),
    bar_steps: bool,
    label_scrub_only: bool,
}

/// An Opacity-like field: whole percents from 1 to 100.
const OPACITY: Setup = Setup { step: 1.0, range: (1.0, 100.0), bar_steps: true, label_scrub_only: false };
/// A Feather-like field: a tenth of a pixel from 0 to 1000.
const FEATHER: Setup = Setup { step: 0.1, range: (0.0, 1000.0), bar_steps: true, label_scrub_only: false };

/// A field named "Amount" with a scrubbing side label, at `start`.
fn field(setup: Setup, start: f64) -> (Fixture, Rc<Cell<Option<f64>>>) {
    let value = Rc::new(Cell::new(Some(start)));
    let shown = Rc::clone(&value);
    let range: RangeInclusive<f64> = setup.range.0..=setup.range.1;
    let mut fixture = Fixture::new(move |ui| {
        Field::new("Amount").side(SideLabel::new().scrub(true)).show(ui, |ui, _| {
            let mut current = shown.get();
            let mut field = NumberField::new("Amount", Bind::bound(&mut current)).step(setup.step).range(range.clone()).scrub_by_label();
            if setup.bar_steps {
                field = field.bar_steps();
            }
            if setup.label_scrub_only {
                field = field.label_scrub_only();
            }
            field.show(ui);
            shown.set(current);
        });
    });
    fixture.settle();
    (fixture, value)
}

/// The value after a click on the field at `start` and `key` with `modifiers`.
fn after_key(setup: Setup, start: f64, key: Key, modifiers: Modifiers) -> Option<f64> {
    let (mut fixture, value) = field(setup, start);
    let target = fixture.by_label("Amount").expect("the Amount field");
    fixture.click(&target).settle();
    fixture.key(key, modifiers).settle();
    value.get()
}

/// storytold/photocraft#16807: Shift+arrows add ten steps without going to a multiple first, Alt is the plain step,
/// and the range still holds.
#[trial]
fn bar_steps_add_to_the_value_as_it_is() {
    assert_eq!(after_key(OPACITY, 53.0, Key::ArrowUp, Modifiers::SHIFT), Some(63.0), "53, Shift+Up");
    assert_eq!(after_key(OPACITY, 57.0, Key::ArrowDown, Modifiers::SHIFT), Some(47.0), "57, Shift+Down");
    assert_eq!(after_key(OPACITY, 53.0, Key::ArrowUp, Modifiers::NONE), Some(54.0), "53, Up");
    assert_eq!(after_key(OPACITY, 50.0, Key::ArrowUp, Modifiers::ALT), Some(51.0), "50, Alt+Up");
    assert_eq!(after_key(OPACITY, 50.0, Key::ArrowDown, Modifiers::ALT), Some(49.0), "50, Alt+Down");
    assert_eq!(after_key(OPACITY, 98.0, Key::ArrowUp, Modifiers::SHIFT), Some(100.0), "98, Shift+Up clamps");
    assert_eq!(after_key(OPACITY, 3.0, Key::ArrowDown, Modifiers::SHIFT), Some(1.0), "3, Shift+Down clamps");
    assert_eq!(after_key(FEATHER, 10.0, Key::ArrowUp, Modifiers::NONE), Some(10.1), "Feather 10, Up");
    assert_eq!(after_key(FEATHER, 10.0, Key::ArrowUp, Modifiers::SHIFT), Some(11.0), "Feather 10, Shift+Up");
    assert_eq!(after_key(FEATHER, 10.05, Key::ArrowUp, Modifiers::SHIFT), Some(11.05), "Feather 10.05, Shift+Up");
}

/// Ctrl+Up, Ctrl+Down, Page Up and Page Down leave a bar's field as it is.
#[trial]
fn bar_steps_take_no_ctrl_or_page_key_steps() {
    for (key, modifiers) in
        [(Key::ArrowUp, Modifiers::CTRL), (Key::ArrowDown, Modifiers::CTRL), (Key::PageUp, Modifiers::NONE), (Key::PageDown, Modifiers::NONE)]
    {
        assert_eq!(after_key(OPACITY, 51.0, key, modifiers), Some(51.0), "{key:?} {modifiers:?}");
    }
}

/// Without `bar_steps` the field keeps its grid steps: Shift goes to the next multiple of ten, Page keys step.
#[trial]
fn a_plain_field_keeps_its_grid_steps() {
    let plain = Setup { bar_steps: false, ..OPACITY };
    assert_eq!(after_key(plain, 53.0, Key::ArrowUp, Modifiers::SHIFT), Some(60.0), "53, Shift+Up");
    assert_eq!(after_key(plain, 51.0, Key::PageUp, Modifiers::NONE), Some(60.0), "51, Page Up");
}

/// storytold/photocraft#16814: with `label_scrub_only` the pointer over the field's body is the text cursor and a drag
/// on it leaves the value and puts the field in edit; a drag on the label still scrubs, a point a step. A plain field
/// scrubs from its body under the resize cursor.
#[trial]
fn only_the_label_scrubs_a_bar_s_field() {
    let setup = Setup { label_scrub_only: true, ..OPACITY };
    let (mut fixture, value) = field(setup, 50.0);
    let body = fixture.by_label("Amount").expect("the Amount field");
    fixture.frame(vec![moved(body.center())]).settle();
    assert_eq!(fixture.cursor_icon(), CursorIcon::Text, "the text cursor over the body");
    fixture.drag_at(body.center(), vec2(30.0, 0.0)).settle();
    assert_eq!(value.get(), Some(50.0), "a drag on the body leaves the value");
    assert!(fixture.focused_stop().is_some(), "a drag on the body puts the field (the one stop) in edit");
    let caption = fixture.texts().iter().find(|text| text.text == "Amount").expect("the scrub caption").rect;
    fixture.drag_at(caption.center(), vec2(20.0, 0.0)).settle();
    assert_eq!(value.get(), Some(70.0), "a drag on the label scrubs a step a point");

    let (mut fixture, value) = field(OPACITY, 50.0);
    let body = fixture.by_label("Amount").expect("the Amount field");
    fixture.frame(vec![moved(body.center())]).settle();
    assert_eq!(fixture.cursor_icon(), CursorIcon::ResizeHorizontal, "the resize cursor over a plain field's body");
    fixture.drag_at(body.center(), vec2(30.0, 0.0)).settle();
    assert!(value.get().is_some_and(|value| value > 50.0), "a drag on a plain field's body scrubs: {:?}", value.get());
}
