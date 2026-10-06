//! The shortcuts dialog: a searchable list of all shortcuts in the registry. Rows show
//! their first binding plus a "+N" for the remaining ones; clicking a row opens the
//! capture dialog (see [`capture`]) listing every binding of that shortcut, where a new
//! combination can be captured and the existing ones dropped individually. Display-only
//! entries render their trigger the same way - a chip, optionally with a pictogram -
//! but open nothing. The widget layout lives in `data/ui/shortcuts/`; this module
//! wires the registry-driven parts.

mod capture;

use super::bindings::{accels, is_overridden, reset, set};
use super::registry::{Gesture, Key, SHORTCUTS, Section, Shortcut};
use crate::{RnApp, RnAppWindow, config};
use adw::prelude::*;
use gettextrs::gettext;
use gtk4::{Builder, gdk, glib, glib::clone};
use std::{cell::RefCell, rc::Rc};

use capture::open_capture;

/// The dialog skeleton: the window and the capture dialog.
fn dialog_ui() -> Builder {
    Builder::from_resource(&(String::from(config::APP_IDPATH) + "ui/shortcuts/shortcuts.ui"))
}

/// The rows instantiated once per section, shortcut and listed binding.
fn rows_ui() -> Builder {
    Builder::from_resource(&(String::from(config::APP_IDPATH) + "ui/shortcuts/rows.ui"))
}

