//! Unified, editable keyboard shortcuts.
//!
//! Every user-facing shortcut is declared exactly once in [`registry::SHORTCUTS`], the
//! single source of truth for
//!
//! * installing GTK action accelerators ([`init`], [`bindings::set`], [`bindings::reset`]),
//! * matching key combos that are checked in code ([`matches`]),
//! * generating the shortcuts dialog (see [`open_dialog`]).
//!
//! User re-bindings are persisted in GSettings (`shortcut-overrides`) and overlaid on
//! the declared defaults ([`bindings`]). Assigning a combination that is already in use
//! disables the other binding ([`bindings::set`]).
//!
//! Accelerators are additionally withdrawn while text input is going on
//! ([`set_focus_text`], [`set_typewriter_text`]): GTK runs them in the capture phase,
//! ahead of the focused widget, so a bound key would otherwise swallow the character
//! instead of letting it be typed ([`install`]).
//!
//! Entries with [`registry::Dispatch::Code`] are matched in code at the input boundary
//! (currently the canvas key controller). Text-editing combinations inside the
//! typewriter tool are engine behavior and are intentionally not part of the registry
//! yet; they can be added later as `Dispatch::Code` entries once the engine input path
//! routes through [`matches`].

mod bindings;
mod dialog;
mod install;
mod registry;

use crate::RnApp;
use gtk4::gdk;

pub(crate) use dialog::open as open_dialog;
pub(crate) use install::{set_focus_text, set_typewriter_text};

/// Hydrates user overrides and installs all accelerators. Call once at application startup.
pub(crate) fn init(app: &RnApp) {
    registry::validate_defaults();
    bindings::hydrate(app);
    install::apply_all(app);
}

/// Matches a key press from the UI event stream against a shortcut, no matter whether it
/// dispatches an action or is handled in code.
pub(crate) fn matches(id: &str, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
    debug_assert!(registry::find(id).is_some(), "unknown shortcut id `{id}`");
    let accel = gtk4::accelerator_name(key, modifiers & gtk4::accelerator_get_default_mod_mask());
    bindings::accels(id).iter().any(|stored| stored == &accel)
}
