use crate::log;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
        LazyLock,
    },
    thread,
    time::Duration,
};

static CONFIG_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let xdg_config_dirs = env::var("XDG_CONFIG_DIRS").unwrap_or_else(|_| String::from("/etc/xdg"));

    for dir in xdg_config_dirs.split(':') {
        let config_dir = PathBuf::from(dir).join("hyprlauncher");
        if config_dir.exists() {
            return config_dir;
        }
    }

    let default_config_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".config")
        .join("hyprlauncher");

    if !default_config_path.exists() {
        fs::create_dir_all(&default_config_path).unwrap_or_default();
    }

    default_config_path
});

pub static LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
    pub item_name_selected: String,
    pub item_description: String,
    pub item_description_selected: String,
    pub item_path: String,
    pub item_path_selected: String,
    pub border: String,
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
            item_name_selected: String::from("#ffffff"),
            item_description: String::from("#a0a0a0"),
            item_description_selected: String::from("#a0a0a0"),
            item_path: String::from("#808080"),
            item_path_selected: String::from("#808080"),
            border: String::from("#333333"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct Theme {
    pub colors: Colors,
    pub corners: Corners,
    pub spacing: Spacing,
    pub typography: Typography,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    pub window: Window,
    pub theme: Theme,
    pub debug: Debug,
    pub finder: Finder,
    pub dmenu: Dmenu,
    pub web_search: WebSearch,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
    pub custom_navigate_keys: NavigateKeys,
    pub show_border: bool,
    pub border_width: i32,
    pub use_gtk_colors: bool,
    pub use_custom_css: bool,
    pub max_entries: usize,
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
            custom_navigate_keys: NavigateKeys::default(),
            anchor: WindowAnchor::center,
            margin_top: 0,
            margin_bottom: 0,
            margin_left: 0,
            margin_right: 0,
            show_border: true,
            border_width: 2,
            use_gtk_colors: false,
            use_custom_css: false,
            max_entries: 50,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Debug {
    pub disable_auto_focus: bool,
    pub enable_logging: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NavigateKeys {
    pub up: String,
    pub down: String,
    pub delete_word: String,
}

impl Default for NavigateKeys {
    fn default() -> Self {
        Self {
            up: String::from("k"),
            down: String::from("j"),
            delete_word: String::from("h"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Finder {
    pub show_hidden: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Dmenu {
    pub allow_invalid: bool,
    pub case_sensitive: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SearchEngine {
    #[serde(rename = "duckduckgo")]
    DuckDuckGo,
    #[serde(rename = "google")]
    Google,
    #[serde(rename = "bing")]
    Bing,
    #[serde(rename = "brave")]
    Brave,
    #[serde(rename = "ecosia")]
    Ecosia,
    #[serde(rename = "startpage")]
    Startpage,
}

impl SearchEngine {
    pub fn get_url(&self) -> String {
        match self {
            SearchEngine::DuckDuckGo => String::from("https://duckduckgo.com/?q="),
            SearchEngine::Google => String::from("https://www.google.com/search?q="),
            SearchEngine::Bing => String::from("https://www.bing.com/search?q="),
            SearchEngine::Brave => String::from("https://search.brave.com/search?q="),
            SearchEngine::Ecosia => String::from("https://www.ecosia.org/search?q="),
            SearchEngine::Startpage => String::from("https://www.startpage.com/do/search?q="),
        }
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::DuckDuckGo
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct WebSearch {
    pub enabled: bool,
    pub engine: SearchEngine,
}

impl Config {
    fn config_dir() -> &'static PathBuf {
        &CONFIG_DIR
    }

    pub fn load() -> Self {
        let config_file = Self::config_dir().join("config.json");
        log!("Loading configuration from: {:?}", config_file);
        let default_config = Config::default();
        LOGGING_ENABLED.store(default_config.debug.enable_logging, Ordering::SeqCst);

        if !config_file.exists() {
            log!("Config file not found, creating default configuration");
            if let Ok(contents) = serde_json::to_string_pretty(&default_config) {
                fs::write(&config_file, contents).unwrap_or_default();
            }
            return default_config;
        }

        log!("Reading existing configuration");
        let file_contents = match fs::read_to_string(&config_file) {
            Ok(contents) => contents,
            Err(e) => {
                log!("Error reading config file: {}", e);
                return default_config;
            }
        };

        let existing_config: serde_json::Value = match serde_json::from_str(&file_contents) {
            Ok(config) => config,
            Err(e) => {
                log!(
                    "Error parsing config JSON: {} at line {}, column {}",
                    e,
                    e.line(),
                    e.column()
                );
                log!("Attempting to merge partial configuration");
                match serde_json::from_str::<serde_json::Value>(&file_contents) {
                    Ok(partial_config) => partial_config,
                    Err(_) => {
                        log!("Unable to parse partial config, using defaults");
                        return default_config;
                    }
                }
            }
        };

        let default_json = match serde_json::to_value(&default_config) {
            Ok(json) => json,
            Err(e) => {
                log!("Error converting default config to JSON: {}", e);
                return default_config;
            }
        };

        let merged_config = merge_json(existing_config, default_json.clone(), &default_json);

        if let Ok(pretty_merged) = serde_json::to_string_pretty(&merged_config) {
            if pretty_merged != file_contents {
                log!("Writing merged configuration back to file");
                fs::write(&config_file, pretty_merged).unwrap_or_default();
            }
        }

        let config = match serde_json::from_value(merged_config.clone()) {
            Ok(config) => config,
            Err(e) => {
                log!("Error converting merged config to struct: {}", e);
                log!(
                    "Merged config was: {}",
                    serde_json::to_string_pretty(&merged_config).unwrap_or_default()
                );
                default_config
            }
        };

        LOGGING_ENABLED.store(config.debug.enable_logging, Ordering::SeqCst);
        config
    }

    pub fn get_css(&self) -> String {
        if self.window.use_custom_css {
            let custom_css_path = Self::config_dir().join("style.css");
            if let Ok(css) = fs::read_to_string(&custom_css_path) {
                return css;
            }
            log!(
                "Custom CSS file not found at {:?}, falling back to default styling",
                custom_css_path
            );
        }

        let theme = &self.theme;
        let window = &self.window;

        let border_style = if window.show_border {
            if window.use_gtk_colors {
                format!("border: {}px solid @borders;", window.border_width)
            } else {
                format!(
                    "border: {}px solid {};",
                    window.border_width, theme.colors.border
                )
            }
        } else {
            String::from("border: none;")
        };

        if window.use_gtk_colors {
            format!(
                "window {{
                    background-color: @theme_bg_color;
                    border-radius: {}px;
                    {}
                }}
                listview {{
                    background: @theme_bg_color;
                }}
                listview > row {{
                    padding: {}px;
                    margin: {}px;
                    border-radius: {}px;
                    background: @theme_bg_color;
                    transition: all 200ms ease;
                }}
                listview > row:selected {{
                    background-color: @theme_selected_bg_color;
                }}
                listview > row:hover:not(:selected) {{
                    background-color: mix(@theme_bg_color, @theme_fg_color, 0.95);
                }}
                entry {{
                    margin: {}px;
                    padding: {}px;
                    border-radius: {}px;
                    background-color: @theme_base_color;
                    color: @theme_text_color;
                    caret-color: @theme_text_color;
                    font-size: {}px;
                    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
                    outline: none;
                }}
                entry:focus {{
                    background-color: @theme_base_color;
                    outline: none;
                }}
                .app-name {{
                    color: @theme_text_color;
                    font-size: {}px;
                    font-weight: bold;
                    margin-right: 8px;
                }}
                listview > row:selected .app-name,
                listview > row:hover:not(:selected) .app-name {{
                    color: @theme_selected_fg_color;
                }}
                .app-description {{
                    color: mix(@theme_fg_color, @theme_bg_color, 0.7);
                    font-size: {}px;
                    margin-right: 8px;
                }}
                listview > row:selected .app-description,
                listview > row:hover:not(:selected) .app-description {{
                    color: mix(@theme_selected_fg_color, @theme_bg_color, 0.7);
                }}
                .app-path {{
                    color: mix(@theme_fg_color, @theme_bg_color, 0.5);
                    font-size: {}px;
                    font-family: {};
                    opacity: 0.8;
                }}
                listview > row:selected .app-path,
                listview > row:hover:not(:selected) .app-path {{
                    color: mix(@theme_selected_fg_color, @theme_bg_color, 0.6);
                }}
                scrollbar {{ opacity: 0; }}",
                theme.corners.window,
                border_style,
                theme.spacing.item_padding,
                theme.spacing.item_margin,
                theme.corners.list_item,
                theme.spacing.search_margin,
                theme.spacing.search_padding,
                theme.corners.search,
                theme.typography.search_font_size,
                theme.typography.item_name_size,
                theme.typography.item_description_size,
                theme.typography.item_path_size,
                theme.typography.item_path_font_family,
            )
        } else {
            format!(
                "window {{
                    background-color: {};
                    border-radius: {}px;
                    {}
                }}
                listview {{
                    background: {};
                }}
                listview > row {{
                    padding: {}px;
                    margin: {}px;
                    border-radius: {}px;
                    background: {};
                    transition: all 200ms ease;
                }}
                listview > row:selected {{
                    background-color: {};
                }}
                listview > row:hover:not(:selected) {{
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
                    outline: none;
                }}
                entry:focus {{
                    background-color: {};
                    outline: none;
                }}
                .app-name {{
                    color: {};
                    font-size: {}px;
                    font-weight: bold;
                    margin-right: 8px;
                }}
                listview > row:selected .app-name,
                listview > row:hover:not(:selected) .app-name {{
                    color: {};
                }}
                .app-description {{
                    color: {};
                    font-size: {}px;
                    margin-right: 8px;
                }}
                listview > row:selected .app-description,
                listview > row:hover:not(:selected) .app-description {{
                    color: {};
                }}
                .app-path {{
                    color: {};
                    font-size: {}px;
                    font-family: {};
                    opacity: 0.8;
                }}
                listview > row:selected .app-path,
                listview > row:hover:not(:selected) .app-path {{
                    color: {};
                }}
                scrollbar {{ opacity: 0; }}",
                theme.colors.window_bg,
                theme.corners.window,
                border_style,
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
                theme.colors.item_name_selected,
                theme.colors.item_description,
                theme.typography.item_description_size,
                theme.colors.item_description_selected,
                theme.colors.item_path,
                theme.typography.item_path_size,
                theme.typography.item_path_font_family,
                theme.colors.item_path_selected,
            )
        }
    }

    pub fn watch_changes<F: Fn() + Send + 'static>(callback: F) {
        let config_path = Self::config_dir().join("config.json");
        let css_path = Self::config_dir().join("style.css");
        log!("Setting up config file watcher for: {:?}", config_path);

        let mut last_content = match fs::read_to_string(&config_path) {
            Ok(content) => {
                log!("Initial config content loaded");
                Some(content)
            }
            Err(e) => {
                log!("Error reading initial config: {}", e);
                None
            }
        };

        let mut last_css_content = match fs::read_to_string(&css_path) {
            Ok(content) => {
                log!("Initial CSS content loaded");
                Some(content)
            }
            Err(_) => None,
        };

        let mut last_update = std::time::Instant::now();

        thread::spawn(move || {
            let (tx, rx) = channel();

            let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
                .expect("Failed to create file watcher");

            watcher
                .watch(config_path.parent().unwrap(), RecursiveMode::NonRecursive)
                .expect("Failed to watch config directory");

            loop {
                match rx.recv() {
                    Ok(event) => {
                        log!("Received file system event: {:?}", event);
                        let now = std::time::Instant::now();
                        if now.duration_since(last_update).as_millis() > 250 {
                            thread::sleep(Duration::from_millis(50));

                            let config_changed = match fs::read_to_string(&config_path) {
                                Ok(new_content) => {
                                    if last_content.as_ref() != Some(&new_content) {
                                        last_content = Some(new_content);
                                        true
                                    } else {
                                        false
                                    }
                                }
                                Err(e) => {
                                    log!("Error reading config file: {}", e);
                                    false
                                }
                            };

                            let css_changed = match fs::read_to_string(&css_path) {
                                Ok(new_content) => {
                                    if last_css_content.as_ref() != Some(&new_content) {
                                        last_css_content = Some(new_content);
                                        true
                                    } else {
                                        false
                                    }
                                }
                                Err(_) => false,
                            };

                            if config_changed || css_changed {
                                last_update = now;
                                callback();
                            }
                        }
                    }
                    Err(e) => {
                        log!("Watch error: {:?}", e);
                        break;
                    }
                }
            }
        });
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

            let schema_obj = match schema.as_object() {
                Some(obj) => obj,
                None => return serde_json::Value::Object(default_obj),
            };

            const MAX_DEPTH: usize = 10;
            static CURRENT_DEPTH: std::sync::atomic::AtomicUsize =
                std::sync::atomic::AtomicUsize::new(0);

            let depth = CURRENT_DEPTH.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if depth >= MAX_DEPTH {
                CURRENT_DEPTH.store(0, std::sync::atomic::Ordering::SeqCst);
                return serde_json::Value::Object(default_obj);
            }

            for (key, schema_val) in schema_obj {
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
                        let is_valid = match schema_val {
                            serde_json::Value::Null => existing_val.is_null(),
                            serde_json::Value::Bool(_) => existing_val.is_boolean(),
                            serde_json::Value::Number(_) => existing_val.is_number(),
                            serde_json::Value::String(_) => existing_val.is_string(),
                            serde_json::Value::Array(_) => existing_val.is_array(),
                            serde_json::Value::Object(_) => existing_val.is_object(),
                        };

                        if is_valid {
                            result.insert(key.clone(), existing_val);
                        } else if let Some(default_val) = default_obj.get(key) {
                            result.insert(key.clone(), default_val.clone());
                        }
                    }
                } else if let Some(default_val) = default_obj.get(key) {
                    result.insert(key.clone(), default_val.clone());
                }
            }

            CURRENT_DEPTH.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            serde_json::Value::Object(result)
        }
        (_, default) => default,
    }
}
