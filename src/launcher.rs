use crate::log;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, os::unix::fs::PermissionsExt, path::PathBuf};
use tokio::sync::RwLock;

pub static APP_CACHE: Lazy<RwLock<HashMap<String, AppEntry>>> =
    Lazy::new(|| RwLock::new(HashMap::with_capacity(2000)));

#[derive(Clone, Debug)]
pub struct AppEntry {
    pub name: String,
    pub description: String,
    pub path: String,
    pub exec: String,
    pub icon_name: String,
    pub launch_count: u32,
    pub entry_type: EntryType,
    pub score_boost: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum EntryType {
    Application,
    File,
}

static HEATMAP_PATH: &str = "~/.local/share/hyprlauncher/heatmap.json";

static DESKTOP_PATHS: &[&str] = &[
    "~/.local/share/applications",
    "/usr/share/applications",
    "/usr/local/share/applications",
    "/var/lib/flatpak/exports/share/applications",
    "~/.local/share/flatpak/exports/share/applications",
];

pub fn increment_launch_count(app: &AppEntry) {
    let app_name = app.name.clone();
    let count = app.launch_count + 1;

    std::thread::spawn(move || {
        save_heatmap(&app_name, count);
    });
}

#[inline]
fn save_heatmap(name: &str, count: u32) {
    let path = shellexpand::tilde(HEATMAP_PATH).to_string();

    if let Some(dir) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }

    let mut heatmap = load_heatmap();
    heatmap.insert(name.to_string(), count);

    if let Ok(contents) = serde_json::to_string(&heatmap) {
        let _ = fs::write(path, contents);
    }
}

#[inline]
fn load_heatmap() -> HashMap<String, u32> {
    let path = shellexpand::tilde(HEATMAP_PATH).to_string();
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_else(|| HashMap::with_capacity(100))
}

pub fn get_desktop_paths() -> Vec<PathBuf> {
    let mut paths = Vec::with_capacity(10);

    if let Ok(xdg_dirs) = std::env::var("XDG_DATA_DIRS") {
        paths.extend(
            xdg_dirs
                .split(':')
                .map(|dir| PathBuf::from(format!("{}/applications", dir))),
        );
    }

    paths.extend(
        DESKTOP_PATHS
            .iter()
            .map(|&path| PathBuf::from(shellexpand::tilde(path).to_string())),
    );

    paths
}

pub async fn load_applications() {
    log!("Starting application loading process");
    let heatmap_future = tokio::task::spawn_blocking(load_heatmap);

    let desktop_paths = get_desktop_paths();
    log!("Scanning desktop entry paths: {:?}", desktop_paths);
    let mut apps = HashMap::with_capacity(2000);

    let entries: Vec<_> = desktop_paths
        .par_iter()
        .flat_map_iter(|path| {
            if let Ok(entries) = std::fs::read_dir(path) {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| {
                        matches!(
                            e.path().extension().and_then(|e| e.to_str()),
                            Some("desktop")
                        )
                    })
                    .filter_map(|entry| parse_desktop_entry(&entry.path()))
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        })
        .collect();

    let heatmap = heatmap_future.await.unwrap_or_default();

    for mut entry in entries {
        if let Some(count) = heatmap.get(&entry.name) {
            entry.launch_count = *count;
        }
        apps.insert(entry.name.clone(), entry);
    }

    log!("Loaded {} total applications", apps.len());
    let mut cache = APP_CACHE.write().await;
    *cache = apps;
}

#[inline]
fn parse_desktop_entry(path: &std::path::Path) -> Option<AppEntry> {
    let entry = freedesktop_entry_parser::parse_entry(path).ok()?;
    let section = entry.section("Desktop Entry");

    if section.attr("NoDisplay").map_or(false, |v| v == "true") {
        return None;
    }

    let name = section.attr("Name")?;
    let exec = section.attr("Exec").unwrap_or_default();
    let icon = section.attr("Icon").unwrap_or("application-x-executable");
    let desc = section
        .attr("Comment")
        .or_else(|| section.attr("GenericName"))
        .unwrap_or("");

    Some(AppEntry {
        name: name.to_string(),
        exec: exec.to_string(),
        icon_name: icon.to_string(),
        description: desc.to_string(),
        path: path.to_string_lossy().into_owned(),
        launch_count: 0,
        entry_type: EntryType::Application,
        score_boost: 0,
    })
}

pub fn create_file_entry(path: String) -> Option<AppEntry> {
    let path = if path.starts_with('~') || path.starts_with('$') {
        shellexpand::full(&path).ok()?.to_string()
    } else {
        path
    };

    let metadata = std::fs::metadata(&path).ok()?;

    if !metadata.is_file() && !metadata.is_dir() {
        return None;
    }

    let name = std::path::Path::new(&path)
        .file_name()?
        .to_str()?
        .to_string();

    let (icon_name, exec, score_boost) = if metadata.is_dir() {
        ("folder", String::new(), 2000)
    } else if metadata.permissions().mode() & 0o111 != 0 {
        ("application-x-executable", format!("\"{}\"", path), 0)
    } else {
        let (icon, exec) = get_mime_type_info(&path);
        (icon, exec, 0)
    };

    Some(AppEntry {
        name,
        exec: exec.to_string(),
        icon_name: icon_name.to_string(),
        description: String::new(),
        path,
        launch_count: 0,
        entry_type: EntryType::File,
        score_boost,
    })
}

#[inline]
fn get_mime_type_info(path: &str) -> (&'static str, String) {
    let output = std::process::Command::new("file")
        .arg("--mime-type")
        .arg(path)
        .output()
        .ok();

    let mime_type = output
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let icon = if mime_type.contains("text/") {
        "text-x-generic"
    } else {
        match std::path::Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
        {
            Some("pdf") => "application-pdf",
            _ => "application-x-generic",
        }
    };

    (icon, format!("xdg-open \"{}\"", path))
}
