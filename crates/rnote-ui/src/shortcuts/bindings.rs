//! The binding layer: effective accelerators - the declared defaults overlaid with the
//! user's GSettings overrides - how they are changed, and how they are persisted.

use super::install;
use super::registry::{Key, find, rebindable};
use crate::RnApp;
use gtk4::prelude::*;
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Mutex};
use tracing::warn;

/// GSettings key holding the user's overrides as `id -> "accel1 accel2"`.
/// An empty value disables the shortcut. Absent ids use their declared defaults.
const OVERRIDE_KEY: &str = "shortcut-overrides";

// The override store. Single-process, hydrated from GSettings in [`hydrate`] and
// written back on every change.
static OVERRIDES: Lazy<Mutex<HashMap<String, Vec<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Hydrates the user overrides from GSettings. Call once at application startup.
pub(super) fn hydrate(app: &RnApp) {
    let Some(settings) = app.app_settings() else {
        return;
    };
    let overrides = settings
        .value(OVERRIDE_KEY)
        .get::<HashMap<String, String>>()
        .unwrap_or_default();
    *OVERRIDES.lock().unwrap() = overrides
        .into_iter()
        .map(|(id, accels)| (id, split_accels(&accels)))
        .collect();
}

/// Effective accelerators of a bound shortcut: the user override if set, the declared
/// defaults otherwise. An empty list means the shortcut is disabled.
pub(crate) fn accels(id: &str) -> Vec<String> {
    let stored = OVERRIDES.lock().unwrap().get(id).cloned();
    let accels = stored.unwrap_or_else(|| {
        find(id)
            .map(|bound| bound.defaults.iter().map(ToString::to_string).collect())
            .unwrap_or_default()
    });
    accels.iter().map(|accel| canonical(accel)).collect()
}

/// Rebinds `id` (passing no accelerators disables it). Other shortcuts only lose the
/// accelerators taken over from them, keeping the rest of their bindings.
pub(crate) fn set(app: &RnApp, id: &str, new: &[&str]) {
    claim(app, id, new.iter().map(|accel| canonical(accel)).collect());
}

/// Restores the default accelerators of `id`, claiming them back from the shortcut that
/// was assigned them in the meantime.
pub(crate) fn reset(app: &RnApp, id: &str) {
    claim(app, id, defaults_of(id));
}

/// Makes `id` the owner of `next`: every other visible shortcut loses the accelerators
/// taken over and is disabled only when it has none of its own left. Applying it to a
/// reset restores the invariant that no accelerator is bound twice.
fn claim(app: &RnApp, id: &str, next: Vec<String>) {
    store(id, next.clone());

    for other in rebindable().filter(|other| other.id != id) {
        let current = accels(other.id);
        let remaining: Vec<String> = current
            .iter()
            .filter(|accel| !next.contains(accel))
            .cloned()
            .collect();
        if remaining.len() < current.len() {
            store(other.id, remaining);
            install::apply(app, other);
        }
    }

    persist(app);
    if let Some(bound) = find(id) {
        install::apply(app, bound);
    }
}

/// Whether the shortcut deviates from its defaults.
pub(crate) fn is_overridden(id: &str) -> bool {
    OVERRIDES.lock().unwrap().contains_key(id)
}

/// Returns the visible shortcut (other than `id`) that already uses `accel`.
pub(crate) fn conflict_for(id: &str, accel: &str) -> Option<&'static Key> {
    rebindable()
        .filter(|key| key.id != id)
        .find(|key| accels(key.id).iter().any(|stored| stored == accel))
}

/// Declared default accelerators of `id`, canonicalized like the stored ones.
fn defaults_of(id: &str) -> Vec<String> {
    find(id)
        .map(|bound| {
            bound
                .defaults
                .iter()
                .map(|accel| canonical(accel))
                .collect()
        })
        .unwrap_or_default()
}

/// Records `accels` as the override of `id`. The entry is dropped when it equals the
/// declared defaults, so [`is_overridden`] keeps meaning "deviates from its defaults".
fn store(id: &str, accels: Vec<String>) {
    let mut overrides = OVERRIDES.lock().unwrap();
    if accels == defaults_of(id) {
        overrides.remove(id);
    } else {
        overrides.insert(id.to_string(), accels);
    }
}

fn persist(app: &RnApp) {
    let Some(settings) = app.app_settings() else {
        return;
    };
    let overrides: HashMap<String, String> = OVERRIDES
        .lock()
        .unwrap()
        .iter()
        .map(|(id, accels)| (id.clone(), accels.join(" ")))
        .collect();
    if let Err(err) = settings.set_value(OVERRIDE_KEY, &overrides.to_variant()) {
        warn!("failed persisting shortcut overrides: {err}");
    }
}

/// Canonicalizes an accelerator, so that user input, defaults and events written by
/// `gtk_accelerator_name` compare equal (e.g. `<Ctrl>` becomes `<Control>`).
fn canonical(accel: &str) -> String {
    match gtk4::accelerator_parse(accel) {
        Some((key, modifiers)) => gtk4::accelerator_name(key, modifiers).to_string(),
        None => accel.to_string(),
    }
}

fn split_accels(value: &str) -> Vec<String> {
    value.split_whitespace().map(ToString::to_string).collect()
}
