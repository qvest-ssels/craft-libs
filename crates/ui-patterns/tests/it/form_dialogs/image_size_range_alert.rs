//! Image Size raises the "Closest value inserted." alert for a value typed in Width, Height or Resolution that the
//! row does not take as typed (photocraft#14778), and takes a resolution up to 29999.999.

use craft_behavior::PixelSize;
use craft_ui::contract::Role;
use craft_ui::dialog::{DialogHost, HostedAlert, StackDialog as _};
use craft_ui::patterns::form_dialogs::{HostedImageSize, ImageSize, ImageSizeValues, LinkedSize};
use craft_ui::runtime::env::{Env, UnitKind};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The measured document: 400 x 300 pixels at 72 ppi.
const SIZE: PixelSize = PixelSize::new(400, 300);

fn open(values: ImageSizeValues) -> (Rc<RefCell<DialogHost>>, Fixture) {
    let mut host = DialogHost::new();
    #[allow(clippy::redundant_closure, reason = "the builder must be generic over the bind's lifetime")]
    host.open(HostedImageSize::new(values, |values| ImageSize::new(values)));
    let host = Rc::new(RefCell::new(host));
    let shown = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| shown.borrow_mut().run(ui));
    fixture.room(vec2(1400.0, 1000.0)).idle(10);
    (host, fixture)
}

/// The decimal mark of the UI locale, which the fields read typed numbers with.
fn decimal_mark() -> char {
    let mark = Rc::new(std::cell::Cell::new('.'));
    let found = Rc::clone(&mark);
    let mut fixture = Fixture::new(move |ui| {
        let half = Env::of(ui).units().number_ungrouped(0.5, 1..=1);
        found.set(half.chars().find(|c| !c.is_ascii_digit()).unwrap_or('.'));
    });
    fixture.idle(1);
    mark.get()
}

/// Types `text` in the number field named `label`, a point standing for the locale's decimal mark, and leaves it with
/// Tab.
fn enter(fixture: &mut Fixture, label: &str, text: &str) {
    let text = text.replace('.', &decimal_mark().to_string());
    click_field(fixture, label);
    fixture.key(Key::A, Modifiers::COMMAND).type_text(&text);
    fixture.key(Key::Tab, Modifiers::NONE).settle();
}

/// Clicks the number field named `label`.
fn click_field(fixture: &mut Fixture, label: &str) {
    let field = fixture
        .by_role(Role::SPIN_BUTTON)
        .into_iter()
        .find(|target| fixture.tree().name_of(target.node) == Some(label))
        .unwrap_or_else(|| panic!("the {label} field"));
    fixture.click(&field).settle();
}

/// The message of the alert the dialog raised, once, with the decimal mark written as a point: the alert writes
/// numbers in the UI locale, and no digit groups, so a comma can only be the decimal mark.
fn alert(host: &Rc<RefCell<DialogHost>>) -> Option<String> {
    let alert = host.borrow_mut().get_mut::<HostedImageSize>().expect("Image Size is open").take_range_alert()?;
    let fields = HostedAlert::new(alert).json_fields().expect("an alert lists its message");
    Some(fields["message"].as_str().expect("the message is text").replace(',', "."))
}

/// A field of the dialog as automation reads it, in the unit the row shows.
fn field(host: &Rc<RefCell<DialogHost>>, key: &str) -> f64 {
    let fields = host.borrow().get::<HostedImageSize>().expect("Image Size is open").json_fields().expect("Image Size lists its fields");
    fields[key].as_f64().unwrap_or_else(|| panic!("{key} is a number"))
}

const PIXELS: &str = "An integer between 1 and 300000 is required. Closest value inserted.";

