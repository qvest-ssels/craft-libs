//! The Layer Style dialog's Name field (photocraft#14852): above the effect list on every page while the host gives the
//! dialog the layer's name, answered with the draft at OK, Enter in it is OK, Cancel and Escape keep nothing, and
//! `ui.dialog.set` drives it as `name`.

use std::cell::RefCell;
use std::rc::Rc;

use craft_imaging::layer_style::{EffectId, EffectInstance, EffectKind, LayerStyle, LayerStyleDraft, LayerStylePage, StyleParams};
use craft_imaging::ui::layer_style::{HostedLayerStyle, LayerStyleDialog};
use craft_ui::contract::Role;
use craft_ui::dialog::{DialogHost, DialogResult, StackDialog, dialog_id};
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, Vec2};

/// A Drop Shadow, so the list has an effect page besides Styles and Blending Options.
fn style() -> LayerStyle {
    let shadow = EffectInstance { id: EffectId(1), kind: EffectKind::DropShadow, on: true, params: StyleParams::default() };
    LayerStyle { effects: vec![shadow], ..LayerStyle::default() }
}

/// The dialog over `style()` on `page`, named "Top" unless `name` is `None`.
fn dialog(page: LayerStylePage, name: Option<&str>) -> HostedLayerStyle {
    let mut draft = LayerStyleDraft::new(style());
    draft.page = page;
    let dialog = HostedLayerStyle::new("Layer Style", draft, vec!["R".to_owned(), "G".to_owned(), "B".to_owned()]);
    match name {
        Some(name) => dialog.name(name),
        None => dialog,
    }
}

fn fixture(host: &Rc<RefCell<DialogHost>>) -> Fixture {
    let shown = Rc::clone(host);
    let mut fixture = Fixture::new(move |ui| shown.borrow_mut().run(ui));
    fixture.room(Vec2::new(1400.0, 1400.0));
    fixture
}

/// A host showing `dialog`, settled.
fn open(dialog: HostedLayerStyle) -> (Fixture, Rc<RefCell<DialogHost>>) {
    let host = Rc::new(RefCell::new(DialogHost::new()));
    let mut fixture = fixture(&host);
    host.borrow_mut().open(dialog);
    fixture.idle(10);
    (fixture, host)
}

/// The text field named Name, when the dialog shows one.
fn name_field(fixture: &Fixture) -> Option<Target> {
    fixture.by_role(Role::TEXT_INPUT).into_iter().find(|target| fixture.tree().name_of(target.node) == Some("Name"))
}

/// The answer the dialog left on the host, once.
fn answer(host: &Rc<RefCell<DialogHost>>) -> Option<DialogResult<LayerStyleDraft>> {
    host.borrow_mut().take::<HostedLayerStyle>()
}

/// Selects the Name field's text and types `text` over it.
fn retype(fixture: &mut Fixture, text: &str) {
    let field = name_field(fixture).unwrap_or_else(|| panic!("a Name field"));
    fixture.click(&field).settle();
    fixture.key(Key::A, Modifiers::COMMAND).type_text(text);
}

#[trial]
fn the_name_field_shows_the_layer_name_above_the_list_on_every_page() {
    for page in [LayerStylePage::Styles, LayerStylePage::Blending, LayerStylePage::Effect(EffectId(1))] {
        let (fixture, _host) = open(dialog(page, Some("Top")));
        let field = name_field(&fixture).unwrap_or_else(|| panic!("a Name field on {page:?}"));
        assert_eq!(fixture.tree().value_of(field.node), Some("Top"), "it holds the layer's name on {page:?}");
        let list = fixture.by_role(Role::LIST_ITEM).into_iter().map(|row| row.rect.top()).fold(f32::INFINITY, f32::min);
        assert!(field.rect.bottom() <= list, "it sits above the effect list on {page:?}");
    }
}

#[trial]
fn without_a_name_the_dialog_shows_no_name_field() {
    let (fixture, host) = open(dialog(LayerStylePage::Styles, None));
    assert!(name_field(&fixture).is_none(), "a host that gives no name gets no Name field");
    let fields = host.borrow().get::<HostedLayerStyle>().and_then(StackDialog::json_fields);
    assert!(fields.is_some_and(|fields| fields.get("name").is_none()), "and no name field for automation");
}

