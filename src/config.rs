use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub width: i32,
    pub height: i32,
    pub show_descriptions: bool,
    pub show_paths: bool,
    pub show_icons: bool,
    pub vim_keys: bool,
    pub show_search: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 600,
            height: 600,
            show_descriptions: false,
            show_paths: true,
            show_icons: true,
            vim_keys: true,
            show_search: true,
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
