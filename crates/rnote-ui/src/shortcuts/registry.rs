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
    /// Additional display-only hint (the row subtitle), translated when the registry
    /// is built.
    pub hint: String,
    /// Only registered and listed in devel builds.
    pub devel: bool,
}

/// A display-only gesture (pointer or touch) that cannot be rebound.
#[derive(Clone, Debug)]
pub(crate) struct Gesture {
    pub title: String,
    pub section: Section,
    /// Modifiers held for the gesture, spelled as an accelerator; empty for none.
    pub modifiers: &'static str,
    /// Display-only hint (the row subtitle), translated when the registry is built.
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
    /// A shortcut dispatching the GTK detailed action `action`, which GTK's accelerator
    /// machinery triggers.
    fn action(
        id: &'static str,
        title: String,
        section: Section,
        action: &'static str,
        defaults: &'static [&'static str],
    ) -> Self {
        Self::Key(Key {
            id,
            title,
            section,
            dispatch: Dispatch::Action(action),
            defaults,
            hint: String::new(),
            devel: false,
        })
    }

    /// A shortcut matched in code; test it with [`super::matches`].
    fn code(
        id: &'static str,
        title: String,
        section: Section,
        defaults: &'static [&'static str],
    ) -> Self {
        Self::Key(Key {
            id,
            title,
            section,
            dispatch: Dispatch::Code,
            defaults,
            hint: String::new(),
            devel: false,
        })
    }

    /// Sets the display-only hint (the row subtitle).
    fn hint(self, hint: String) -> Self {
        match self {
            Self::Key(key) => Self::Key(Key { hint, ..key }),
            Self::Gesture(gesture) => Self::Gesture(Gesture { hint, ..gesture }),
        }
    }

    /// Restricts the shortcut to devel builds.
    fn devel_only(self) -> Self {
        match self {
            Self::Key(key) => Self::Key(Key { devel: true, ..key }),
            gesture => gesture,
        }
    }

    /// A display-only gesture that cannot be rebound.
    fn gesture(
        title: String,
        section: Section,
        modifiers: &'static str,
        icon: &'static str,
    ) -> Self {
        Self::Gesture(Gesture {
            title,
            section,
            modifiers,
            hint: String::new(),
            icon,
        })
    }

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

