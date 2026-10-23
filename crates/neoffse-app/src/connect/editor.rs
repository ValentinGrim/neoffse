//! Wires the editor page to its view model.
//!
//! Every callback holds a `slint::Weak<MainWindow>` and a clone of [`App`],
//! never a strong window handle: the window owns the callbacks, so a strong
//! handle would form a cycle. Edits run on the UI thread, under the session
//! lock: they patch an in-memory document and the rebuild reads a few hundred
//! rows.
//!
//! After an action the page refreshes from the session. When the shape of the
//! data is unchanged, the refresh sets the changed rows one by one on the
//! models the screen already shows, so a widget keeps its keyboard focus.

use slint::{ComponentHandle, Model, ModelRc, VecModel, Weak};

use super::history::sync_history;
use crate::editor::{EditorViewModel, History, Input, growth_notice, levels_gained, problem_notice};
use crate::theme::sync_theme;
use crate::{App, EditNotice, EditProblem, EditorBridge, EditorData, EntityPage, FieldRow, ItemList, ItemRow, MainWindow, Overview, PickerEntry, SectionData};

/// Wires the editor callbacks of the [`EditorBridge`] global to `app.editor`.
///
/// Call it on the UI thread, before the event loop starts.
pub fn connect_editor(window: &MainWindow, app: &App) {
    let bridge = window.global::<EditorBridge>();
    let weak = window.as_weak();

    bridge.on_field_text_committed({
        let (weak, app) = (weak.clone(), app.clone());
        move |key, text| act(&weak, &app, |editor| editor.commit(key, Input::Text(text.to_string())))
    });
    bridge.on_field_toggled({
        let (weak, app) = (weak.clone(), app.clone());
        move |key, checked| act(&weak, &app, |editor| editor.commit(key, Input::Toggle(checked)))
    });
    bridge.on_field_picked({
        let (weak, app) = (weak.clone(), app.clone());
        move |key, id| act(&weak, &app, |editor| editor.commit(key, Input::Pick(i64::from(id))))
    });
    bridge.on_field_name_picked({
        let (weak, app) = (weak.clone(), app.clone());
        move |key, index| {
            act(&weak, &app, |editor| match usize::try_from(index) {
                Ok(index) => editor.commit(key, Input::Name(index)),
                Err(_) => Some(problem_notice(EditProblem::NotEditable, "")),
            });
        }
    });
    bridge.on_item_add_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move |list, id| act(&weak, &app, |editor| editor.add_row(list, id))
    });
    bridge.on_item_remove_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move |row| act(&weak, &app, |editor| editor.remove_row(row))
    });
    bridge.on_field_picker_query({
        let (weak, app) = (weak.clone(), app.clone());
        move |key, text| show_entries(&weak, app.editor.field_entries(key, &text))
    });
    bridge.on_list_picker_query({
        let (weak, app) = (weak.clone(), app.clone());
        move |list, text| show_entries(&weak, app.editor.list_entries(list, &text))
    });
    bridge.on_notice_dismissed({
        let weak = weak.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                set_notice(&window.global::<EditorBridge>(), None, false);
            }
        }
    });
}

/// Pushes `app.editor.data()` into the bridge and clears the notice and the
/// expert offer.
///
/// The rows of a shape that did not change update in place; any other change
/// replaces the data whole. A failed `data()` sets the notice instead.
pub fn refresh_editor(window: &MainWindow, app: &App) {
    sync_theme(window, app.state.game());
    refresh_with(window, app, None);
}

/// Runs `action`, then refreshes with the notice it returned.
///
/// An action that went through in default mode may have raised levels, and
/// the game interface then grew the stats with random draws
/// (`docs/formats/pixel-remaster.md#level-up-in-ff-i`). The overview before
/// and after tells how many levels grew, and the page says so. An error
/// notice wins over that info notice.
fn act(weak: &Weak<MainWindow>, app: &App, action: impl FnOnce(&EditorViewModel) -> Option<EditNotice>) {
    if let Some(window) = weak.upgrade() {
        let before = window.global::<EditorBridge>().get_overview();
        let notice = action(&app.editor);
        let notice = notice.or_else(|| {
            let gained = if app.editor.expert() {
                0
            } else {
                levels_gained(&before, &app.editor.overview())
            };
            (gained > 0).then(|| growth_notice(gained))
        });
        refresh_with(&window, app, notice);
    }
}

/// The notice of the action wins over a failure of the rebuild: a stale edit
/// after Back says "not editable", not "internal".
pub(super) fn refresh_with(window: &MainWindow, app: &App, notice: Option<EditNotice>) {
    let bridge = window.global::<EditorBridge>();
    let failure = match app.editor.data() {
        Ok(data) => {
            apply(&bridge, data);
            bridge.set_overview(app.editor.overview());
            // `history` locks the session, which a running save holds.
            if !app.editor.is_saving() {
                sync_history(&bridge, app.editor.history());
            }
            None
        }
        Err(notice) => {
            bridge.set_overview(Overview::default());
            sync_history(&bridge, History::default());
            Some(notice)
        }
    };
    let notice = notice.or(failure);
    let offer = notice.as_ref().is_some_and(|notice| app.editor.offers_expert(notice));
    set_notice(&bridge, notice, offer);
}

