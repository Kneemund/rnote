use super::install::ACCELS;
use super::registry::{Key, find, rebindable};
use crate::RnApp;
use gtk4::prelude::*;
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Mutex};
use tracing::warn;

/// GSettings key holding the user's overrides as `id -> "accel1 accel2"`; an empty
/// value disables the shortcut.
const OVERRIDE_KEY: &str = "shortcut-overrides";

/// The user's accelerators, overlaid on the declared defaults.
#[derive(Default)]
pub(super) struct Overrides {
    map: Mutex<HashMap<String, Vec<String>>>,
}

pub(super) static OVERRIDES: Lazy<Overrides> = Lazy::new(Overrides::default);

impl Overrides {
    pub(super) fn hydrate(&self, app: &RnApp) {
        let Some(settings) = app.app_settings() else {
            return;
        };
        let overrides = settings
            .value(OVERRIDE_KEY)
            .get::<HashMap<String, String>>()
            .unwrap_or_default();
        *self.map.lock().unwrap() = overrides
            .into_iter()
            .map(|(id, accels)| (id, split_accels(&accels)))
            .collect();
    }

    /// The user override if set, the declared defaults otherwise. An empty list means
    /// the shortcut is disabled.
    pub(super) fn accels(&self, id: &str) -> Vec<String> {
        let stored = self.map.lock().unwrap().get(id).cloned();
        let accels = stored.unwrap_or_else(|| {
            find(id)
                .map(|bound| bound.defaults.iter().map(ToString::to_string).collect())
                .unwrap_or_default()
        });
        accels.iter().map(|accel| canonical(accel)).collect()
    }

    /// Rebinds `id` (passing no accelerators disables it), only taking over the
    /// accelerators from the shortcuts that currently use them.
    pub(super) fn set(&self, app: &RnApp, id: &str, new: &[&str]) {
        self.claim(app, id, new.iter().map(|accel| canonical(accel)).collect());
    }

    /// Restores the default accelerators of `id`, claiming them back from the shortcut
    /// that was assigned them in the meantime.
    pub(super) fn reset(&self, app: &RnApp, id: &str) {
        self.claim(app, id, self.defaults_of(id));
    }

    pub(super) fn is_overridden(&self, id: &str) -> bool {
        self.map.lock().unwrap().contains_key(id)
    }

    pub(super) fn conflict_for(&self, id: &str, accel: &str) -> Option<&'static Key> {
        rebindable()
            .filter(|key| key.id != id)
            .find(|key| self.accels(key.id).iter().any(|stored| stored == accel))
    }

    fn claim(&self, app: &RnApp, id: &str, next: Vec<String>) {
        self.store(id, next.clone());

        for other in rebindable().filter(|other| other.id != id) {
            let current = self.accels(other.id);
            let remaining: Vec<String> = current
                .iter()
                .filter(|accel| !next.contains(accel))
                .cloned()
                .collect();
            if remaining.len() < current.len() {
                self.store(other.id, remaining);
                ACCELS.apply(app, other);
            }
        }

        self.persist(app);
        if let Some(bound) = find(id) {
            ACCELS.apply(app, bound);
        }
    }

    fn defaults_of(&self, id: &str) -> Vec<String> {
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

    /// Records `accels` as the override of `id`, dropping the entry when it equals the
    /// declared defaults so that [`Overrides::is_overridden`] keeps meaning "deviates
    /// from them".
    fn store(&self, id: &str, accels: Vec<String>) {
        let mut map = self.map.lock().unwrap();
        if accels == self.defaults_of(id) {
            map.remove(id);
        } else {
            map.insert(id.to_string(), accels);
        }
    }

    fn persist(&self, app: &RnApp) {
        let Some(settings) = app.app_settings() else {
            return;
        };
        let overrides: HashMap<String, String> = self
            .map
            .lock()
            .unwrap()
            .iter()
            .map(|(id, accels)| (id.clone(), accels.join(" ")))
            .collect();
        if let Err(err) = settings.set_value(OVERRIDE_KEY, &overrides.to_variant()) {
            warn!("failed persisting shortcut overrides: {err}");
        }
    }
}

/// Canonicalizes an accelerator into the spelling `gtk_accelerator_name` produces
/// (e.g. `<Control>`), so that user input, defaults and events compare equal.
fn canonical(accel: &str) -> String {
    match gtk4::accelerator_parse(accel) {
        Some((key, modifiers)) => gtk4::accelerator_name(key, modifiers).to_string(),
        None => accel.to_string(),
    }
}

fn split_accels(value: &str) -> Vec<String> {
    value.split_whitespace().map(ToString::to_string).collect()
}
