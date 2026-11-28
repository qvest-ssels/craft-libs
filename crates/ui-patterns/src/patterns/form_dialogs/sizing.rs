//! The size dialogs `ImageSize` and `CanvasSize`: the rows, the readouts and the dialog around them that the two share,
//! and the `Resample` and `CanvasExtension` choices.

use super::FormDialog;
use super::size_model::{LinkedSize, MAX_DOCUMENT_SIDE};
use crate::contract::{Bind, Bracket, Entry, NoValue, Outcome, Props as _, Show as _, ShowContainer as _, Sizing};
use crate::dialog::{Alert, DialogResult, FieldRefusal, HeldAnswer};
use crate::display::Text;
use crate::form::{Field, FieldGroup};
use crate::layout_components::{Row, Stack};
use crate::pickers::UnitSelect;
use crate::runtime::env::{Env, Label, UnitKind, Units};
use crate::runtime::overlay::CloseReason;
use crate::text::{LinkMode, LinkedWords, TextWords, link_toggle};
use craft_behavior::PixelSize;
use craft_keymap::Key;
use craft_theme::variant::Tone;
use egui::{Response, Ui};
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;
use std::sync::Arc;

pub(crate) mod canvas;
pub(crate) mod image;
#[cfg(feature = "suite")]
pub(crate) mod stories;

pub use canvas::{CanvasSize, CanvasSizeOutput, CanvasSizeResult, CanvasSizeValues, HostedCanvasSize};
pub use image::{HostedImageSize, ImageSize, ImageSizeOutput, ImageSizeResult, ImageSizeValues};

craft_ui::words! {
    /// The words of the size dialogs beside the common OK and Cancel.
    pub struct SizingWords {
        image_size_title: "sizing.imageSizeTitle" = "Image Size";
        canvas_size_title: "sizing.canvasSizeTitle" = "Canvas Size";
        image_size: "sizing.imageSize" = "Image Size";
        /// `{size}` is the new file size, `{old}` the one the dialog opened with.
        size_was: template "sizing.sizeWas" = "{size} (was {old})";
        dimensions: "sizing.dimensions" = "Dimensions";
        /// `{width}` and `{height}` are the sides with their unit.
        dimensions_value: template "sizing.dimensionsValue" = "{width} × {height}";
        current_size: "sizing.currentSize" = "Current Size";
        new_size: "sizing.newSize" = "New Size";
        width: "sizing.width" = "Width";
        height: "sizing.height" = "Height";
        resolution: "sizing.resolution" = "Resolution";
        width_unit: "sizing.widthUnit" = "Width unit";
        height_unit: "sizing.heightUnit" = "Height unit";
        resolution_unit: "sizing.resolutionUnit" = "Resolution unit";
        resample: "sizing.resample" = "Resample";
        resample_method: "sizing.resampleMethod" = "Resample method";
        scale_styles: "sizing.scaleStyles" = "Scale Styles";
        relative: "sizing.relative" = "Relative";
        anchor: "sizing.anchor" = "Anchor";
        extension: "sizing.extension" = "Canvas extension color";
        /// Why a number of pixels cannot be typed while Resample is off.
        resample_off: "sizing.resampleOff" = "Turn on Resample to change the number of pixels.";
        /// `{max}` is the greatest side in pixels.
        too_large: template "sizing.tooLarge" = "Each side must be 1 to {max} pixels.";
        automatic: "sizing.automatic" = "Automatic";
        preserve_details: "sizing.preserveDetails" = "Preserve Details";
        bicubic_smoother: "sizing.bicubicSmoother" = "Bicubic Smoother (enlargement)";
        bicubic_sharper: "sizing.bicubicSharper" = "Bicubic Sharper (reduction)";
        bicubic: "sizing.bicubic" = "Bicubic (smooth gradients)";
        nearest_neighbor: "sizing.nearestNeighbor" = "Nearest Neighbor (hard edges)";
        bilinear: "sizing.bilinear" = "Bilinear";
        lanczos: "sizing.lanczos" = "Lanczos (sharp)";
        foreground: "sizing.foreground" = "Foreground";
        background: "sizing.background" = "Background";
        white: "sizing.white" = "White";
        black: "sizing.black" = "Black";
        gray: "sizing.gray" = "Gray";
        other: "sizing.other" = "Other";
        transparent: "sizing.transparent" = "Transparent";
    }
}

