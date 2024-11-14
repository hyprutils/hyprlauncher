use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Corners {
    pub window: i32,
    pub search: i32,
    pub list_item: i32,
}

impl Default for Corners {
    fn default() -> Self {
        Self {
            window: 12,
            search: 8,
            list_item: 8,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Colors {
    pub window_bg: String,
    pub search_bg: String,
    pub search_bg_focused: String,
    pub item_bg: String,
    pub item_bg_hover: String,
    pub item_bg_selected: String,
    pub search_text: String,
    pub search_caret: String,
    pub item_name: String,
    pub item_description: String,
    pub item_path: String,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            window_bg: String::from("#0f0f0f"),
            search_bg: String::from("#1f1f1f"),
            search_bg_focused: String::from("#282828"),
            item_bg: String::from("#0f0f0f"),
            item_bg_hover: String::from("#181818"),
            item_bg_selected: String::from("#1f1f1f"),
            search_text: String::from("#e0e0e0"),
            search_caret: String::from("#808080"),
            item_name: String::from("#ffffff"),
            item_description: String::from("#a0a0a0"),
            item_path: String::from("#808080"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Spacing {
    pub search_margin: i32,
    pub search_padding: i32,
    pub item_margin: i32,
    pub item_padding: i32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            search_margin: 12,
            search_padding: 12,
            item_margin: 6,
            item_padding: 4,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Typography {
    pub search_font_size: i32,
    pub item_name_size: i32,
    pub item_description_size: i32,
    pub item_path_size: i32,
    pub item_path_font_family: String,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            search_font_size: 16,
            item_name_size: 14,
            item_description_size: 12,
            item_path_size: 12,
            item_path_font_family: String::from("monospace"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Theme {
    pub colors: Colors,
    pub corners: Corners,
    pub spacing: Spacing,
    pub typography: Typography,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub window: Window,
    pub theme: Theme,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize)]
pub enum WindowAnchor {
    center,
    top,
    bottom,
    left,
    right,
    top_left,
    top_right,
    bottom_left,
    bottom_right,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Window {
    pub width: i32,
    pub height: i32,
    pub anchor: WindowAnchor,
    pub margin_top: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,
    pub margin_right: i32,
    pub show_descriptions: bool,
    pub show_paths: bool,
    pub show_icons: bool,
    pub show_search: bool,
    pub vim_keys: bool,
    pub show_border: bool,
    pub border_width: i32,
    pub border_color: String,
    pub show_scrollbar: bool,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            width: 600,
            height: 600,
            show_descriptions: false,
            show_paths: false,
            show_icons: true,
            show_search: true,
            vim_keys: true,
            anchor: WindowAnchor::center,
            margin_top: 0,
            margin_bottom: 0,
            margin_left: 0,
            margin_right: 0,
            show_border: true,
            border_width: 2,
            border_color: String::from("#333333"),
            show_scrollbar: true,
        }
    }
}

impl Config {
    fn ensure_config_dir() -> PathBuf {
        let config_path = config_dir()
            .map(|mut p| {
                p.push("hyprlauncher");
                p
            })
            .unwrap_or_else(|| PathBuf::from("~/.config/hyprlauncher"));

        if !config_path.exists() {
            fs::create_dir_all(&config_path).unwrap_or_default();
        }

        let css_path = config_path.join("style.css");
        if !css_path.exists() {
            fs::write(&css_path, Self::get_default_css()).unwrap_or_default();
        }

        config_path
    }

    pub fn load() -> Self {
        let config_path = Self::ensure_config_dir();
        let config_file = config_path.join("config.json");
        let default_config = Config::default();

        if !config_file.exists() {
            if let Ok(contents) = serde_json::to_string_pretty(&default_config) {
                fs::write(&config_file, contents).unwrap_or_default();
            }
            return default_config;
        }

        let existing_config: serde_json::Value = fs::read_to_string(&config_file)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        let merged_config = if let Ok(contents) = serde_json::to_string(&default_config) {
            let default_json: serde_json::Value =
                serde_json::from_str(&contents).unwrap_or_default();
            merge_json(existing_config, default_json.clone(), &default_json)
        } else {
            existing_config
        };

        if let Ok(contents) = serde_json::to_string_pretty(&merged_config) {
            fs::write(&config_file, contents).unwrap_or_default();
        }

        serde_json::from_value(merged_config).unwrap_or_default()
    }

    pub fn load_css() -> String {
        let config_path = Self::ensure_config_dir();
        let css_path = config_path.join("style.css");

        let css = Self::generate_css(&Config::load());

        fs::write(&css_path, &css).unwrap_or_default();

        css
    }

    fn generate_css(config: &Config) -> String {
        let theme = &config.theme;
        let window = &config.window;

        let scrollbar_style = if window.show_scrollbar {
            String::new()
        } else {
            String::from(
                "
scrollbar {
    opacity: 0;
    -gtk-secondary-caret-color: transparent;
}",
            )
        };

        format!(
            "/*
 * ███████╗████████╗ ██████╗ ██████╗     ██╗
 * ██╔════╝╚══██╔══╝██╔═══██╗██╔══██╗    ██║
 * ███████╗   ██║   ██║   ██║██████╔╝    ██║
 * ╚════██║   ██║   ██║   ██║██╔═══╝     ╚═╝
 * ███████║   ██║   ╚██████╔╝██║         ██╗
 * ╚══════╝   ╚═╝    ╚═════╝ ╚═╝         ╚═╝
 *
 * DO NOT EDIT THIS FILE DIRECTLY!
 * 
 * This file is automatically generated from your config.json theme settings.
 * Any direct changes to this file will be overwritten.
 * 
 * To customize the appearance, edit the theme section in:
 * ~/.config/hyprlauncher/config.json
 */

window {{ 
    background-color: {};
    border-radius: {}px;
    {}
}}

list {{ 
    background: {};
}}

list row {{ 
    padding: {}px;
    margin: {}px;
    border-radius: {}px;
    background: {};
    transition: all 200ms ease;
}}

list row:selected {{ 
    background-color: {};
}}

list row:hover:not(:selected) {{
    background-color: {};
}}

entry {{
    margin: {}px;
    padding: {}px;
    border-radius: {}px;
    background-color: {};
    color: {};
    caret-color: {};
    font-size: {}px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}}

entry:focus {{
    background-color: {};
}}

.app-name {{
    color: {};
    font-size: {}px;
    font-weight: bold;
    margin-right: 8px;
}}

.app-description {{
    color: {};
    font-size: {}px;
    margin-right: 8px;
}}

.app-path {{
    color: {};
    font-size: {}px;
    font-family: {};
    opacity: 0.8;
}}

scrollbar {{
    background-color: transparent;
    border: none;
}}

scrollbar slider {{
    min-width: 6px;
    min-height: 6px;
    border-radius: 3px;
    background-color: alpha(#808080, 0.7);
}}

scrollbar.vertical slider {{
    min-width: 6px;
}}

scrollbar.horizontal slider {{
    min-height: 6px;
}}

{}",
            theme.colors.window_bg,
            theme.corners.window,
            if window.show_border {
                format!(
                    "border: {}px solid {};",
                    window.border_width, window.border_color
                )
            } else {
                String::from("border: none;")
            },
            theme.colors.window_bg,
            theme.spacing.item_padding,
            theme.spacing.item_margin,
            theme.corners.list_item,
            theme.colors.item_bg,
            theme.colors.item_bg_selected,
            theme.colors.item_bg_hover,
            theme.spacing.search_margin,
            theme.spacing.search_padding,
            theme.corners.search,
            theme.colors.search_bg,
            theme.colors.search_text,
            theme.colors.search_caret,
            theme.typography.search_font_size,
            theme.colors.search_bg_focused,
            theme.colors.item_name,
            theme.typography.item_name_size,
            theme.colors.item_description,
            theme.typography.item_description_size,
            theme.colors.item_path,
            theme.typography.item_path_size,
            theme.typography.item_path_font_family,
            scrollbar_style,
        )
    }

    fn get_default_css() -> String {
        Self::generate_css(&Config::default())
    }
}

fn merge_json(
    existing: serde_json::Value,
    default: serde_json::Value,
    schema: &serde_json::Value,
) -> serde_json::Value {
    match (existing, default) {
        (serde_json::Value::Object(mut existing_obj), serde_json::Value::Object(default_obj)) => {
            let mut result = serde_json::Map::new();

            for (key, schema_val) in schema.as_object().unwrap() {
                if let Some(existing_val) = existing_obj.remove(key) {
                    if schema_val.is_object() && existing_val.is_object() {
                        result.insert(
                            key.clone(),
                            merge_json(
                                existing_val,
                                default_obj.get(key).cloned().unwrap_or_default(),
                                schema_val,
                            ),
                        );
                    } else {
                        result.insert(key.clone(), existing_val);
                    }
                } else if let Some(default_val) = default_obj.get(key) {
                    result.insert(key.clone(), default_val.clone());
                }
            }

            serde_json::Value::Object(result)
        }
        (_, default) => default,
    }
}