// Width in Pixels: 0, 300001 and 150.5 insert 1, 300000 and 151 with the integer alert, and Height follows; a width in
// range and text that is no number raise none.
#[trial]
fn a_width_in_pixels_past_its_range_or_a_fraction_raises_the_integer_alert() {
    let (host, mut fixture) = open(ImageSizeValues::new(SIZE));
    for (typed, width, height) in [("0", 1.0, 1.0), ("300001", 300_000.0, 225_000.0), ("150.5", 151.0, 113.0)] {
        enter(&mut fixture, "Width", typed);
        assert_eq!((field(&host, "width"), field(&host, "height")), (width, height), "{typed} inserts {width} and Height follows");
        assert_eq!(alert(&host).as_deref(), Some(PIXELS), "{typed} raises the integer alert");
        assert_eq!(alert(&host), None, "the alert is taken once");
    }
    enter(&mut fixture, "Width", "200");
    assert_eq!(field(&host, "width"), 200.0);
    assert_eq!(alert(&host), None, "a width in range raises no alert");
    enter(&mut fixture, "Width", "abc");
    assert_eq!(field(&host, "width"), 200.0, "letters leave the width");
    assert_eq!(alert(&host), None, "and raise no alert");
    enter(&mut fixture, "Height", "0");
    assert_eq!(alert(&host).as_deref(), Some(PIXELS), "Height alerts the same way");
    click_field(&mut fixture, "Width");
    fixture.key(Key::ArrowUp, Modifiers::NONE).settle();
    assert_eq!(field(&host, "width"), 2.0, "Up steps the width");
    assert_eq!(alert(&host), None, "a step raises no alert");
}

// Width in Percent alerts 0.25 to 75000.00 (1 to 300000 pixels of 400), Width in Inches at 72 ppi 0.014 to 4166.666.
#[trial]
fn a_width_in_percent_or_inches_alerts_the_range_at_the_units_decimals() {
    let (host, mut fixture) = open(ImageSizeValues::new(SIZE).unit(UnitKind::Percent));
    enter(&mut fixture, "Width", "0");
    assert_eq!(alert(&host).as_deref(), Some("A number between 0.25 and 75000.00 is required. Closest value inserted."));
    enter(&mut fixture, "Width", "50");
    assert_eq!(alert(&host), None, "50 % is in range");

    let (host, mut fixture) = open(ImageSizeValues::new(SIZE).unit(UnitKind::Inches));
    enter(&mut fixture, "Width", "5000");
    assert_eq!(alert(&host).as_deref(), Some("A number between 0.014 and 4166.666 is required. Closest value inserted."));
    enter(&mut fixture, "Width", "2.5");
    assert_eq!(alert(&host), None, "a fraction of an inch is no alert");
}

// Resolution takes 1 to 29999.999 pixels per inch: 0 and 30000 insert the ends with the alert.
#[trial]
fn resolution_takes_1_to_29999_999_and_alerts_past_it() {
    let (host, mut fixture) = open(ImageSizeValues::new(SIZE));
    let range = "A number between 1.000 and 29999.999 is required. Closest value inserted.";
    enter(&mut fixture, "Resolution", "0");
    assert_eq!(field(&host, "resolution"), 1.0);
    assert_eq!(alert(&host).as_deref(), Some(range));
    enter(&mut fixture, "Resolution", "30000");
    assert_eq!(field(&host, "resolution"), 29_999.999, "30000 is no longer taken");
    assert_eq!(alert(&host).as_deref(), Some(range));
    enter(&mut fixture, "Resolution", "300");
    assert_eq!(field(&host, "resolution"), 300.0);
    assert_eq!(alert(&host), None, "a resolution in range raises no alert");
}

// A document over the range opens at its end; the size model of the other dialogs keeps 30000.
#[trial]
fn only_image_size_limits_the_resolution_to_29999_999() {
    let (host, _fixture) = open(ImageSizeValues::new(SIZE).pixels_per_inch(30_000.0));
    assert_eq!(field(&host, "resolution"), 29_999.999, "a document over the range opens at its end");
    assert_eq!(*LinkedSize::new(SIZE).resolution_range().end(), 30_000.0, "New Document and the others keep 30000");
    assert_eq!(*LinkedSize::new(SIZE).max_resolution(29_999.999).resolution_range().end(), 29_999.999);
    assert_eq!(*LinkedSize::new(SIZE).max_resolution(40_000.0).resolution_range().end(), 30_000.0, "the default range is the limit");
}