/// How the pixels are made when the size changes: Photoshop's Resample list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum Resample {
    /// The method suited to the change.
    Automatic,
    /// Enlarges and keeps the detail.
    PreserveDetails,
    /// Smooth gradients for an enlargement.
    BicubicSmoother,
    /// Sharp edges for a reduction.
    BicubicSharper,
    /// Smooth gradients.
    Bicubic,
    /// Hard edges.
    NearestNeighbor,
    /// The average of the four nearest pixels.
    Bilinear,
    /// Sharp results.
    Lanczos,
}

impl Resample {
    /// Every method, in the order of the list.
    pub const ALL: [Resample; 8] = [
        Resample::Automatic,
        Resample::PreserveDetails,
        Resample::BicubicSmoother,
        Resample::BicubicSharper,
        Resample::Bicubic,
        Resample::NearestNeighbor,
        Resample::Bilinear,
        Resample::Lanczos,
    ];

    /// The Alt+digit keys that pick a method, Photoshop's: Alt+3 (Preserve Details 2.0 there) picks nothing and
    /// Lanczos has no key. The dialog takes every one of them, also when it picks nothing, so no digit reaches a
    /// focused field.
    pub const KEYS: [(Key, Option<Resample>); 8] = [
        (Key::Num1, Some(Resample::Automatic)),
        (Key::Num2, Some(Resample::PreserveDetails)),
        (Key::Num3, None),
        (Key::Num4, Some(Resample::BicubicSmoother)),
        (Key::Num5, Some(Resample::BicubicSharper)),
        (Key::Num6, Some(Resample::Bicubic)),
        (Key::Num7, Some(Resample::NearestNeighbor)),
        (Key::Num8, Some(Resample::Bilinear)),
    ];

    /// The method's name in the list.
    pub fn label(self, words: &SizingWords) -> Label {
        match self {
            Resample::Automatic => words.automatic.clone(),
            Resample::PreserveDetails => words.preserve_details.clone(),
            Resample::BicubicSmoother => words.bicubic_smoother.clone(),
            Resample::BicubicSharper => words.bicubic_sharper.clone(),
            Resample::Bicubic => words.bicubic.clone(),
            Resample::NearestNeighbor => words.nearest_neighbor.clone(),
            Resample::Bilinear => words.bilinear.clone(),
            Resample::Lanczos => words.lanczos.clone(),
        }
    }

    /// The list entries of the methods in `allowed`, in the order given; all of them in the order of
    /// [`Resample::ALL`] when `allowed` names none.
    pub fn entries(words: &SizingWords, allowed: &[Resample]) -> Vec<Entry<Resample>> {
        let methods = if allowed.is_empty() { &Resample::ALL[..] } else { allowed };
        methods.iter().map(|&method| Entry::new(method, method.label(words))).collect()
    }
}

/// What fills the area a larger canvas adds. Serialised camelCase (`"background"`, `{"other": [r, g, b]}`), the
/// form automation reads and writes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum CanvasExtension {
    /// The foreground colour.
    Foreground,
    /// The background colour.
    Background,
    /// White.
    White,
    /// Black.
    Black,
    /// Mid gray.
    Gray,
    /// A colour the app chose, as red, green and blue between 0 and 1.
    Other([f32; 3]),
    /// Nothing: the added area is transparent (a document with no background).
    Transparent,
}

impl CanvasExtension {
    /// The extension's name in the list.
    pub fn label(&self, words: &SizingWords) -> Label {
        match self {
            CanvasExtension::Foreground => words.foreground.clone(),
            CanvasExtension::Background => words.background.clone(),
            CanvasExtension::White => words.white.clone(),
            CanvasExtension::Black => words.black.clone(),
            CanvasExtension::Gray => words.gray.clone(),
            CanvasExtension::Other(_) => words.other.clone(),
            CanvasExtension::Transparent => words.transparent.clone(),
        }
    }

