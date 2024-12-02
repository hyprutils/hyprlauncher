use crate::log;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;

pub static APP_CACHE: Lazy<RwLock<HashMap<String, AppEntry>>> =
    Lazy::new(|| RwLock::new(HashMap::with_capacity(2000)));

#[derive(Clone, Debug)]
pub struct DesktopAction {
    pub name: String,
    pub exec: String,
    pub icon_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AppEntry {
    pub name: String,
    pub description: String,
    pub path: String,
    pub exec: String,
    pub icon_name: String,
    pub launch_count: u32,
    pub last_used: Option<u64>,
    pub entry_type: EntryType,
    pub score_boost: i64,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub terminal: bool,
    pub actions: Vec<DesktopAction>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum EntryType {
    Application,
    File,
}

static HEATMAP_PATH: &str = "~/.local/share/hyprlauncher/heatmap.json";

static DESKTOP_PATHS: &[&str] = &[
    "/usr/share/applications",
    "/usr/local/share/applications",
    "/var/lib/flatpak/exports/share/applications",
    "~/.local/share/applications",
    "~/.local/share/flatpak/exports/share/applications",
];

const DEFAULT_SCORE_BOOST: i64 = 2000;

#[derive(Serialize, Deserialize)]
pub struct HeatmapEntry {
    pub count: u32,
    pub last_used: u64,
}

pub fn increment_launch_count(app: &AppEntry) -> Result<u32, std::io::Error> {
    let app_name = app.name.clone();
    let count = app.launch_count + 1;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    std::thread::spawn(move || {
        let mut cache = APP_CACHE.blocking_write();
        if let Some(cached_app) = cache.get_mut(&app_name) {
            cached_app.launch_count = count;
            cached_app.last_used = Some(now);
        }
        save_heatmap(&app_name, count).unwrap();
    });

    Ok(count)
}

#[inline]
fn save_heatmap(name: &str, count: u32) -> Result<(), std::io::Error> {
    let path = shellexpand::tilde(HEATMAP_PATH).to_string();

    if let Some(dir) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }

    let mut heatmap: HashMap<String, HeatmapEntry> = load_heatmap()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    heatmap.insert(
        name.to_string(),
        HeatmapEntry {
            count,
            last_used: now,
        },
    );

    if let Ok(contents) = serde_json::to_string(&heatmap) {
        let _ = fs::write(path, contents);
    }

    Ok(())
}

pub fn load_heatmap() -> Result<HashMap<String, HeatmapEntry>, std::io::Error> {
    let path = shellexpand::tilde(HEATMAP_PATH).to_string();
    Ok(fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_else(|| HashMap::with_capacity(100)))
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

pub async fn load_applications() -> Result<(), std::io::Error> {
    log!("Starting application loading process");
    let heatmap = load_heatmap()?;
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

    for mut entry in entries {
        if let Some(heat_entry) = heatmap.get(&entry.name) {
            entry.launch_count = heat_entry.count;
            entry.last_used = Some(heat_entry.last_used);
        }
        apps.insert(entry.name.clone(), entry);
    }

    log!("Loaded {} total applications", apps.len());
    let mut cache = APP_CACHE.write().await;
    *cache = apps;

    Ok(())
}

#[inline]
fn parse_desktop_entry(path: &std::path::Path) -> Option<AppEntry> {
    let entry = freedesktop_entry_parser::parse_entry(path).ok()?;
    let section = entry.section("Desktop Entry");

    if section.attr("NoDisplay").map_or(false, |v| v == "true") {
        return None;
    }

    let current_desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_uppercase();
    let desktops: Vec<String> = current_desktop
        .split(':')
        .map(|s| s.to_uppercase())
        .collect();

    if let Some(only_show_in) = section.attr("OnlyShowIn") {
        let allowed_desktops: Vec<String> = only_show_in
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_uppercase())
            .collect();
        if !desktops.iter().any(|d| allowed_desktops.contains(d)) {
            return None;
        }
    }

    if let Some(not_show_in) = section.attr("NotShowIn") {
        let excluded_desktops: Vec<String> = not_show_in
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_uppercase())
            .collect();
        if desktops.iter().any(|d| excluded_desktops.contains(d)) {
            return None;
        }
    }

    let name = String::from(section.attr("Name")?);
    let raw_exec = String::from(section.attr("Exec").unwrap_or_default());

    let exec = raw_exec
        .split_whitespace()
        .filter(|&arg| !arg.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");

    let icon = String::from(section.attr("Icon").unwrap_or("application-x-executable"));
    let desc = String::from(
        section
            .attr("Comment")
            .or_else(|| section.attr("GenericName"))
            .unwrap_or(""),
    );

    let keywords = section
        .attr("Keywords")
        .map(|k| {
            k.split(';')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let categories = section
        .attr("Categories")
        .map(|c| {
            c.split(';')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let terminal = section.attr("Terminal").map_or(false, |v| v == "true");

    let mut actions = Vec::new();
    if let Some(action_list) = section.attr("Actions") {
        for action_name in action_list.split(';').filter(|s| !s.is_empty()) {
            let section_name = format!("Desktop Action {}", action_name);
            let action_section = entry.section(&section_name);
            if let Some(raw_action_exec) = action_section.attr("Exec") {
                let action_exec = raw_action_exec
                    .split_whitespace()
                    .filter(|&arg| !arg.starts_with('%'))
                    .collect::<Vec<_>>()
                    .join(" ");

                let action = DesktopAction {
                    name: action_section
                        .attr("Name")
                        .unwrap_or(action_name)
                        .to_string(),
                    exec: action_exec,
                    icon_name: action_section.attr("Icon").map(String::from),
                };
                actions.push(action);
            }
        }
    }

    Some(AppEntry {
        name,
        exec,
        icon_name: icon,
        description: desc,
        path: path.to_string_lossy().into_owned(),
        launch_count: 0,
        last_used: None,
        entry_type: EntryType::Application,
        score_boost: 0,
        keywords,
        categories,
        terminal,
        actions,
    })
}

pub fn create_file_entry(path: String) -> Option<AppEntry> {
    let path = if path.starts_with('~') || path.starts_with('$') {
        shellexpand::full(&path).ok()?.to_string()
    } else {
        path
    };

    let metadata = std::fs::metadata(&path).ok()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if !metadata.is_file() && !metadata.is_dir() {
        return None;
    }

    let name = std::path::Path::new(&path)
        .file_name()?
        .to_str()?
        .to_string();

    let (icon_name, exec, score_boost) = if metadata.is_dir() {
        ("folder", String::new(), DEFAULT_SCORE_BOOST)
    } else if metadata.permissions().mode() & 0o111 != 0 {
        ("application-x-executable", format!("\"{}\"", path), 0)
    } else {
        let (icon, exec) = get_mime_type_info(&path);
        (icon, exec, 0)
    };

    Some(AppEntry {
        name,
        exec,
        icon_name: icon_name.to_string(),
        description: String::new(),
        path,
        launch_count: 0,
        last_used: Some(now),
        entry_type: EntryType::File,
        score_boost,
        keywords: Vec::new(),
        categories: Vec::new(),
        terminal: false,
        actions: Vec::new(),
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
