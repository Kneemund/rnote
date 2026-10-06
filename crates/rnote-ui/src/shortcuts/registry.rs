//! The registry: every shortcut that exists, in display order. [`SHORTCUTS`] is built
//! - and translated - once, on first access.

use crate::config;
use gettextrs::gettext;
use once_cell::sync::Lazy;
use tracing::warn;

/// Display grouping of shortcuts, in the order they are listed.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    General,
    Navigation,
    View,
    Document,
    Drawing,
    MouseGestures,
    TouchGestures,
}

impl Section {
    pub(crate) const ALL: [Section; 7] = [
        Self::General,
        Self::Navigation,
        Self::View,
        Self::Document,
        Self::Drawing,
        Self::MouseGestures,
        Self::TouchGestures,
    ];

    pub(crate) fn title(self) -> String {
        match self {
            Self::General => gettext("General"),
            Self::Navigation => gettext("Navigation"),
            Self::View => gettext("View"),
            Self::Document => gettext("Document"),
            Self::Drawing => gettext("Drawing"),
            Self::MouseGestures => gettext("Mouse Gestures"),
            Self::TouchGestures => gettext("Touch Gestures"),
        }
    }
}

/// Where a shortcut's keystroke is dispatched to.
#[derive(Copy, Clone, Debug)]
pub(crate) enum Dispatch {
    /// A GTK detailed-action name (e.g. `"win.save-doc"`), dispatched by GTK's
    /// accelerator machinery.
    Action(&'static str),
    /// A key combo checked in code; test it with [`super::matches`].
    Code,
}

/// A rebindable keyboard shortcut.
#[derive(Clone, Debug)]
pub(crate) struct Key {
    /// Stable identifier. Also the key of the user override in settings.
    pub id: &'static str,
    pub title: String,
    pub section: Section,
    pub dispatch: Dispatch,
    pub defaults: &'static [&'static str],
    /// Additional display-only hint (e.g. `"When in Fixed-Size Layout"`), translated
    /// once when the registry is built.
    pub hint: String,
    /// Only registered and listed in devel builds.
    pub devel: bool,
}

/// A display-only gesture (pointer or touch) that cannot be rebound.
#[derive(Clone, Debug)]
pub(crate) struct Gesture {
    pub title: String,
    pub section: Section,
    /// Modifiers held for the gesture, spelled as an accelerator and rendered as
    /// a keycap chip (e.g. `"<Alt>"`, `"<Ctrl>"`); empty for none.
    pub modifiers: &'static str,
    /// Display-only subtitle (e.g. the name of a gesture), translated once when the
    /// registry is built.
    pub hint: String,
    /// Pictogram of the gesture itself (wheel, drag, pinch), shown next to the
    /// modifier chip. Empty for none.
    pub icon: &'static str,
}

#[derive(Clone, Debug)]
pub(crate) enum Shortcut {
    Key(Key),
    Gesture(Gesture),
}

impl Shortcut {
    pub(crate) fn key(&self) -> Option<&Key> {
        match self {
            Self::Key(key) => Some(key),
            Self::Gesture(_) => None,
        }
    }

    pub(crate) fn section(&self) -> Section {
        match self {
            Self::Key(key) => key.section,
            Self::Gesture(gesture) => gesture.section,
        }
    }

    pub(crate) fn visible(&self) -> bool {
        match self {
            Self::Key(key) => !key.devel || is_devel(),
            Self::Gesture(_) => true,
        }
    }
}

// Registry entry constructors, so the table below stays terse.

fn act(
    id: &'static str,
    title: String,
    section: Section,
    action: &'static str,
    defaults: &'static [&'static str],
) -> Shortcut {
    Shortcut::Key(Key {
        id,
        title,
        section,
        dispatch: Dispatch::Action(action),
        defaults,
        hint: String::new(),
        devel: false,
    })
}

fn code(
    id: &'static str,
    title: String,
    section: Section,
    defaults: &'static [&'static str],
) -> Shortcut {
    Shortcut::Key(Key {
        id,
        title,
        section,
        dispatch: Dispatch::Code,
        defaults,
        hint: String::new(),
        devel: false,
    })
}

