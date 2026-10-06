use super::bindings::OVERRIDES;
use super::registry::{Dispatch, Key, SHORTCUTS, Shortcut, is_devel};
use crate::RnApp;
use gtk4::{gdk, prelude::*};
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};

/// Why the character-producing accelerators are currently withheld: a text entry has
/// focus, or the current pen types text.
#[derive(Clone, Copy)]
enum WithheldFor {
    TextEntry,
    Pen,
}

/// The accelerators GTK runs for the app's actions.
#[derive(Default)]
pub(super) struct Accels {
    text_entry: AtomicBool,
    pen_types_text: AtomicBool,
}

impl Accels {
    /// Withholds the accelerators for `reason` (or restores them), reapplying them all
    /// when the set of withheld ones changed.
    fn withhold(&self, app: &RnApp, reason: WithheldFor, on: bool) {
        let withheld = match reason {
            WithheldFor::TextEntry => &self.text_entry,
            WithheldFor::Pen => &self.pen_types_text,
        };
        if withheld.swap(on, Ordering::Relaxed) != on {
            self.apply_all(app);
        }
    }

    pub(super) fn apply_all(&self, app: &RnApp) {
        for key in SHORTCUTS.iter().filter_map(Shortcut::key) {
            self.apply(app, key);
        }
    }

    pub(super) fn apply(&self, app: &RnApp, key: &Key) {
        if key.devel && !is_devel() {
            return;
        }
        if let Dispatch::Action(action) = key.dispatch {
            let current = OVERRIDES.accels(key.id);
            let accels: Vec<&str> = current
                .iter()
                .filter(|accel| self.installs(accel))
                .map(String::as_str)
                .collect();
            app.set_accels_for_action(action, &accels);
        }
    }

    /// While anything is withheld, the accelerators that would type a character - an
    /// unmodified (or shift-only) key with a Unicode value - are kept back, so that
    /// combinations like `Ctrl+C` still reach their action.
    fn installs(&self, accel: &str) -> bool {
        let withheld =
            self.text_entry.load(Ordering::Relaxed) || self.pen_types_text.load(Ordering::Relaxed);
        !withheld || !types_character(accel)
    }
}

pub(super) static ACCELS: Lazy<Accels> = Lazy::new(Accels::default);

/// Whether a text entry (e.g. the page number entry) has focus.
pub(crate) fn set_text_entry_focus(app: &RnApp, on: bool) {
    ACCELS.withhold(app, WithheldFor::TextEntry, on);
}

/// Whether the current pen types text with the keyboard.
pub(crate) fn set_pen_types_text(app: &RnApp, on: bool) {
    ACCELS.withhold(app, WithheldFor::Pen, on);
}

fn types_character(accel: &str) -> bool {
    gtk4::accelerator_parse(accel).is_some_and(|(key, modifiers)| {
        modifiers
            .difference(gdk::ModifierType::SHIFT_MASK)
            .is_empty()
            && key.to_unicode().is_some()
    })
}