    /// The list entries for a canvas whose current extension is `current`: the five named colours, `Other` while it is
    /// the current one, and Transparent for a document with no background (`has_background` false).
    pub fn entries(words: &SizingWords, current: &CanvasExtension, has_background: bool) -> Vec<Entry<CanvasExtension>> {
        Self::offered(current, has_background).into_iter().map(|extension| Entry::new(extension.clone(), extension.label(words))).collect()
    }

    /// The extensions the list offers, in its order: what [`entries`](Self::entries) shows and automation may pick.
    fn offered(current: &CanvasExtension, has_background: bool) -> Vec<CanvasExtension> {
        let mut list =
            vec![CanvasExtension::Foreground, CanvasExtension::Background, CanvasExtension::White, CanvasExtension::Black, CanvasExtension::Gray];
        if matches!(current, CanvasExtension::Other(_)) {
            list.push(current.clone());
        }
        if !has_background {
            list.push(CanvasExtension::Transparent);
        }
        list
    }
}

/// What a size dialog reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct SizingOutput<R> {
    /// The dialog's response.
    pub response: Response,
    /// How the dialog closed in this pass; `None` while it stays open.
    pub closed: Option<CloseReason>,
    /// The answer, the pass the dialog closed as OK.
    pub result: Option<R>,
    /// The range alert of a value typed in a row and replaced by the closest value it takes, in the pass it was
    /// committed (Image Size only).
    pub(crate) range_alert: Option<SizeAlert>,
}

impl<R> SizingOutput<R> {
    /// The answer for a [`DialogHost`](crate::dialog::DialogHost) entry: `Ok` with the result the pass the dialog closed
    /// as OK, `Cancel` the pass it closed any other way, `None` while it is open.
    pub(crate) fn hosted(self) -> Option<DialogResult<R>> {
        match (self.result, self.closed) {
            (Some(result), _) => Some(DialogResult::Ok(result)),
            (None, Some(_)) => Some(DialogResult::Cancel),
            (None, None) => None,
        }
    }
}

impl<R> Outcome for SizingOutput<R> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The state a size dialog edits.
pub(super) trait SizeForm: Clone + PartialEq + Send + Sync + 'static {
    /// The pixel size the dialog would answer with.
    fn pixels(&self) -> PixelSize;
}

/// Whether both sides are 1 to `MAX_DOCUMENT_SIDE` pixels.
fn size_valid(size: PixelSize) -> bool {
    let side = |pixels: u32| (1.0..=MAX_DOCUMENT_SIDE).contains(&f64::from(pixels));
    side(size.width) && side(size.height)
}

/// The automation fields of the width and height rows both dialogs share: `width` and `height` in the unit their
/// field shows, as typing in the field is; `unit`, the shared unit; and `widthUnit`, the unit the width shows.
fn size_json(size: &LinkedSize) -> serde_json::Map<String, serde_json::Value> {
    let mut fields = serde_json::Map::new();
    fields.insert("width".into(), serde_json::json!(size.width()));
    fields.insert("height".into(), serde_json::json!(size.height()));
    fields.insert("unit".into(), serde_json::json!(size.unit()));
    fields.insert("widthUnit".into(), serde_json::json!(size.width_unit()));
    fields
}

/// Sets one of [`size_json`]'s fields through the setter its row calls; `None` when `field` is not one of them. A
/// side whose row is disabled (pixels with Resample off) refuses as inactive.
fn set_size_json(size: &mut LinkedSize, field: &str, value: &serde_json::Value) -> Option<Result<(), String>> {
    let rejects = || FieldRefusal::Rejects(value).message(field);
    let kind = || serde_json::from_value::<UnitKind>(value.clone()).map_err(|_| rejects());
    Some(match field {
        "width" | "height" => {
            let editable = if field == "width" { size.is_width_editable() } else { size.is_editable() };
            if !editable {
                return Some(Err(FieldRefusal::Inactive.message(field)));
            }
            let Some(number) = value.as_f64() else { return Some(Err(rejects())) };
            if field == "width" {
                size.set_width(number);
            } else {
                size.set_height(number);
            }
            Ok(())
        }
        "unit" => kind().and_then(|kind| if size.set_unit(kind) { Ok(()) } else { Err(rejects()) }),
        "widthUnit" => kind().and_then(|kind| if size.set_width_unit(kind) { Ok(()) } else { Err(rejects()) }),
        _ => return None,
    })
}

