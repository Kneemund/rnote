//! The capture dialog: capturing a new combination for a shortcut and managing the
//! bindings it currently has.

use super::super::bindings::{accels, conflict_for};
use super::{dialog_ui, human_label, rows_ui};
use adw::prelude::*;
use gettextrs::gettext;
use gtk4::{gdk, glib, glib::clone};
use std::{cell::RefCell, rc::Rc};

/// The shared key handler of an open capture dialog.
type KeyHandler =
    Rc<dyn Fn(&gtk4::EventControllerKey, gdk::Key, u32, gdk::ModifierType) -> glib::Propagation>;

/// A capture-phase key controller forwarding to the shared handler.
fn key_controller(on_key: &KeyHandler) -> gtk4::EventControllerKey {
    let controller = gtk4::EventControllerKey::builder()
        .propagation_phase(gtk4::PropagationPhase::Capture)
        .build();
    let on_key = on_key.clone();
    controller.connect_key_pressed(move |controller, key, code, state| {
        on_key(controller, key, code, state)
    });
    controller
}

/// A modal dialog editing the bindings of `id`: a combination is captured at the top and
/// the bindings in use are listed below, each with its own remove button that applies
/// immediately - dropping all of them disables the shortcut. "Add" appends the captured
/// combination to them, "Replace" makes it the only binding, and `on_set` receives the
/// complete new list either way.
pub(super) fn open_capture(
    parent: &adw::Window,
    title: &'static str,
    id: &'static str,
    on_set: impl Fn(Vec<String>) + 'static,
) {
    let builder = dialog_ui();
    let dialog: adw::Dialog = builder.object("capture_dialog").unwrap();
    let prompt: gtk4::Label = builder.object("capture_prompt").unwrap();
    let capture: gtk4::Box = builder.object("capture_line").unwrap();
    let accel_label: adw::ShortcutLabel = builder.object("capture_accel").unwrap();
    let warning: gtk4::Label = builder.object("capture_warning").unwrap();
    let group: adw::PreferencesGroup = builder.object("capture_group").unwrap();
    let add_button: gtk4::Button = builder.object("capture_add").unwrap();
    let replace_button: gtk4::Button = builder.object("capture_replace").unwrap();

    prompt.set_label(&gettext("Press the shortcut for “{}”").replace("{}", title));

    let original = accels(id);
    group.set_visible(!original.is_empty());

    // The combination captured at the top, `None` while nothing was pressed.
    let captured: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let on_set: Rc<dyn Fn(Vec<String>)> = Rc::new(on_set);

    for accel in original {
        bind_row(&accel, id, &group, &captured, &add_button, &on_set);
    }

    // "Replace" makes the captured combination the only binding - the way to replace them
    // all at once - while "Add" appends it to the ones in use, so combinations like the
    // number row and the keypad can coexist.
    let captured_for_replace = captured.clone();
    let on_set_for_replace = on_set.clone();
    replace_button.connect_clicked(clone!(
        #[weak]
        dialog,
        #[upgrade_or_default]
        move |_| {
            let Some(accel) = captured_for_replace.borrow().clone() else {
                return;
            };
            on_set_for_replace(vec![accel]);
            dialog.close();
        }
    ));

    // "Add" keeps the dialog open so several combinations can be attached one after
    // another: it lists the binding right away and empties the capture for the next one.
    let captured_for_add = captured.clone();
    let group_for_add = group.downgrade();
    add_button.connect_clicked(clone!(
        #[weak]
        add_button,
        #[weak]
        warning,
        #[weak]
        capture,
        #[upgrade_or_default]
        move |_| {
            let (Some(group), Some(accel)) =
                (group_for_add.upgrade(), captured_for_add.borrow().clone())
            else {
                return;
            };
            let mut next = accels(id);
            if !next.contains(&accel) {
                next.push(accel.clone());
            }
            on_set(next);
            bind_row(&accel, id, &group, &captured_for_add, &add_button, &on_set);

            *captured_for_add.borrow_mut() = None;
            warning.set_visible(false);
            capture.set_visible(false);
        }
    ));

    let on_key: KeyHandler = Rc::new(clone!(
        #[weak]
        dialog,
        #[weak]
        accel_label,
        #[weak]
        warning,
        #[weak]
        add_button,
        #[weak]
        replace_button,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, state| {
            let modifiers = state & gtk4::accelerator_get_default_mod_mask();

            if is_modifier_key(key) {
                return glib::Propagation::Stop;
            }
            if modifiers.is_empty() && key == gdk::Key::Escape {
                dialog.close();
                return glib::Propagation::Stop;
            }
            if gtk4::accelerator_valid(key, modifiers) {
                let accel = gtk4::accelerator_name(key, modifiers);
                *captured.borrow_mut() = Some(accel.to_string());
                accel_label.set_accelerator(&accel);
                capture.set_visible(true);
                replace_button.set_sensitive(true);
                sync_add(id, &add_button, &captured.borrow());

                match conflict_for(id, &accel) {
                    Some(other) => {
                        warning
                            .set_label(&gettext("Will replace “{}”").replace("{}", &other.title));
                        warning.set_visible(true);
                    }
                    None => warning.set_visible(false),
                }
            }
            glib::Propagation::Stop
        }
    ));

    // A press is dispatched along the ancestors of the focus: while the focus sits
    // inside the dialog that walk stops there, once it escaped it reaches the window
    // instead. Both therefore carry the handler - the first to see a press also ends
    // it.
    let window = parent.clone();
    let window_controller = key_controller(&on_key);
    window.add_controller(window_controller.clone());
    dialog.add_controller(key_controller(&on_key));
    dialog.connect_closed(clone!(
        #[weak]
        window,
        #[upgrade_or_default]
        move |_| window.remove_controller(&window_controller)
    ));

    dialog.present(Some(parent));
}

