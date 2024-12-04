use crate::log;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
        LazyLock, Mutex,
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

static CURRENT_CONFIG_ERROR: Lazy<Mutex<Option<ConfigError>>> = Lazy::new(|| Mutex::new(None));

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
    pub show_actions: bool,
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
            show_actions: false,
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
pub struct Dmenu {
    pub allow_invalid: bool,
    pub case_sensitive: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum SearchEngine {
    Preset(PresetEngine),
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PresetEngine {
    DuckDuckGo,
    Google,
    Bing,
    Brave,
    Ecosia,
    Startpage,
}

impl SearchEngine {
    pub fn get_url(&self) -> String {
        match self {
            SearchEngine::Preset(engine) => match engine {
                PresetEngine::DuckDuckGo => String::from("https://duckduckgo.com/?q="),
                PresetEngine::Google => String::from("https://www.google.com/search?q="),
                PresetEngine::Bing => String::from("https://www.bing.com/search?q="),
                PresetEngine::Brave => String::from("https://search.brave.com/search?q="),
                PresetEngine::Ecosia => String::from("https://www.ecosia.org/search?q="),
                PresetEngine::Startpage => String::from("https://www.startpage.com/do/search?q="),
            },
            SearchEngine::Custom(url) => url.clone(),
        }
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::Preset(PresetEngine::DuckDuckGo)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SearchPrefix {
    pub prefix: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct WebSearch {
    pub enabled: bool,
    pub engine: SearchEngine,
    pub prefixes: Vec<SearchPrefix>,
}

#[derive(Debug, Clone)]
pub struct ConfigError {
    pub line: usize,
    pub message: String,
    pub suggestion: String,
}

impl ConfigError {
    pub fn new(line: usize, message: &str, suggestion: &str) -> Self {
        Self {
            line,
            message: message.to_string(),
            suggestion: suggestion.to_string(),
        }
    }
}

impl Config {
    fn config_dir() -> &'static PathBuf {
        &CONFIG_DIR
    }

    pub fn load() -> Self {
        let config_file = Self::config_dir().join("config.toml");
        log!("Loading configuration from: {:?}", config_file);

        if !config_file.exists() {
            log!("Config file not found, creating default configuration");
            let default_config = Config::default();
            if let Ok(contents) = toml::to_string_pretty(&default_config) {
                fs::write(&config_file, contents).unwrap_or_default();
            }
            *CURRENT_CONFIG_ERROR.lock().unwrap() = None;
            return default_config;
        }

        match fs::read_to_string(&config_file) {
            Ok(contents) => {
                let required_categories = ["window", "theme", "debug", "dmenu", "web_search"];
                let doc = match contents.parse::<toml::Table>() {
                    Ok(doc) => doc,
                    Err(_) => {
                        let error = ConfigError::new(
                            1,
                            "Failed to parse config file",
                            "Verify the TOML syntax is correct",
                        );
                        *CURRENT_CONFIG_ERROR.lock().unwrap() = Some(error);
                        let mut default_config = Config::default();
                        default_config.debug.disable_auto_focus = true;
                        return default_config;
                    }
                };

                for category in required_categories {
                    if !doc.contains_key(category) {
                        let error = ConfigError::new(
                            1,
                            &format!("Missing required category '[{}]'", category),
                            "Add the missing category with its required fields",
                        );
                        *CURRENT_CONFIG_ERROR.lock().unwrap() = Some(error);
                        let mut default_config = Config::default();
                        default_config.debug.disable_auto_focus = true;
                        return default_config;
                    }
                }

                match toml::from_str::<Config>(&contents) {
                    Ok(config) => {
                        LOGGING_ENABLED.store(config.debug.enable_logging, Ordering::SeqCst);
                        *CURRENT_CONFIG_ERROR.lock().unwrap() = None;
                        config
                    }
                    Err(e) => {
                        let line = e.span().map(|s| s.start).unwrap_or(0);
                        let suggestion = match e.to_string() {
                            s if s.contains("invalid type") => {
                                "Check the type of this value matches what's expected in the config"
                            }
                            s if s.contains("missing field") => {
                                "Add the missing field with an appropriate value"
                            }
                            _ => "Verify the syntax follows TOML format",
                        };
                        let error = ConfigError::new(line, &e.to_string(), suggestion);
                        *CURRENT_CONFIG_ERROR.lock().unwrap() = Some(error);
                        let mut default_config = Config::default();
                        default_config.debug.disable_auto_focus = true;
                        default_config
                    }
                }
            }
            Err(e) => {
                log!("Error reading config file: {}", e);
                *CURRENT_CONFIG_ERROR.lock().unwrap() = None;
                Config::default()
            }
        }
    }

    pub fn get_current_error() -> Option<ConfigError> {
        CURRENT_CONFIG_ERROR.lock().unwrap().clone()
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
                scrollbar {{ opacity: 0; }}
                .error-overlay {{
                    background-color: rgba(200, 0, 0, 0.95);
                    padding: 12px;
                    margin: 8px;
                    border-radius: 6px;
                }}
                .error-message {{
                    color: white;
                    font-weight: bold;
                    font-size: 14px;
                }}
                .error-suggestion {{
                    color: rgba(255, 255, 255, 0.9);
                    font-size: 14px;
                    font-weight: bold;
                }}",
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
                scrollbar {{ opacity: 0; }}
                .error-overlay {{
                    background-color: rgba(200, 0, 0, 0.95);
                    padding: 12px;
                    margin: 8px;
                    border-radius: 6px;
                }}
                .error-message {{
                    color: white;
                    font-weight: bold;
                    font-size: 14px;
                }}
                .error-suggestion {{
                    color: rgba(255, 255, 255, 0.9);
                    font-size: 14px;
                    font-weight: bold;
                }}",
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
        let config_path = Self::config_dir().join("config.toml");
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
                                        last_content = Some(new_content.clone());
                                        match toml::from_str::<Config>(&new_content) {
                                            Ok(_) => {
                                                *CURRENT_CONFIG_ERROR.lock().unwrap() = None;
                                                callback();
                                                true
                                            }
                                            Err(e) => {
                                                let line = e.span().map(|s| s.start).unwrap_or(0);
                                                let error = ConfigError::new(
                                                    line,
                                                    &e.to_string(),
                                                    "Check your config syntax",
                                                );
                                                *CURRENT_CONFIG_ERROR.lock().unwrap() = Some(error);
                                                callback();
                                                true
                                            }
                                        }
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