/// A size dialog on `FormDialog`: `rows` edits the values and `answer` makes the result from them at OK. OK, Cancel
/// and Alt-for-Reset follow `ParamDialog`; OK is refused while the pixel size is empty or over the limit. With `held`,
/// OK files the values there for the host and the form shows the host's refusal ([`FormDialog::held`]).
pub(super) fn show_form<'a, S: SizeForm + Default, R>(
    ui: &mut Ui,
    title: Label,
    open: Option<Bind<'a, bool>>,
    values: Bind<'a, S>,
    held: Option<&'a mut HeldAnswer<S>>,
    mut rows: impl FnMut(&mut Ui, &mut S),
    answer: impl Fn(&S) -> R,
) -> SizingOutput<R> {
    let mut dialog = FormDialog::new(title.clone(), values).outer_gesture(true);
    if let Some(held) = held {
        dialog = dialog.held(held);
    }
    if let Some(open) = open {
        dialog = dialog.opened(open);
    }
    let shown = dialog.show(ui, |ui, _, state| {
        FieldGroup::new(title.clone()).show(ui, |ui, _| rows(ui, state));
        size_valid(state.pixels())
    });
    SizingOutput { response: shown.response, closed: shown.closed, result: shown.accepted.map(|state| answer(&state)), range_alert: None }
}

/// A value typed in an Image Size row that the row did not take as typed: one past the row's range, or a fraction in
/// a row of whole pixels. The row inserted the closest value it takes; the alert names the range.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SizeAlert {
    /// The values the row takes, in its unit.
    range: RangeInclusive<f64>,
    /// The digits after the decimal mark the range's ends are written with; `None` when the row takes whole numbers.
    decimals: Option<u8>,
}

impl SizeAlert {
    /// The alert of a typed commit in a row of `kind` over `range`: `held` when the row held the number to its range,
    /// `typed` the number it committed. Only the units whose alert is known have one: whole Pixels (which also alert
    /// a fraction), Percent and Inches, and the resolution in pixels per inch.
    fn of(kind: UnitKind, resolution: bool, range: RangeInclusive<f64>, held: bool, typed: Option<f64>) -> Option<Self> {
        let decimals = match (kind, resolution) {
            (UnitKind::Pixels, false) => None,
            (UnitKind::Percent, false) => Some(2),
            (UnitKind::Inches, false) | (UnitKind::PixelsPerInch, true) => Some(3),
            _ => return None,
        };
        let fraction = decimals.is_none() && typed.is_some_and(|number| number.is_finite() && number.fract() != 0.0);
        (held || fraction).then_some(SizeAlert { range, decimals })
    }

    /// The alert titled `title`, in the words and number format of `env`: "An integer between 1 and 300000 is
    /// required. Closest value inserted.", or "A number between 0.014 and 4166.666 …" at the row's decimals. The ends
    /// are rounded inward to those decimals, so each lies in the range, and carry no digit groups and no unit.
    pub(crate) fn alert(&self, title: Label, env: &Env) -> Alert<'static> {
        let words = env.words::<TextWords>();
        let units = env.units();
        let digits = self.decimals.unwrap_or(0);
        let scale = 10f64.powi(i32::from(digits));
        // The margin keeps an end on its digits (0.25 % is 25.000000000000004 hundredths) from moving a digit inward.
        let low = ((self.range.start() * scale) - 1e-6).ceil() / scale;
        let high = ((self.range.end() * scale) + 1e-6).floor() / scale;
        let shown = |n: f64| units.number_ungrouped(n, usize::from(digits)..=usize::from(digits));
        let (low, high) = (shown(low), shown(high));
        let sentence = if self.decimals.is_none() { &words.range_integer } else { &words.range_number };
        Alert::new(title, sentence.fill(&[("low", low.as_str()), ("high", high.as_str())]))
    }
}

