mod capture;

use super::bindings::OVERRIDES;
use super::registry::{Gesture, Key, SHORTCUTS, Section, Shortcut};
use crate::{RnApp, RnAppWindow, config};
use adw::prelude::*;
use gettextrs::gettext;
use gtk4::{Builder, gdk, glib, glib::clone};
use std::{cell::RefCell, rc::Rc};

use capture::open_capture;

fn dialog_ui() -> Builder {
    Builder::from_resource(&(String::from(config::APP_IDPATH) + "ui/shortcuts/shortcuts.ui"))
}

fn rows_ui() -> Builder {
    Builder::from_resource(&(String::from(config::APP_IDPATH) + "ui/shortcuts/rows.ui"))
}

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
                Shortcut::Key(key) => RowUi::key(key),
                Shortcut::Gesture(gesture) => RowUi::gesture(gesture),
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
            ui.borrow().filter();
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
                    OVERRIDES.set(&ui.app, id, &list);
                    ui.refresh();
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
                OVERRIDES.reset(&ui.app, id);
                ui.refresh();
            });
        }
    }

    ui.borrow_mut().refresh();

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

impl Ui {
    /// Syncs the displayed accelerators and the "reset" affordances with the store, then
    /// re-applies the search filter.
    fn refresh(&mut self) {
        for group in &mut self.groups {
            for row in &mut group.rows {
                let (Some(id), Some(bindings)) = (row.id, row.bindings.as_ref()) else {
                    continue;
                };

                let current = OVERRIDES.accels(id);
                while let Some(child) = bindings.first_child() {
                    bindings.remove(&child);
                }
                // Only the first binding is spelled out, the rest are summed up as "+N",
                // so the row stays readable at a glance. The dialog lists them all.
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
                    reset_button.set_visible(OVERRIDES.is_overridden(id));
                }
            }
        }
        self.filter();
    }

    /// Shows only the rows matching the search entry.
    fn filter(&self) {
        let query = self.search.text().to_lowercase();
        let mut total = 0;

        for group in &self.groups {
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
        self.empty.set_visible(total == 0);
    }
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
    bindings: Option<gtk4::Box>,
    reset: Option<gtk4::Button>,
    /// Lowercased title (+ hint/label), always searchable.
    base: String,
    /// [`Self::base`] plus the current accelerators, filtered on.
    search_text: String,
}

impl RowUi {
    fn key(key: &'static Key) -> Self {
        let builder = rows_ui();
        let row: adw::ActionRow = builder.object("key_row").unwrap();
        row.set_title(&key.title);
        if !key.hint.is_empty() {
            row.set_subtitle(&key.hint);
        }
        let bindings: gtk4::Box = builder.object("key_row_bindings").unwrap();
        let reset: gtk4::Button = builder.object("key_row_reset").unwrap();
        let base = search_text(&key.title, Some(&key.hint));

        Self {
            id: Some(key.id),
            title: key.title.as_str(),
            row,
            bindings: Some(bindings),
            reset: Some(reset),
            search_text: base.clone(),
            base,
        }
    }

    /// A display-only entry: modifier chip and pictogram instead of bindings.
    fn gesture(gesture: &'static Gesture) -> Self {
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
        if !gesture.icon.is_empty() {
            let pictogram = gtk4::Image::from_icon_name(gesture.icon);
            pictogram.set_pixel_size(40);
            trigger.append(&pictogram);
        }

        let mut base = search_text(&gesture.title, Some(&gesture.hint));
        if !gesture.modifiers.is_empty() {
            base.push(' ');
            base.push_str(&human_label(gesture.modifiers).to_lowercase());
        }
        Self {
            id: None,
            title: gesture.title.as_str(),
            row,
            bindings: None,
            reset: None,
            search_text: base.clone(),
            base,
        }
    }
}

fn keycap(accel: &str) -> adw::ShortcutLabel {
    let chip = adw::ShortcutLabel::new(accel);
    chip.set_disabled_text(&gettext("Disabled"));
    chip.set_valign(gtk4::Align::Center);
    chip
}

fn more_label(count: usize) -> gtk4::Label {
    gtk4::Label::builder()
        .label(format!("+{count}"))
        .css_classes(["dim-label"])
        .valign(gtk4::Align::Center)
        .build()
}

fn search_text(title: &str, extra: Option<&str>) -> String {
    match extra.filter(|extra| !extra.is_empty()) {
        Some(extra) => format!("{title} {extra}"),
        None => title.to_string(),
    }
    .to_lowercase()
}

fn human_label(accel: &str) -> String {
    match gtk4::accelerator_parse(accel) {
        Some((key, modifiers)) => gtk4::accelerator_get_label(key, modifiers).to_string(),
        None => accel.to_string(),
    }
}