/// Shows or clears the notice. `offer` adds the "Switch to expert mode"
/// button to it.
fn set_notice(bridge: &EditorBridge<'_>, notice: Option<EditNotice>, offer: bool) {
    bridge.set_offer_expert(offer);
    bridge.set_has_notice(notice.is_some());
    bridge.set_notice(notice.unwrap_or_default());
}

fn show_entries(weak: &Weak<MainWindow>, entries: Vec<PickerEntry>) {
    if let Some(window) = weak.upgrade() {
        window.global::<EditorBridge>().set_picker_entries(ModelRc::new(VecModel::from(entries)));
    }
}

fn apply(bridge: &EditorBridge<'_>, new: EditorData) {
    let old = bridge.get_data();
    if same_shape(&old, &new) {
        update_section(&old.party, &new.party);
        update_section(&old.inventory, &new.inventory);
        update_section(&old.magic, &new.magic);
    } else {
        bridge.set_data(new);
    }
}

/// The `VecModel` behind `model`, `None` for a model of another type.
fn vec_of<T: Clone + 'static>(model: &ModelRc<T>) -> Option<&VecModel<T>> {
    model.as_any().downcast_ref::<VecModel<T>>()
}

/// Whether `old` and `new` hold the same number of rows and `same` accepts
/// each pair. A model that is not a `VecModel` never matches.
fn aligned<T: Clone + 'static>(old: &ModelRc<T>, new: &ModelRc<T>, same: impl Fn(&T, &T) -> bool) -> bool {
    vec_of(old).is_some() && old.row_count() == new.row_count() && old.iter().zip(new.iter()).all(|(old, new)| same(&old, &new))
}

fn same_fields(old: &ModelRc<FieldRow>, new: &ModelRc<FieldRow>) -> bool {
    aligned(old, new, |old, new| old.kind == new.kind)
}

fn same_shape(old: &EditorData, new: &EditorData) -> bool {
    [(&old.party, &new.party), (&old.inventory, &new.inventory), (&old.magic, &new.magic)]
        .into_iter()
        .all(|(old, new)| same_section(old, new))
}

fn same_section(old: &SectionData, new: &SectionData) -> bool {
    old.present == new.present
        && old.read_only == new.read_only
        && aligned(&old.pages, &new.pages, |old, new| same_fields(&old.fields, &new.fields))
        && aligned(&old.lists, &new.lists, |old, new| {
            old.key == new.key
                && old.title == new.title
                && old.insertable == new.insertable
                && aligned(&old.rows, &new.rows, |old, new| same_fields(&old.fields, &new.fields))
        })
}

/// Sets the rows of `new` that differ from `old`, on the models `old` holds.
/// Only called when [`same_shape`] holds.
fn update_fields(old: &ModelRc<FieldRow>, new: &ModelRc<FieldRow>) {
    let Some(model) = vec_of(old) else { return };
    let pairs: Vec<_> = old.iter().zip(new.iter()).collect();
    for (index, (old, new)) in pairs.into_iter().enumerate() {
        if old != new {
            model.set_row_data(index, new);
        }
    }
}

fn update_section(old: &SectionData, new: &SectionData) {
    if let Some(model) = vec_of(&old.pages) {
        let pairs: Vec<(EntityPage, EntityPage)> = old.pages.iter().zip(new.pages.iter()).collect();
        for (index, (old, new)) in pairs.into_iter().enumerate() {
            update_fields(&old.fields, &new.fields);
            if old.label != new.label {
                model.set_row_data(
                    index,
                    EntityPage {
                        label: new.label,
                        fields: old.fields,
                    },
                );
            }
        }
    }
    let lists: Vec<(ItemList, ItemList)> = old.lists.iter().zip(new.lists.iter()).collect();
    for (old, new) in lists {
        update_rows(&old.rows, &new.rows);
    }
}

fn update_rows(old: &ModelRc<ItemRow>, new: &ModelRc<ItemRow>) {
    let Some(model) = vec_of(old) else { return };
    let pairs: Vec<(ItemRow, ItemRow)> = old.iter().zip(new.iter()).collect();
    for (index, (old, new)) in pairs.into_iter().enumerate() {
        update_fields(&old.fields, &new.fields);
        let changed =
            old.key != new.key || old.label != new.label || old.locked != new.locked || old.removable != new.removable || old.can_remove != new.can_remove;
        if changed {
            model.set_row_data(index, ItemRow { fields: old.fields, ..new });
        }
    }
}