/// The rows both dialogs show, with the words and units they need.
pub(super) struct Kit {
    words: Arc<SizingWords>,
    units: Units,
    link: Label,
    lengths: Vec<Entry<UnitKind>>,
    /// `lengths` and Columns: what the width offers when the values carry a column size.
    widths: Vec<Entry<UnitKind>>,
    resolutions: Vec<Entry<UnitKind>>,
}

/// The length units in Canvas Size order.
const LENGTH_UNITS: [UnitKind; 7] =
    [UnitKind::Pixels, UnitKind::Percent, UnitKind::Inches, UnitKind::Centimetres, UnitKind::Millimetres, UnitKind::Points, UnitKind::Picas];

/// Image Size lists Percent first in both dimension menus.
const IMAGE_LENGTH_UNITS: [UnitKind; 7] =
    [UnitKind::Percent, UnitKind::Pixels, UnitKind::Inches, UnitKind::Centimetres, UnitKind::Millimetres, UnitKind::Points, UnitKind::Picas];

/// The units the resolution is shown in.
const RESOLUTION_UNITS: [UnitKind; 2] = [UnitKind::PixelsPerInch, UnitKind::PixelsPerCentimetre];

/// Which of the width, height and resolution a row edits.
#[derive(Clone, Copy)]
enum Dimension {
    Width,
    Height,
    Resolution,
}

impl Kit {
    /// The rows of Image Size.
    pub(super) fn new(ui: &Ui) -> Self {
        Self::with_length_order(ui, &IMAGE_LENGTH_UNITS)
    }

    /// The rows of Canvas Size.
    pub(super) fn for_canvas(ui: &Ui) -> Self {
        Self::with_length_order(ui, &LENGTH_UNITS)
    }

    fn with_length_order(ui: &Ui, length_order: &[UnitKind]) -> Self {
        let env = Env::of(ui);
        let units = env.units();
        let entries = |kinds: &[UnitKind]| -> Vec<Entry<UnitKind>> { kinds.iter().map(|kind| Entry::new(*kind, units.name(*kind))).collect() };
        let lengths = entries(length_order);
        let mut widths = lengths.clone();
        widths.extend(entries(&[UnitKind::Columns]));
        Kit {
            words: env.words::<SizingWords>(),
            link: LinkMode::Ratio.label(&env.words::<LinkedWords>()),
            lengths,
            widths,
            resolutions: entries(&RESOLUTION_UNITS),
            units,
        }
    }

    pub(super) fn words(&self) -> &SizingWords {
        &self.words
    }

    /// A line of text named `label`.
    pub(super) fn readout(&self, ui: &mut Ui, label: &Label, text: String, tone: Option<Tone>) {
        Field::new(label.clone()).show(ui, |ui, _| {
            let text = Text::new(text.clone());
            match tone {
                Some(tone) => text.tone(tone).show(ui),
                None => text.show(ui),
            };
        });
    }

    /// The size of a flat picture of `size` pixels at `bytes_per_pixel`, as a file size. A row takes whole bytes, so a
    /// Bitmap row of `w` pixels at 1/8 byte each takes `ceil(w / 8)` bytes.
    pub(super) fn file_size(&self, size: PixelSize, bytes_per_pixel: f64) -> String {
        let bytes = (f64::from(size.width) * bytes_per_pixel).ceil() * f64::from(size.height);
        self.units.file_size(bytes.max(0.0).round() as u64)
    }

    /// `size` as "width × height" with the pixel unit.
    pub(super) fn dimensions(&self, size: PixelSize) -> String {
        let (width, height) = (self.units.pixels(f64::from(size.width)), self.units.pixels(f64::from(size.height)));
        self.words.dimensions_value.fill(&[("width", width.as_str()), ("height", height.as_str())])
    }

    /// `pixels` in the pixel unit.
    pub(super) fn pixels(&self, pixels: u32) -> String {
        self.units.pixels(f64::from(pixels))
    }

    /// The readout of a new size: its text, in the tone of an error while the size cannot be answered.
    pub(super) fn tone_of(&self, size: PixelSize) -> Option<Tone> {
        (!size_valid(size)).then_some(Tone::Bad)
    }

