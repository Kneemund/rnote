// Modules
pub mod brushconfig;
pub mod eraserconfig;
pub mod selectorconfig;
pub mod shaperconfig;
pub mod toolsconfig;
pub mod typewriterconfig;

// Re-exports
pub use brushconfig::BrushConfig;
pub use eraserconfig::EraserConfig;
pub use selectorconfig::SelectorConfig;
pub use shaperconfig::ShaperConfig;
pub use toolsconfig::ToolsConfig;
pub use typewriterconfig::TypewriterConfig;

// Imports
use super::shortcuts::ShortcutAction;
use super::{PenStyle, Shortcuts};
use rnote_compose::Color;
use rnote_compose::penevent::ShortcutKey;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename = "pens_config")]
pub struct PensConfig {
    #[serde(default, rename = "brush_config")]
    pub brush_config: BrushConfig,
    #[serde(default, rename = "shaper_config")]
    pub shaper_config: ShaperConfig,
    #[serde(default, rename = "typewriter_config")]
    pub typewriter_config: TypewriterConfig,
    #[serde(default, rename = "eraser_config")]
    pub eraser_config: EraserConfig,
    #[serde(default, rename = "selector_config")]
    pub selector_config: SelectorConfig,
    #[serde(default, rename = "tools_config")]
    pub tools_config: ToolsConfig,

    #[serde(rename = "shortcuts")]
    pub shortcuts: Shortcuts,
    #[serde(rename = "pen_mode_pen_style")]
    pub pen_mode_pen_style: PenStyle,
    #[serde(rename = "pen_mode_eraser_style")]
    pub pen_mode_eraser_style: PenStyle,
}

impl Default for PensConfig {
    fn default() -> Self {
        Self {
            brush_config: BrushConfig::default(),
            shaper_config: ShaperConfig::default(),
            typewriter_config: TypewriterConfig::default(),
            eraser_config: EraserConfig::default(),
            selector_config: SelectorConfig::default(),
            tools_config: ToolsConfig::default(),
            shortcuts: Shortcuts::default(),
            pen_mode_pen_style: PenStyle::default(),
            pen_mode_eraser_style: PenStyle::Eraser,
        }
    }
}

impl PensConfig {
    /// The colors which the global colorpicker shows for the currently selected pen style.
    ///
    /// The first entry is the stroke color, the second the fill color, which is `None` for
    /// styles that don't have one. The stroke color falls back to [`Color::TRANSPARENT`]
    /// when the style does not draw a stroke.
    ///
    /// Returns `None` for pens without any colors, like the eraser.
    pub fn current_colors(&self, pen_style: PenStyle) -> Option<(Color, Option<Color>)> {
        match pen_style {
            PenStyle::Brush => Some((
                self.brush_config
                    .stroke_color_for_style(self.brush_config.style)
                    .unwrap_or(Color::TRANSPARENT),
                self.brush_config
                    .fill_color_for_style(self.brush_config.style),
            )),
            PenStyle::Shaper => Some((
                self.shaper_config
                    .stroke_color_for_style(self.shaper_config.style)
                    .unwrap_or(Color::TRANSPARENT),
                self.shaper_config
                    .fill_color_for_style(self.shaper_config.style),
            )),
            PenStyle::Typewriter => Some((self.typewriter_config.text_style.color, None)),
            PenStyle::Eraser | PenStyle::Selector | PenStyle::Tools => None,
        }
    }

    /// Remember a new stroke color for the currently selected pen style.
    ///
    /// Does nothing for pens without any colors.
    pub fn set_current_stroke_color(&mut self, pen_style: PenStyle, stroke_color: Color) {
        match pen_style {
            PenStyle::Brush => {
                let style = self.brush_config.style;
                self.brush_config
                    .set_stroke_color_for_style(style, stroke_color);
            }
            PenStyle::Shaper => {
                let style = self.shaper_config.style;
                self.shaper_config
                    .set_stroke_color_for_style(style, stroke_color);
            }
            PenStyle::Typewriter => self.typewriter_config.text_style.color = stroke_color,
            PenStyle::Eraser | PenStyle::Selector | PenStyle::Tools => {}
        }
    }

    /// Remember a new fill color for the currently selected pen style.
    ///
    /// Does nothing for pens or styles without a fill color.
    pub fn set_current_fill_color(&mut self, pen_style: PenStyle, fill_color: Color) {
        match pen_style {
            PenStyle::Brush => {
                let style = self.brush_config.style;
                self.brush_config
                    .set_fill_color_for_style(style, fill_color);
            }
            PenStyle::Shaper => {
                let style = self.shaper_config.style;
                self.shaper_config
                    .set_fill_color_for_style(style, fill_color);
            }
            PenStyle::Typewriter | PenStyle::Eraser | PenStyle::Selector | PenStyle::Tools => {}
        }
    }

    /// Get the current registered shortcuts.
    pub fn shortcuts(&self) -> Shortcuts {
        self.shortcuts.clone()
    }

    /// Clear all shortcuts
    pub fn clear_shortcuts(&mut self) {
        self.shortcuts.clear();
    }

    /// Replace all shortcuts.
    pub fn set_shortcuts(&mut self, shortcuts: Shortcuts) {
        self.shortcuts = shortcuts;
    }

    /// Register a shortcut key and action.
    pub fn register_shortcut(&mut self, key: ShortcutKey, action: ShortcutAction) {
        self.shortcuts.insert(key, action);
    }

    /// Remove the shortcut action for the given shortcut key, if it is registered.
    pub fn remove_shortcut(&mut self, key: ShortcutKey) -> Option<ShortcutAction> {
        self.shortcuts.remove(&key)
    }

    // Get the current registered action for the given shortcut key.
    pub fn get_shortcut_action(&self, key: ShortcutKey) -> Option<ShortcutAction> {
        self.shortcuts.get(&key).cloned()
    }

    /// List all current registered shortcut keys and their action.
    pub fn list_current_shortcuts(&self) -> Vec<(ShortcutKey, ShortcutAction)> {
        self.shortcuts
            .iter()
            .map(|(key, action)| (*key, *action))
            .collect()
    }
}