/// Every shortcut, in display order. Built - and translated - once, on first access.
pub(crate) static SHORTCUTS: Lazy<Vec<Shortcut>> = Lazy::new(|| {
    vec![
        // General
        Shortcut::action(
            "show-shortcuts",
            gettext("Show Keyboard Shortcuts"),
            Section::General,
            "win.keyboard-shortcuts",
            &["<Ctrl>question"],
        ),
        Shortcut::action(
            "new-window",
            gettext("New Window"),
            Section::General,
            "app.new-window",
            &["<Ctrl>n"],
        ),
        Shortcut::action(
            "new-tab",
            gettext("New Tab"),
            Section::General,
            "win.new-tab",
            &["<Ctrl>t"],
        ),
        Shortcut::action(
            "close-tab",
            gettext("Close the Active Tab"),
            Section::General,
            "win.active-tab-close",
            &["<Ctrl>w"],
        ),
        Shortcut::action(
            "quit",
            gettext("Quit the Application"),
            Section::General,
            "app.quit",
            &["<Ctrl>q"],
        ),
        Shortcut::action(
            "toggle-overview",
            gettext("Toggle Tabs Overview"),
            Section::General,
            "win.toggle-overview",
            &["<Ctrl><Shift>o"],
        ),
        Shortcut::action(
            "open-canvas-menu",
            gettext("Open the Canvas-Menu"),
            Section::General,
            "win.open-canvasmenu",
            &["F9"],
        ),
        Shortcut::action(
            "open-app-menu",
            gettext("Open the App-Menu"),
            Section::General,
            "win.open-appmenu",
            &["F10"],
        ),
        Shortcut::action(
            "fullscreen",
            gettext("Toggle Fullscreen"),
            Section::General,
            "win.fullscreen",
            &["F11"],
        ),
        Shortcut::action(
            "visual-debug",
            gettext("Visual Debug"),
            Section::General,
            "win.visual-debug",
            &["<Ctrl><Shift>v"],
        )
        .devel_only(),
        // Navigation
        Shortcut::action(
            "pen-style-brush",
            gettext("Switch to the 'Brush'"),
            Section::Navigation,
            "win.pen-style::brush",
            &["<Ctrl>1", "<Ctrl>KP_1"],
        ),
        Shortcut::action(
            "pen-style-shaper",
            gettext("Switch to the 'Shaper'"),
            Section::Navigation,
            "win.pen-style::shaper",
            &["<Ctrl>2", "<Ctrl>KP_2"],
        ),
        Shortcut::action(
            "pen-style-typewriter",
            gettext("Switch to the 'Typewriter'"),
            Section::Navigation,
            "win.pen-style::typewriter",
            &["<Ctrl>3", "<Ctrl>KP_3"],
        ),
        Shortcut::action(
            "pen-style-eraser",
            gettext("Switch to the 'Eraser'"),
            Section::Navigation,
            "win.pen-style::eraser",
            &["<Ctrl>4", "<Ctrl>KP_4"],
        ),
        Shortcut::action(
            "pen-style-selector",
            gettext("Switch to the 'Selector'"),
            Section::Navigation,
            "win.pen-style::selector",
            &["<Ctrl>5", "<Ctrl>KP_5"],
        ),
        Shortcut::action(
            "pen-style-tools",
            gettext("Switch to the 'Tools'"),
            Section::Navigation,
            "win.pen-style::tools",
            &["<Ctrl>6", "<Ctrl>KP_6"],
        ),
        Shortcut::code(
            "pen-shortcut-keyboard",
            gettext("Keyboard Pen Shortcut"),
            Section::Navigation,
            &["<Ctrl>space"],
        ),
        Shortcut::action(
            "select-color-1",
            gettext("Select Color 1"),
            Section::Navigation,
            "win.set-color-1",
            &["<Ctrl><Alt>1", "<Ctrl><Alt>KP_1"],
        ),
        Shortcut::action(
            "select-color-2",
            gettext("Select Color 2"),
            Section::Navigation,
            "win.set-color-2",
            &["<Ctrl><Alt>2", "<Ctrl><Alt>KP_2"],
        ),
        Shortcut::action(
            "select-color-3",
            gettext("Select Color 3"),
            Section::Navigation,
            "win.set-color-3",
            &["<Ctrl><Alt>3", "<Ctrl><Alt>KP_3"],
        ),
        Shortcut::action(
            "select-color-4",
            gettext("Select Color 4"),
            Section::Navigation,
            "win.set-color-4",
            &["<Ctrl><Alt>4", "<Ctrl><Alt>KP_4"],
        ),
        Shortcut::action(
            "select-color-5",
            gettext("Select Color 5"),
            Section::Navigation,
            "win.set-color-5",
            &["<Ctrl><Alt>5", "<Ctrl><Alt>KP_5"],
        ),
        Shortcut::action(
            "select-color-6",
            gettext("Select Color 6"),
            Section::Navigation,
            "win.set-color-6",
            &["<Ctrl><Alt>6", "<Ctrl><Alt>KP_6"],
        ),
        Shortcut::action(
            "select-color-7",
            gettext("Select Color 7"),
            Section::Navigation,
            "win.set-color-7",
            &["<Ctrl><Alt>7", "<Ctrl><Alt>KP_7"],
        ),
        Shortcut::action(
            "select-color-8",
            gettext("Select Color 8"),
            Section::Navigation,
            "win.set-color-8",
            &["<Ctrl><Alt>8", "<Ctrl><Alt>KP_8"],
        ),
        Shortcut::action(
            "select-color-9",
            gettext("Select Color 9"),
            Section::Navigation,
            "win.set-color-9",
            &["<Ctrl><Alt>9", "<Ctrl><Alt>KP_9"],
        ),
        // View
        Shortcut::action(
            "zoom-in",
            gettext("Zoom in"),
            Section::View,
            "win.zoom-in",
            &["<Ctrl>plus", "<Ctrl>equal", "<Ctrl>KP_Add"],
        ),
        Shortcut::action(
            "zoom-out",
            gettext("Zoom out"),
            Section::View,
            "win.zoom-out",
            &["<Ctrl>minus", "<Ctrl>KP_Subtract"],
        ),
        Shortcut::action(
            "zoom-reset",
            gettext("Reset Zoom"),
            Section::View,
            "win.zoom-reset",
            &["<Ctrl>0", "<Ctrl>KP_0"],
        ),
        // Document
        Shortcut::action(
            "open-doc",
            gettext("Open Document"),
            Section::Document,
            "win.open-doc",
            &["<Ctrl>o"],
        ),
        Shortcut::action(
            "save-doc",
            gettext("Save Document"),
            Section::Document,
            "win.save-doc",
            &["<Ctrl>s"],
        ),
        Shortcut::action(
            "save-doc-as",
            gettext("Save Document As"),
            Section::Document,
            "win.save-doc-as",
            &["<Ctrl><Shift>s"],
        ),
        Shortcut::action(
            "print-doc",
            gettext("Print Document"),
            Section::Document,
            "win.print-doc",
            &["<Ctrl>p"],
        ),
        Shortcut::action(
            "import-file",
            gettext("Import File"),
            Section::Document,
            "win.import-file",
            &["<Ctrl><Shift>i"],
        ),
        Shortcut::action(
            "clear-doc",
            gettext("Clear Document"),
            Section::Document,
            "win.clear-doc",
            &["<Ctrl>l"],
        ),
        Shortcut::action(
            "add-page-to-doc",
            gettext("Add Page"),
            Section::Document,
            "win.add-page-to-doc",
            &["<Ctrl><Shift>a"],
        )
        .hint(gettext("When in Fixed-Size Layout")),
        Shortcut::action(
            "remove-page-from-doc",
            gettext("Remove Last Page"),
            Section::Document,
            "win.remove-page-from-doc",
            &["<Ctrl><Shift>r"],
        )
        .hint(gettext("When in Fixed-Size Layout")),
        Shortcut::action(
            "snap-positions",
            gettext("Snap Positions"),
            Section::Document,
            "win.snap-positions",
            &["<Ctrl><Shift>p"],
        ),
        Shortcut::action(
            "text-bold",
            gettext("Bold"),
            Section::Document,
            "win.text-bold",
            &["<Ctrl>b"],
        ),
        Shortcut::action(
            "text-italic",
            gettext("Italic"),
            Section::Document,
            "win.text-italic",
            &["<Ctrl>i"],
        ),
        Shortcut::action(
            "text-underline",
            gettext("Underline"),
            Section::Document,
            "win.text-underline",
            &["<Ctrl>u"],
        ),
        // Drawing
        Shortcut::action(
            "clipboard-copy",
            gettext("Copy to Clipboard"),
            Section::Drawing,
            "win.clipboard-copy",
            &["<Ctrl>c"],
        ),
        Shortcut::action(
            "clipboard-cut",
            gettext("Cut to Clipboard"),
            Section::Drawing,
            "win.clipboard-cut",
            &["<Ctrl>x"],
        ),
        Shortcut::action(
            "clipboard-paste",
            gettext("Paste Clipboard"),
            Section::Drawing,
            "win.clipboard-paste",
            &["<Ctrl>v"],
        ),
        Shortcut::action(
            "selection-duplicate",
            gettext("Duplicate Selection"),
            Section::Drawing,
            "win.selection-duplicate",
            &["<Ctrl>d"],
        ),
        Shortcut::action(
            "undo",
            gettext("Undo"),
            Section::Drawing,
            "win.undo",
            &["<Ctrl>z"],
        ),
        Shortcut::action(
            "redo",
            gettext("Redo"),
            Section::Drawing,
            "win.redo",
            &["<Ctrl><Shift>z"],
        ),
        // Mouse Gestures
        Shortcut::gesture(
            gettext("Move View"),
            Section::MouseGestures,
            "<Alt>",
            "mouse-keys-symbolic",
        )
        .hint(gettext("Mouse drag")),
        Shortcut::gesture(
            gettext("Zoom in/out"),
            Section::MouseGestures,
            "<Shift><Alt>",
            "mouse-keys-symbolic",
        )
        .hint(gettext("Mouse drag")),
        Shortcut::gesture(
            gettext("Zoom in/out"),
            Section::MouseGestures,
            "<Ctrl>",
            "mouse-wheel-scroll-symbolic",
        )
        .hint(gettext("Mouse wheel")),
        // Touch Gestures
        Shortcut::gesture(
            gettext("Zoom in"),
            Section::TouchGestures,
            "",
            "gesture-stretch-symbolic",
        )
        .hint(gettext("Two-finger stretch")),
        Shortcut::gesture(
            gettext("Zoom out"),
            Section::TouchGestures,
            "",
            "gesture-pinch-symbolic",
        )
        .hint(gettext("Two-finger pinch")),
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

/// Warns about declared accelerators GTK cannot parse (they would be ignored or
/// render as an empty chip).
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