    /// The line that says why OK is refused, when it is.
    pub(super) fn refusal(&self, ui: &mut Ui, size: PixelSize) {
        if !size_valid(size) {
            let max = self.units.number(MAX_DOCUMENT_SIDE, 0..=0);
            Text::new(self.words.too_large.fill(&[("max", max.as_str())])).tone(Tone::Bad).show(ui);
        }
    }

    /// The width and height rows, with the chain bracketed to them when the model draws one, and the range alert of a
    /// value typed in one of them.
    pub(super) fn sides(&self, ui: &mut Ui, size: &mut LinkedSize) -> Option<SizeAlert> {
        let mut alert = None;
        let chain = size.has_chain();
        let row = if chain { Row::new().bracket(if size.is_linked() { Bracket::Linked } else { Bracket::Unlinked }) } else { Row::new() };
        row.show(ui, |ui, _| {
            Stack::new().show(ui, |ui, _| {
                let width = self.row(ui, size, Dimension::Width);
                alert = self.row(ui, size, Dimension::Height).or(width);
            });
            if chain {
                let toggle = link_toggle(self.link.clone(), Bind::given(size.is_linked())).enabled(size.is_resampling()).id_salt("link").show(ui);
                if let Some(on) = toggle.value().copied() {
                    size.lock(on);
                }
            }
        });
        alert
    }

    /// The resolution row, and the range alert of a value typed in it.
    pub(super) fn resolution(&self, ui: &mut Ui, size: &mut LinkedSize) -> Option<SizeAlert> {
        self.row(ui, size, Dimension::Resolution)
    }

    /// One number with its unit, over the model, and the range alert of a value typed in it.
    fn row(&self, ui: &mut Ui, size: &mut LinkedSize, which: Dimension) -> Option<SizeAlert> {
        let words = &self.words;
        let widths = if size.columns().is_some() { &self.widths } else { &self.lengths };
        let (name, unit_name, value, range, unit, entries, salt) = match which {
            Dimension::Width => (&words.width, &words.width_unit, size.width(), size.width_range(), size.width_unit(), widths, "width"),
            Dimension::Height => (&words.height, &words.height_unit, size.height(), size.height_range(), size.unit(), &self.lengths, "height"),
            Dimension::Resolution => (
                &words.resolution,
                &words.resolution_unit,
                size.resolution(),
                size.resolution_range(),
                size.resolution_unit(),
                &self.resolutions,
                "resolution",
            ),
        };
        let editable = match which {
            Dimension::Width => size.is_width_editable(),
            Dimension::Height => size.is_editable(),
            Dimension::Resolution => true,
        };
        let mut field = Field::new(name.clone()).id_salt(salt);
        if !editable {
            field = field.tooltip(words.resample_off.clone());
        }
        let mut alert = None;
        field.show(ui, |ui, _| {
            let shown = UnitSelect::new(name.clone(), Bind::given(Some(value)), Bind::given(unit), entries)
                .range(range.clone())
                .unit_label(unit_name.clone())
                .number_enabled(editable)
                .sizing(Sizing::Content)
                .id_salt("field")
                .show(ui);
            // Only a typed commit (Enter, Tab, focus loss) alerts; a step, the wheel and a scrub never do.
            if shown.committed {
                let typed = shown.edit.value().copied().flatten();
                let resolution = matches!(which, Dimension::Resolution);
                alert = SizeAlert::of(unit, resolution, range.clone(), shown.out_of_range.is_some(), typed);
            }
            if let Some(number) = shown.edit.value().copied().flatten() {
                match which {
                    Dimension::Width => size.set_width(number),
                    Dimension::Height => size.set_height(number),
                    Dimension::Resolution => size.set_resolution(number),
                }
            }
            if let Some(kind) = shown.unit.value().copied() {
                match which {
                    Dimension::Width => size.set_width_unit(kind),
                    Dimension::Height => size.set_unit(kind),
                    Dimension::Resolution => size.set_resolution_unit(kind),
                };
            }
        });
        alert
    }
}