/// Appends the row for `accel`. The row's trash button drops that binding through
/// `on_set` and hides the group once the last one is gone.
fn bind_row(
    accel: &str,
    id: &'static str,
    group: &adw::PreferencesGroup,
    captured: &Rc<RefCell<Option<String>>>,
    add_button: &gtk4::Button,
    on_set: &Rc<dyn Fn(Vec<String>)>,
) {
    let builder = rows_ui();
    let row: adw::ActionRow = builder.object("binding_row").unwrap();
    let chip: adw::ShortcutLabel = builder.object("binding_row_chip").unwrap();
    let remove_button: gtk4::Button = builder.object("binding_row_remove").unwrap();

    chip.set_accelerator(accel);
    let label = human_label(accel);
    row.update_property(&[gtk4::accessible::Property::Label(&label)]);

    let accel = accel.to_string();
    let listed = row.downgrade();
    let group_weak = group.downgrade();
    let captured = captured.clone();
    let add_button = add_button.clone();
    let on_set = on_set.clone();
    remove_button.connect_clicked(move |_| {
        let (Some(group), Some(listed)) = (group_weak.upgrade(), listed.upgrade()) else {
            return;
        };
        let mut next = accels(id);
        next.retain(|other| other != &accel);
        group.remove(&listed);
        group.set_visible(!next.is_empty());
        on_set(next);
        sync_add(id, &add_button, &captured.borrow());
    });

    group.add(&row);
    group.set_visible(true);
}

/// Keeps "Add" usable only for a captured combination the shortcut doesn't use yet.
fn sync_add(id: &str, add_button: &gtk4::Button, captured: &Option<String>) {
    let in_use = accels(id);
    add_button.set_sensitive(
        captured
            .as_ref()
            .is_some_and(|accel| !in_use.iter().any(|other| other == accel)),
    );
}

/// Modifier-only presses, which never become a binding (they would render as a bare
/// modifier in the UI).
fn is_modifier_key(key: gdk::Key) -> bool {
    matches!(
        key,
        gdk::Key::Shift_L
            | gdk::Key::Shift_R
            | gdk::Key::Control_L
            | gdk::Key::Control_R
            | gdk::Key::Alt_L
            | gdk::Key::Alt_R
            | gdk::Key::Super_L
            | gdk::Key::Super_R
            | gdk::Key::Meta_L
            | gdk::Key::Meta_R
            | gdk::Key::Hyper_L
            | gdk::Key::Hyper_R
            | gdk::Key::Caps_Lock
            | gdk::Key::Num_Lock
            | gdk::Key::Scroll_Lock
            | gdk::Key::ISO_Level3_Shift
            | gdk::Key::ISO_Level5_Shift
            | gdk::Key::Mode_switch
    )
}
