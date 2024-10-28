use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub width: i32,
    pub height: i32,
    pub font_size: i32,
    pub max_results: usize,
    pub theme: Theme,
    pub search_tooltip: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Theme {
    pub background_color: String,
    pub text_color: String,
    pub selection_color: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 600,
            height: 600,
            font_size: 14,
            max_results: 10,
            theme: Theme {
                background_color: String::from("#0f0f0f"),
                text_color: String::from("#eceff4"),
                selection_color: String::from("#5e81ac"),
            },
            search_tooltip: false,
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
            fs::write(&css_path, get_default_css()).unwrap_or_default();
        }

        config_path
    }

    pub fn load() -> Self {
        let config_path = Self::ensure_config_dir();
        let config_file = config_path.join("config.json");

        if !config_file.exists() {
            let config = Config::default();
            if let Ok(contents) = serde_json::to_string_pretty(&config) {
                fs::write(&config_file, contents).unwrap_or_default();
            }
        }

        fs::read_to_string(config_file)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default()
    }

    pub fn load_css() -> String {
        let config_path = Self::ensure_config_dir();
        let css_path = config_path.join("style.css");
        fs::read_to_string(css_path).unwrap_or_else(|_| get_default_css())
    }
}

fn get_default_css() -> String {
    String::from(
        "window { 
            background-color: #0f0f0f;
        }
        
        list { 
            background: #0f0f0f;
        }
        
        list row { 
            padding: 4px;
            margin: 2px 6px;
            border-radius: 8px;
            background: #0f0f0f;
            transition: all 200ms ease;
        }
        
        list row:selected { 
            background-color: #1f1f1f;
        }
        
        list row:hover:not(:selected) {
            background-color: #181818;
        }
        
        entry {
            margin: 12px;
            margin-bottom: 8px;
            padding: 12px;
            border-radius: 8px;
            background-color: #1f1f1f;
            color: #e0e0e0;
            caret-color: #808080;
            font-size: 16px;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
        }
        
        entry:focus {
            background-color: #282828;
        }
        
        .app-name {
            color: #ffffff;
            font-size: 14px;
            font-weight: bold;
            margin-right: 8px;
        }
        
        .app-description {
            color: #a0a0a0;
            font-size: 12px;
            margin-right: 8px;
        }
        
        .app-path {
            color: #808080;
            font-size: 12px;
            font-family: monospace;
            opacity: 0.8;
        }",
    )
}