/// Attaches a display-only hint (the row subtitle).
fn hint(shortcut: Shortcut, hint: String) -> Shortcut {
    match shortcut {
        Shortcut::Key(key) => Shortcut::Key(Key { hint, ..key }),
        Shortcut::Gesture(gesture) => Shortcut::Gesture(Gesture { hint, ..gesture }),
    }
}

fn devel_only(shortcut: Shortcut) -> Shortcut {
    match shortcut {
        Shortcut::Key(key) => Shortcut::Key(Key { devel: true, ..key }),
        gesture => gesture,
    }
}

fn gesture(
    title: String,
    section: Section,
    modifiers: &'static str,
    icon: &'static str,
) -> Shortcut {
    Shortcut::Gesture(Gesture {
        title,
        section,
        modifiers,
        hint: String::new(),
        icon,
    })
}

/// The registry: every shortcut, in display order. Add new shortcuts here only. Built
/// (and translated) once, on first access.
pub(crate) static SHORTCUTS: Lazy<Vec<Shortcut>> = Lazy::new(|| {
    vec![
        // General
        act(
            "show-shortcuts",
            gettext("Show Keyboard Shortcuts"),
            Section::General,
            "win.keyboard-shortcuts",
            &["<Ctrl>question"],
        ),
        act(
            "new-window",
            gettext("New Window"),
            Section::General,
            "app.new-window",
            &["<Ctrl>n"],
        ),
        act(
            "new-tab",
            gettext("New Tab"),
            Section::General,
            "win.new-tab",
            &["<Ctrl>t"],
        ),
        act(
            "close-tab",
            gettext("Close the Active Tab"),
            Section::General,
            "win.active-tab-close",
            &["<Ctrl>w"],
        ),
        act(
            "quit",
            gettext("Quit the Application"),
            Section::General,
            "app.quit",
            &["<Ctrl>q"],
        ),
        act(
            "toggle-overview",
            gettext("Toggle Tabs Overview"),
            Section::General,
            "win.toggle-overview",
            &["<Ctrl><Shift>o"],
        ),
        act(
            "open-canvas-menu",
            gettext("Open the Canvas-Menu"),
            Section::General,
            "win.open-canvasmenu",
            &["F9"],
        ),
        act(
            "open-app-menu",
            gettext("Open the App-Menu"),
            Section::General,
            "win.open-appmenu",
            &["F10"],
        ),
        act(
            "fullscreen",
            gettext("Toggle Fullscreen"),
            Section::General,
            "win.fullscreen",
            &["F11"],
        ),
        devel_only(act(
            "visual-debug",
            gettext("Visual Debug"),
            Section::General,
            "win.visual-debug",
            &["<Ctrl><Shift>v"],
        )),
        // Navigation
        act(
            "pen-style-brush",
            gettext("Switch to the 'Brush'"),
            Section::Navigation,
            "win.pen-style::brush",
            &["<Ctrl>1", "<Ctrl>KP_1"],
        ),
        act(
            "pen-style-shaper",
            gettext("Switch to the 'Shaper'"),
            Section::Navigation,
            "win.pen-style::shaper",
            &["<Ctrl>2", "<Ctrl>KP_2"],
        ),
        act(
            "pen-style-typewriter",
            gettext("Switch to the 'Typewriter'"),
            Section::Navigation,
            "win.pen-style::typewriter",
            &["<Ctrl>3", "<Ctrl>KP_3"],
        ),
        act(
            "pen-style-eraser",
            gettext("Switch to the 'Eraser'"),
            Section::Navigation,
            "win.pen-style::eraser",
            &["<Ctrl>4", "<Ctrl>KP_4"],
        ),
        act(
            "pen-style-selector",
            gettext("Switch to the 'Selector'"),
            Section::Navigation,
            "win.pen-style::selector",
            &["<Ctrl>5", "<Ctrl>KP_5"],
        ),
        act(
            "pen-style-tools",
            gettext("Switch to the 'Tools'"),
            Section::Navigation,
            "win.pen-style::tools",
            &["<Ctrl>6", "<Ctrl>KP_6"],
        ),
        code(
            "pen-shortcut-keyboard",
            gettext("Keyboard Pen Shortcut"),
            Section::Navigation,
            &["<Ctrl>space"],
        ),
        act(
            "select-color-1",
            gettext("Select Color 1"),
            Section::Navigation,
            "win.set-color-1",
            &["<Ctrl><Alt>1", "<Ctrl><Alt>KP_1"],
        ),
        act(
            "select-color-2",
            gettext("Select Color 2"),
            Section::Navigation,
            "win.set-color-2",
            &["<Ctrl><Alt>2", "<Ctrl><Alt>KP_2"],
        ),
        act(
            "select-color-3",
            gettext("Select Color 3"),
            Section::Navigation,
            "win.set-color-3",
            &["<Ctrl><Alt>3", "<Ctrl><Alt>KP_3"],
        ),
        act(
            "select-color-4",
            gettext("Select Color 4"),
            Section::Navigation,
            "win.set-color-4",
            &["<Ctrl><Alt>4", "<Ctrl><Alt>KP_4"],
        ),
        act(
            "select-color-5",
            gettext("Select Color 5"),
            Section::Navigation,
            "win.set-color-5",
            &["<Ctrl><Alt>5", "<Ctrl><Alt>KP_5"],
        ),
        act(
            "select-color-6",
            gettext("Select Color 6"),
            Section::Navigation,
            "win.set-color-6",
            &["<Ctrl><Alt>6", "<Ctrl><Alt>KP_6"],
        ),
        act(
            "select-color-7",
            gettext("Select Color 7"),
            Section::Navigation,
            "win.set-color-7",
            &["<Ctrl><Alt>7", "<Ctrl><Alt>KP_7"],
        ),
        act(
            "select-color-8",
            gettext("Select Color 8"),
            Section::Navigation,
            "win.set-color-8",
            &["<Ctrl><Alt>8", "<Ctrl><Alt>KP_8"],
        ),
        act(
            "select-color-9",
            gettext("Select Color 9"),
            Section::Navigation,
            "win.set-color-9",
            &["<Ctrl><Alt>9", "<Ctrl><Alt>KP_9"],
        ),
        // View
        act(
            "zoom-in",
            gettext("Zoom in"),
            Section::View,
            "win.zoom-in",
            &["<Ctrl>plus", "<Ctrl>equal", "<Ctrl>KP_Add"],
        ),
        act(
            "zoom-out",
            gettext("Zoom out"),
            Section::View,
            "win.zoom-out",
            &["<Ctrl>minus", "<Ctrl>KP_Subtract"],
        ),
        act(
            "zoom-reset",
            gettext("Reset Zoom"),
            Section::View,
            "win.zoom-reset",
            &["<Ctrl>0", "<Ctrl>KP_0"],
        ),
        // Document
        act(
            "open-doc",
            gettext("Open Document"),
            Section::Document,
            "win.open-doc",
            &["<Ctrl>o"],
        ),
        act(
            "save-doc",
            gettext("Save Document"),
            Section::Document,
            "win.save-doc",
            &["<Ctrl>s"],
        ),
        act(
            "save-doc-as",
            gettext("Save Document As"),
            Section::Document,
            "win.save-doc-as",
            &["<Ctrl><Shift>s"],
        ),
        act(
            "print-doc",
            gettext("Print Document"),
            Section::Document,
            "win.print-doc",
            &["<Ctrl>p"],
        ),
        act(
            "import-file",
            gettext("Import File"),
            Section::Document,
            "win.import-file",
            &["<Ctrl><Shift>i"],
        ),
        act(
            "clear-doc",
            gettext("Clear Document"),
            Section::Document,
            "win.clear-doc",
            &["<Ctrl>l"],
        ),
        hint(
            act(
                "add-page-to-doc",
                gettext("Add Page"),
                Section::Document,
                "win.add-page-to-doc",
                &["<Ctrl><Shift>a"],
            ),
            gettext("When in Fixed-Size Layout"),
        ),
        hint(
            act(
                "remove-page-from-doc",
                gettext("Remove Last Page"),
                Section::Document,
                "win.remove-page-from-doc",
                &["<Ctrl><Shift>r"],
            ),
            gettext("When in Fixed-Size Layout"),
        ),
        act(
            "snap-positions",
            gettext("Snap Positions"),
            Section::Document,
            "win.snap-positions",
            &["<Ctrl><Shift>p"],
        ),
        act(
            "text-bold",
            gettext("Bold"),
            Section::Document,
            "win.text-bold",
            &["<Ctrl>b"],
        ),
        act(
            "text-italic",
            gettext("Italic"),
            Section::Document,
            "win.text-italic",
            &["<Ctrl>i"],
        ),
        act(
            "text-underline",
            gettext("Underline"),
            Section::Document,
            "win.text-underline",
            &["<Ctrl>u"],
        ),
        // Drawing
        act(
            "clipboard-copy",
            gettext("Copy to Clipboard"),
            Section::Drawing,
            "win.clipboard-copy",
            &["<Ctrl>c"],
        ),
        act(
            "clipboard-cut",
            gettext("Cut to Clipboard"),
            Section::Drawing,
            "win.clipboard-cut",
            &["<Ctrl>x"],
        ),
        act(
            "clipboard-paste",
            gettext("Paste Clipboard"),
            Section::Drawing,
            "win.clipboard-paste",
            &["<Ctrl>v"],
        ),
        act(
            "selection-duplicate",
            gettext("Duplicate Selection"),
            Section::Drawing,
            "win.selection-duplicate",
            &["<Ctrl>d"],
        ),
        act(
            "undo",
            gettext("Undo"),
            Section::Drawing,
            "win.undo",
            &["<Ctrl>z"],
        ),
        act(
            "redo",
            gettext("Redo"),
            Section::Drawing,
            "win.redo",
            &["<Ctrl><Shift>z"],
        ),
        // Mouse Gestures
        hint(
            gesture(
                gettext("Move View"),
                Section::MouseGestures,
                "<Alt>",
                "mouse-keys-symbolic",
            ),
            gettext("Mouse drag"),
        ),
        hint(
            gesture(
                gettext("Zoom in/out"),
                Section::MouseGestures,
                "<Shift><Alt>",
                "mouse-keys-symbolic",
            ),
            gettext("Mouse drag"),
        ),
        hint(
            gesture(
                gettext("Zoom in/out"),
                Section::MouseGestures,
                "<Ctrl>",
                "mouse-wheel-scroll-symbolic",
            ),
            gettext("Mouse wheel"),
        ),
        // Touch Gestures
        hint(
            gesture(
                gettext("Zoom in"),
                Section::TouchGestures,
                "",
                "gesture-stretch-symbolic",
            ),
            gettext("Two-finger stretch"),
        ),
        hint(
            gesture(
                gettext("Zoom out"),
                Section::TouchGestures,
                "",
                "gesture-pinch-symbolic",
            ),
            gettext("Two-finger pinch"),
        ),
    ]
});

