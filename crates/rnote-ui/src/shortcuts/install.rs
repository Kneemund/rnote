//! Installing the accelerators on the GTK application, and withdrawing them while text
//! input is going on.

use super::bindings::accels;
use super::registry::{Dispatch, Key, SHORTCUTS, Shortcut, is_devel};
use crate::RnApp;
use gtk4::{gdk, prelude::*};
use std::sync::atomic::{AtomicU8, Ordering};

// Reasons accelerators are currently withdrawn, as a bitmask. They are independent so
// that (for instance) leaving a text entry does not reinstall accelerators that the
// typewriter still needs withheld.
const FOCUS: u8 = 1;
const TYPEWRITER: u8 = 2;
static WITHDRAWN: AtomicU8 = AtomicU8::new(0);

/// Withdraws `reason`'s accelerators (or reinstalls them) and reapplies the rest.
fn set_withdrawn(app: &RnApp, reason: u8, on: bool) {
    let before = if on {
        WITHDRAWN.fetch_or(reason, Ordering::Relaxed)
    } else {
        WITHDRAWN.fetch_and(!reason, Ordering::Relaxed)
    };
    if ((before & reason) != 0) != on {
        apply_all(app);
    }
}

/// Whether a text entry has focus (e.g. the page number entry). Text-producing
/// accelerators are withdrawn so they don't eat the typed characters.
pub(crate) fn set_focus_text(app: &RnApp, on: bool) {
    set_withdrawn(app, FOCUS, on);
}

/// Whether the typewriter is editing a text stroke.
pub(crate) fn set_typewriter_text(app: &RnApp, on: bool) {
    set_withdrawn(app, TYPEWRITER, on);
}

/// Whether `accel` is currently installed. While text is being entered only the
/// accelerators that would type a character - an unmodified (or shift-only) key with a
/// Unicode value - are withheld, so that combinations like `Ctrl+C` keep reaching the
/// actions that handle them.
fn is_installed(accel: &str) -> bool {
    let withdrawn = WITHDRAWN.load(Ordering::Relaxed);
    if withdrawn & (FOCUS | TYPEWRITER) == 0 {
        return true;
    }
    let types_character = gtk4::accelerator_parse(accel).is_some_and(|(key, modifiers)| {
        modifiers
            .difference(gdk::ModifierType::SHIFT_MASK)
            .is_empty()
            && key.to_unicode().is_some()
    });
    !types_character
}

pub(super) fn apply_all(app: &RnApp) {
    for key in SHORTCUTS.iter().filter_map(Shortcut::key) {
        apply(app, key);
    }
}

pub(super) fn apply(app: &RnApp, key: &Key) {
    if key.devel && !is_devel() {
        return;
    }
    if let Dispatch::Action(action) = key.dispatch {
        let current = accels(key.id);
        let accels: Vec<&str> = current
            .iter()
            .filter(|accel| is_installed(accel))
            .map(String::as_str)
            .collect();
        app.set_accels_for_action(action, &accels);
    }
}
