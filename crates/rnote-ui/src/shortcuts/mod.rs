mod bindings;
mod dialog;
mod install;
mod registry;

use crate::RnApp;
use gtk4::gdk;

pub(crate) use dialog::open as open_dialog;
pub(crate) use install::{set_pen_types_text, set_text_entry_focus};

/// Hydrates the user overrides and installs all accelerators; call once at startup.
pub(crate) fn init(app: &RnApp) {
    registry::validate_defaults();
    bindings::OVERRIDES.hydrate(app);
    install::ACCELS.apply_all(app);
}

/// Whether `key` + `modifiers` is bound to `id`.
pub(crate) fn matches(id: &str, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
    debug_assert!(registry::find(id).is_some(), "unknown shortcut id `{id}`");
    let accel = gtk4::accelerator_name(key, modifiers & gtk4::accelerator_get_default_mod_mask());
    bindings::OVERRIDES
        .accels(id)
        .iter()
        .any(|stored| stored == &accel)
}