/// All rebindable shortcuts that are currently registered.
pub(crate) fn rebindable() -> impl Iterator<Item = &'static Key> {
    SHORTCUTS
        .iter()
        .filter_map(Shortcut::key)
        .filter(|key| !key.devel || is_devel())
}

pub(super) fn find(id: &str) -> Option<&'static Key> {
    SHORTCUTS
        .iter()
        .filter_map(Shortcut::key)
        .find(|key| key.id == id)
}

/// Warns once about declared accelerators GTK cannot parse (they would be silently
/// ignored when installed, or render as an empty chip).
pub(super) fn validate_defaults() {
    for shortcut in SHORTCUTS.iter() {
        match shortcut {
            Shortcut::Key(key) => {
                for accel in key.defaults {
                    if gtk4::accelerator_parse(*accel).is_none() {
                        warn!(
                            "shortcut `{}` has unparseable default accel `{accel}`",
                            key.id
                        );
                    }
                }
            }
            Shortcut::Gesture(gesture) => {
                if !gesture.modifiers.is_empty()
                    && gtk4::accelerator_parse(gesture.modifiers).is_none()
                {
                    warn!(
                        "gesture entry `{}` has unparseable modifiers `{}`",
                        gesture.title, gesture.modifiers
                    );
                }
            }
        }
    }
}

pub(super) fn is_devel() -> bool {
    config::PROFILE.to_lowercase() == "devel"
}
