//! The Angle field of the Layer Style pages: Drop Shadow, Inner Shadow and Gradient Overlay take a typed angle in
//! −360..=360 and pass it on as typed (photocraft#14893, #14940, #14913); the other pages keep −180..=180.

use std::cell::RefCell;
use std::rc::Rc;

use craft_imaging::layer_style::{EffectId, EffectInstance, EffectKind, EffectSpec, FieldKind, LayerStyle, LayerStyleDraft};
use craft_imaging::ui::layer_style::LayerStyleDialog;
use craft_ui::contract::Role;
use craft_ui::testing::access::name;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Vec2};

/// The room the dialog is shown in: tall enough for the longest effect page.
const ROOM: Vec2 = Vec2::new(900.0, 1400.0);

/// A dialog over one instance of `kind` with Photoshop's table and defaults, whose host applies every edit.
fn rig(kind: EffectKind) -> (Fixture, Rc<RefCell<LayerStyleDraft>>) {
    let spec = EffectSpec::photoshop(kind);
    let effects = vec![EffectInstance { id: EffectId(1), kind, on: true, params: spec.defaults.clone() }];
    let draft = Rc::new(RefCell::new(LayerStyleDraft::new(LayerStyle { effects, ..LayerStyle::default() })));
    let shown = draft.clone();
    let (specs, channels) = (vec![spec], ["R", "G", "B"].map(String::from).to_vec());
    let mut fixture = Fixture::new(move |ui| {
        let mut draft = shown.borrow_mut();
        let out = LayerStyleDialog::new("Layer Style", &draft, &channels).specs(&specs).show(ui);
        out.apply_view(&mut draft);
        for event in out.edit {
            if let Some(style) = event.value() {
                draft.style = style.clone();
            }
        }
    });
    fixture.room(ROOM).settle();
    (fixture, draft)
}

/// Types `text` over the Angle field's value and commits it with Enter; the instance's angle after.
fn type_angle(fixture: &mut Fixture, draft: &Rc<RefCell<LayerStyleDraft>>, text: &str) -> Option<f64> {
    let field = fixture
        .by_role(Role::SPIN_BUTTON)
        .into_iter()
        .find(|target| name(fixture.tree(), target.node) == Some("Angle"))
        .unwrap_or_else(|| panic!("an Angle field"));
    fixture.click(&field).settle();
    fixture.key(Key::A, Modifiers::COMMAND).type_text(text);
    fixture.key(Key::Enter, Modifiers::NONE).settle();
    draft.borrow().style.effect(EffectId(1)).and_then(|effect| effect.params.number("angle"))
}

/// The pages whose Angle field takes up to a turn either way, and those that keep the dial's one turn.
const TURN_EITHER_WAY: [EffectKind; 3] = [EffectKind::DropShadow, EffectKind::InnerShadow, EffectKind::GradientOverlay];
const ONE_TURN: [EffectKind; 3] = [EffectKind::PatternOverlay, EffectKind::BevelEmboss, EffectKind::Satin];

#[trial]
fn the_photoshop_tables_give_each_angle_field_its_typed_range() {
    let range = |kind| EffectSpec::photoshop(kind).field("angle").map(|field| field.kind.clone());
    for kind in TURN_EITHER_WAY {
        assert_eq!(range(kind), Some(FieldKind::Angle { min: -360.0, max: 360.0 }), "{kind:?}");
    }
    for kind in ONE_TURN {
        assert_eq!(range(kind), Some(FieldKind::Angle { min: -180.0, max: 180.0 }), "{kind:?}");
    }
}

/// Drop Shadow, with Use Global Light on: 200 and −200 are written as typed, to the instance and to the shared light
/// angle, for the host to wrap; 400 and −400 stop at the range's ends.
#[trial]
fn a_drop_shadow_angle_past_half_a_turn_is_written_as_typed() {
    let (mut fixture, draft) = rig(EffectKind::DropShadow);
    assert_eq!(type_angle(&mut fixture, &draft, "200"), Some(200.0));
    assert_eq!(draft.borrow().style.global_light, 200.0, "with Use Global Light on the shared angle takes it too");
    assert_eq!(type_angle(&mut fixture, &draft, "-200"), Some(-200.0));
    assert_eq!(type_angle(&mut fixture, &draft, "400"), Some(360.0));
    assert_eq!(type_angle(&mut fixture, &draft, "-400"), Some(-360.0));
}

#[trial]
fn an_inner_shadow_angle_past_half_a_turn_is_written_as_typed() {
    let (mut fixture, draft) = rig(EffectKind::InnerShadow);
    assert_eq!(type_angle(&mut fixture, &draft, "200"), Some(200.0));
    assert_eq!(type_angle(&mut fixture, &draft, "400"), Some(360.0));
}

/// Gradient Overlay keeps the typed sign: −270 and −90 stay as typed, and 360 reaches the host, which stores it as 0.
#[trial]
fn a_gradient_overlay_angle_keeps_its_typed_sign() {
    let (mut fixture, draft) = rig(EffectKind::GradientOverlay);
    for (typed, written) in [("270", 270.0), ("-270", -270.0), ("-90", -90.0), ("360", 360.0), ("400", 360.0)] {
        assert_eq!(type_angle(&mut fixture, &draft, typed), Some(written), "typed {typed}");
    }
}

/// The other pages keep −180..=180 until their own items measure a range.
#[trial]
fn the_other_angle_fields_keep_one_turn() {
    for kind in ONE_TURN {
        let (mut fixture, draft) = rig(kind);
        assert_eq!(type_angle(&mut fixture, &draft, "200"), Some(180.0), "{kind:?}");
        assert_eq!(type_angle(&mut fixture, &draft, "-200"), Some(-180.0), "{kind:?}");
    }
}