#[trial]
fn ok_answers_with_the_typed_name_and_the_style_unchanged() {
    let (mut fixture, host) = open(dialog(LayerStylePage::Styles, Some("Top")));
    retype(&mut fixture, "Top 2");
    fixture.idle(2);
    let ok = fixture.by_label("OK").unwrap_or_else(|| panic!("the OK button"));
    fixture.click(&ok).idle(4);
    let Some(DialogResult::Ok(draft)) = answer(&host) else { panic!("OK answers with the draft") };
    assert_eq!(draft.name.as_deref(), Some("Top 2"));
    assert_eq!(draft.style, style(), "only the name changed");
}

#[trial]
fn enter_in_the_name_field_is_ok() {
    let (mut fixture, host) = open(dialog(LayerStylePage::Effect(EffectId(1)), Some("Top")));
    retype(&mut fixture, "Top 2");
    fixture.key(Key::Enter, Modifiers::NONE).idle(4);
    let answered = answer(&host);
    assert!(matches!(&answered, Some(DialogResult::Ok(draft)) if draft.name.as_deref() == Some("Top 2")), "Enter answered OK: {answered:?}");
}

#[trial]
fn cancel_and_escape_keep_nothing_of_the_typed_name() {
    let (mut fixture, host) = open(dialog(LayerStylePage::Styles, Some("Top")));
    retype(&mut fixture, "Top 2");
    fixture.idle(2);
    let cancel = fixture.by_label("Cancel").unwrap_or_else(|| panic!("the Cancel button"));
    fixture.click(&cancel).idle(4);
    assert_eq!(answer(&host), Some(DialogResult::Cancel));

    host.borrow_mut().open(dialog(LayerStylePage::Styles, Some("Top")));
    fixture.idle(10);
    retype(&mut fixture, "Top 2");
    fixture.key(Key::Escape, Modifiers::NONE).idle(2).key(Key::Escape, Modifiers::NONE).idle(4);
    assert_eq!(answer(&host), Some(DialogResult::Cancel), "Escape cancels, the field's own Escape first");
}

/// The body reports typing in the Name field beside the style edit, never as one: Undo in the dialog never moves the
/// name, and the preview does not re-render for it.
#[trial]
fn typing_a_name_is_view_state_not_a_style_edit() {
    let mut draft = LayerStyleDraft::new(style());
    draft.name = Some("Top".to_owned());
    let hash = draft.render_hash();
    let draft = Rc::new(RefCell::new(draft));
    let edits = Rc::new(RefCell::new(0_usize));
    let (shown, counted) = (draft.clone(), edits.clone());
    let channels = ["R", "G", "B"].map(String::from).to_vec();
    let mut fixture = Fixture::new(move |ui| {
        let mut draft = shown.borrow_mut();
        let out = LayerStyleDialog::new("Layer Style", &draft, &channels).show(ui);
        *counted.borrow_mut() += out.edit.len();
        out.apply_view(&mut draft);
    });
    fixture.room(Vec2::new(900.0, 1400.0)).settle();
    retype(&mut fixture, "Top 2");
    fixture.key(Key::Tab, Modifiers::NONE).idle(2);
    assert_eq!(draft.borrow().name.as_deref(), Some("Top 2"), "apply_view keeps the typed name");
    assert_eq!(*edits.borrow(), 0, "no style edit");
    assert_eq!(draft.borrow().render_hash(), hash, "the name does not count for the preview");
}

#[trial]
fn automation_reads_and_sets_the_name() {
    let (mut fixture, host) = open(dialog(LayerStylePage::Styles, Some("Top")));
    let id = dialog_id("layer.style");
    let ctx = fixture.ctx().clone();
    let read = |host: &Rc<RefCell<DialogHost>>| host.borrow().get::<HostedLayerStyle>().and_then(StackDialog::json_fields).map(|f| f["name"].clone());
    assert_eq!(read(&host), Some(serde_json::json!("Top")));
    assert_eq!(host.borrow_mut().set_json_field(&ctx, id, "name", serde_json::json!("Top 2")), Ok(()));
    assert_eq!(read(&host), Some(serde_json::json!("Top 2")), "the set name reads back");
    assert!(host.borrow_mut().set_json_field(&ctx, id, "name", serde_json::json!(5)).is_err(), "a name is text");
    fixture.idle(4);
    let field = name_field(&fixture).unwrap_or_else(|| panic!("a Name field"));
    assert_eq!(fixture.tree().value_of(field.node), Some("Top 2"), "the field shows the set name");

    let (_, unnamed) = open(dialog(LayerStylePage::Styles, None));
    assert!(unnamed.borrow_mut().set_json_field(&ctx, id, "name", serde_json::json!("Top 2")).is_err(), "no Name field, no name to set");
}