/// Opens the keyboard shortcuts dialog.
pub(crate) fn open(appwindow: &RnAppWindow) {
    let builder = dialog_ui();
    let window: adw::Window = builder.object("shortcuts_window").unwrap();
    let search: gtk4::SearchEntry = builder.object("shortcuts_search").unwrap();
    let content: gtk4::Box = builder.object("shortcuts_content").unwrap();
    let empty: adw::StatusPage = builder.object("shortcuts_empty").unwrap();
    window.set_transient_for(Some(appwindow));

    let mut groups = Vec::new();
    for section in Section::ALL {
        let group: adw::PreferencesGroup = rows_ui().object("group").unwrap();
        group.set_title(&section.title());
        let mut rows = Vec::new();
        for shortcut in SHORTCUTS
            .iter()
            .filter(|shortcut| shortcut.section() == section && shortcut.visible())
        {
            let row = match shortcut {
                Shortcut::Key(key) => key_row(key),
                Shortcut::Gesture(gesture) => gesture_row(gesture),
            };
            group.add(&row.row);
            rows.push(row);
        }
        if rows.is_empty() {
            continue;
        }
        content.append(&group);
        groups.push(GroupUi { group, rows });
    }

    let ui = Rc::new(RefCell::new(Ui {
        app: appwindow.app(),
        search: search.clone(),
        groups,
        empty,
    }));

    let search_weak = Rc::downgrade(&ui);
    search.connect_search_changed(move |_| {
        if let Some(ui) = search_weak.upgrade() {
            filter(&ui.borrow());
        }
    });

    for row in ui.borrow().groups.iter().flat_map(|group| &group.rows) {
        let Some(id) = row.id else { continue };
        let title = row.title;

        let activated_weak = Rc::downgrade(&ui);

        row.row.connect_activated(clone!(
            #[weak]
            window,
            #[upgrade_or_default]
            move |_| {
                let on_set_weak = activated_weak.clone();
                open_capture(&window, title, id, move |next| {
                    let Some(ui) = on_set_weak.upgrade() else {
                        return;
                    };
                    let mut ui = ui.borrow_mut();
                    let list: Vec<&str> = next.iter().map(String::as_str).collect();
                    set(&ui.app, id, &list);
                    refresh(&mut ui);
                });
            }
        ));

        if let Some(reset_button) = row.reset.as_ref() {
            let reset_weak = Rc::downgrade(&ui);
            reset_button.connect_clicked(move |_| {
                let Some(ui) = reset_weak.upgrade() else {
                    return;
                };
                let mut ui = ui.borrow_mut();
                reset(&ui.app, id);
                refresh(&mut ui);
            });
        }
    }

    refresh(&mut ui.borrow_mut());

    let escape = gtk4::EventControllerKey::new();
    escape.connect_key_pressed(clone!(
        #[weak]
        window,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, modifiers| {
            if key == gdk::Key::Escape
                && modifiers.is_empty()
                && ui.borrow().search.text().is_empty()
            {
                window.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    ));
    window.add_controller(escape);

    window.present();
}

struct Ui {
    app: RnApp,
    search: gtk4::SearchEntry,
    groups: Vec<GroupUi>,
    empty: adw::StatusPage,
}

struct GroupUi {
    group: adw::PreferencesGroup,
    rows: Vec<RowUi>,
}

struct RowUi {
    /// `None` for display-only entries.
    id: Option<&'static str>,
    title: &'static str,
    row: adw::ActionRow,
    /// The first binding as a keycap chip, plus a "+N" counting the ones it hides.
    bindings: Option<gtk4::Box>,
    reset: Option<gtk4::Button>,
    /// Lowercased title (+ hint/label), always searchable.
    base: String,
    /// [`Self::base`] plus the current accelerators, filtered on.
    search_text: String,
}

/// A keycap chip showing a single binding; an empty accelerator renders as "Disabled".
fn keycap(accel: &str) -> adw::ShortcutLabel {
    let chip = adw::ShortcutLabel::new(accel);
    chip.set_disabled_text(&gettext("Disabled"));
    chip.set_valign(gtk4::Align::Center);
    chip
}

/// A "+N" standing for the bindings a row does not spell out.
fn more_label(count: usize) -> gtk4::Label {
    gtk4::Label::builder()
        .label(format!("+{count}"))
        .css_classes(["dim-label"])
        .valign(gtk4::Align::Center)
        .build()
}

fn key_row(key: &'static Key) -> RowUi {
    let builder = rows_ui();
    let row: adw::ActionRow = builder.object("key_row").unwrap();
    row.set_title(&key.title);
    if !key.hint.is_empty() {
        row.set_subtitle(&key.hint);
    }
    let bindings: gtk4::Box = builder.object("key_row_bindings").unwrap();
    let reset: gtk4::Button = builder.object("key_row_reset").unwrap();

    let base = search_text(key.title.as_str(), Some(key.hint.as_str()));
    RowUi {
        id: Some(key.id),
        title: key.title.as_str(),
        row,
        bindings: Some(bindings),
        reset: Some(reset),
        search_text: base.clone(),
        base,
    }
}

/// A display-only entry: not clickable, no reset. Where key rows show binding chips,
/// it shows the gesture's modifier chip and pictogram instead.
fn gesture_row(gesture: &'static Gesture) -> RowUi {
    let builder = rows_ui();
    let row: adw::ActionRow = builder.object("gesture_row").unwrap();
    row.set_title(&gesture.title);
    if !gesture.hint.is_empty() {
        row.set_subtitle(&gesture.hint);
    }
    let trigger: gtk4::Box = builder.object("gesture_row_trigger").unwrap();

    if !gesture.modifiers.is_empty() {
        trigger.append(&keycap(gesture.modifiers));
    }
    // The pictogram stands for the gesture itself (wheel, drag, pinch).
    if !gesture.icon.is_empty() {
        let pictogram = gtk4::Image::from_icon_name(gesture.icon);
        pictogram.set_pixel_size(40);
        trigger.append(&pictogram);
    }

    let mut base = search_text(gesture.title.as_str(), Some(gesture.hint.as_str()));
    if !gesture.modifiers.is_empty() {
        base.push(' ');
        base.push_str(&human_label(gesture.modifiers).to_lowercase());
    }
    RowUi {
        id: None,
        title: gesture.title.as_str(),
        row,
        bindings: None,
        reset: None,
        search_text: base.clone(),
        base,
    }
}

fn search_text(title: &str, extra: Option<&str>) -> String {
    match extra.filter(|extra| !extra.is_empty()) {
        Some(extra) => format!("{title} {extra}"),
        None => title.to_string(),
    }
    .to_lowercase()
}

/// Syncs the displayed accelerators and the "reset" affordances with the store, then
/// re-applies the search filter.
fn refresh(ui: &mut Ui) {
    for group in &mut ui.groups {
        for row in &mut group.rows {
            let (Some(id), Some(bindings)) = (row.id, row.bindings.as_ref()) else {
                continue;
            };

            let current = accels(id);
            while let Some(child) = bindings.first_child() {
                bindings.remove(&child);
            }
            // Only the first binding is spelled out, the rest are summed up as "+N", so
            // the row stays readable at a glance. The dialog lists them all.
            match current.split_first() {
                Some((first, rest)) => {
                    bindings.append(&keycap(first));
                    if !rest.is_empty() {
                        bindings.append(&more_label(rest.len()));
                    }
                }
                None => bindings.append(&keycap("")),
            }

            let labels: Vec<String> = current.iter().map(|accel| human_label(accel)).collect();
            row.search_text = format!("{} {}", row.base, labels.join(" "))
                .trim()
                .to_lowercase();

            if let Some(reset_button) = row.reset.as_ref() {
                reset_button.set_visible(is_overridden(id));
            }
        }
    }
    filter(ui);
}

/// Shows only the rows matching the search entry.
fn filter(ui: &Ui) {
    let query = ui.search.text().to_lowercase();
    let mut total = 0;

    for group in &ui.groups {
        let mut visible = 0;
        for row in &group.rows {
            let matches = query.is_empty() || row.search_text.contains(&query);
            row.row.set_visible(matches);
            if matches {
                visible += 1;
            }
        }
        group.group.set_visible(visible > 0);
        total += visible;
    }
    ui.empty.set_visible(total == 0);
}

fn human_label(accel: &str) -> String {
    match gtk4::accelerator_parse(accel) {
        Some((key, modifiers)) => gtk4::accelerator_get_label(key, modifiers).to_string(),
        None => accel.to_string(),
    }
}
