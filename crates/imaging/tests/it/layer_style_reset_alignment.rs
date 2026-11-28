//! Gradient Overlay's Reset Alignment button (photocraft#14904): right of the Angle field on its row, it puts an offset
//! dragged on the image back to the centre and leaves every other value as it was.

use std::cell::RefCell;
use std::rc::Rc;

use craft_imaging::layer_style::{EffectId, EffectInstance, EffectKind, EffectSpec, LayerStyle, LayerStyleDraft, StyleParams, StyleValue};
use craft_imaging::ui::layer_style::LayerStyleDialog;
use craft_ui::contract::Role;
use craft_ui::testing::access::is_interactive;
use craft_ui::testing::{Fixture, Target, trial};
use egui::Vec2;

/// What the host saw: the draft it keeps and how many style edits the dialog reported.
struct Host {
    draft: LayerStyleDraft,
    edits: usize,
}

/// A dialog over one instance of `kind` with Photoshop's table, `params` over its defaults; the host applies every edit.
fn rig(kind: EffectKind, params: &[(&str, StyleValue)]) -> (Fixture, Rc<RefCell<Host>>) {
    let spec = EffectSpec::photoshop(kind);
    let mut values = spec.defaults.clone();
    for (key, value) in params {
        values.set(*key, value.clone());
    }
    let effects = vec![EffectInstance { id: EffectId(1), kind, on: true, params: values }];
    let host = Rc::new(RefCell::new(Host { draft: LayerStyleDraft::new(LayerStyle { effects, ..LayerStyle::default() }), edits: 0 }));
    let shown = host.clone();
    let (specs, channels) = (vec![spec], ["R", "G", "B"].map(String::from).to_vec());
    let mut fixture = Fixture::new(move |ui| {
        let mut guard = shown.borrow_mut();
        let host = &mut *guard;
        let out = LayerStyleDialog::new("Layer Style", &host.draft, &channels).specs(&specs).show(ui);
        out.apply_view(&mut host.draft);
        for event in out.edit {
            if let Some(style) = event.value() {
                host.draft.style = style.clone();
            }
            host.edits += 1;
        }
    });
    fixture.room(Vec2::new(900.0, 1400.0)).settle();
    (fixture, host)
}

fn find(fixture: &Fixture, role: Role, label: &str) -> Option<Target> {
    fixture.by_role(role).into_iter().find(|target| fixture.tree().name_of(target.node) == Some(label))
}

fn params(host: &Rc<RefCell<Host>>) -> StyleParams {
    host.borrow().draft.style.effect(EffectId(1)).map(|effect| effect.params.clone()).unwrap_or_default()
}

/// A gradient dragged on the image (+50, +30 px on a 200 x 150 layer stores 25 %, 20 %), with Angle −90 and Scale 150.
fn dragged() -> [(&'static str, StyleValue); 3] {
    [("offset", StyleValue::Point([0.25, 0.2])), ("angle", StyleValue::Number(-90.0)), ("scale", StyleValue::Number(150.0))]
}

#[trial]
fn reset_alignment_sits_right_of_the_angle_field_on_its_row() {
    let (fixture, _host) = rig(EffectKind::GradientOverlay, &dragged());
    let button = find(&fixture, Role::BUTTON, "Reset Alignment").unwrap_or_else(|| panic!("a Reset Alignment button"));
    let angle = find(&fixture, Role::SPIN_BUTTON, "Angle").unwrap_or_else(|| panic!("the Angle field"));
    assert!(button.rect.left() >= angle.rect.right(), "right of the Angle field: {:?} {:?}", button.rect, angle.rect);
    assert!(button.rect.top() < angle.rect.bottom() && angle.rect.top() < button.rect.bottom(), "on the Angle row");
    assert!(is_interactive(fixture.tree(), button.node), "it is always enabled");
}

#[trial]
fn reset_alignment_centres_the_offset_and_keeps_the_rest() {
    let (mut fixture, host) = rig(EffectKind::GradientOverlay, &dragged());
    let before = params(&host);
    let button = find(&fixture, Role::BUTTON, "Reset Alignment").unwrap_or_else(|| panic!("a Reset Alignment button"));
    fixture.click(&button).idle(2);
    let after = params(&host);
    assert_eq!(after.get("offset"), Some(&StyleValue::Point([0.0, 0.0])), "the offset is back in the centre");
    let mut expected = before;
    expected.set("offset", StyleValue::Point([0.0, 0.0]));
    assert_eq!(after, expected, "angle, scale, style, reverse, method and colours stay as they were");
    assert_eq!(host.borrow().edits, 1, "one edit");
}

#[trial]
fn reset_alignment_with_the_gradient_centred_changes_nothing() {
    let (mut fixture, host) = rig(EffectKind::GradientOverlay, &[]);
    let before = params(&host);
    let button = find(&fixture, Role::BUTTON, "Reset Alignment").unwrap_or_else(|| panic!("a Reset Alignment button"));
    fixture.click(&button).idle(2);
    assert_eq!(params(&host), before);
    assert_eq!(host.borrow().edits, 0, "no edit");
}

#[trial]
fn the_other_angle_pages_have_no_reset_alignment() {
    for kind in [EffectKind::DropShadow, EffectKind::InnerShadow, EffectKind::Satin, EffectKind::BevelEmboss, EffectKind::PatternOverlay] {
        let (fixture, _host) = rig(kind, &[]);
        assert!(find(&fixture, Role::BUTTON, "Reset Alignment").is_none(), "{kind:?}");
    }
}
