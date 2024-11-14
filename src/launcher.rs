use freedesktop_entry_parser::parse_entry;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tokio::sync::RwLock;
use walkdir::WalkDir;

pub static APP_CACHE: Lazy<RwLock<HashMap<String, AppEntry>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon_name: String,
    pub path: String,
    pub description: String,
    pub launch_count: u32,
    pub entry_type: EntryType,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum EntryType {
    Application,
    File,
}

pub static HEATMAP_PATH: &str = "~/.local/share/hyprlauncher/heatmap.json";

pub fn increment_launch_count(app: &AppEntry) {
    println!("Launching application: {}", app.name);
    let app_name = app.name.clone();
    let count = app.launch_count + 1;

    std::thread::spawn(move || {
        save_heatmap(&app_name, count);
    });
}

fn save_heatmap(name: &str, count: u32) {
    let path = shellexpand::tilde(HEATMAP_PATH).to_string();

    if let Some(dir) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(dir).unwrap_or_default();
    }

    let mut heatmap = load_heatmap();
    heatmap.insert(name.to_string(), count);

    if let Ok(contents) = serde_json::to_string(&heatmap) {
        fs::write(path, contents).unwrap_or_default();
    }
}

fn load_heatmap() -> HashMap<String, u32> {
    let path = shellexpand::tilde(HEATMAP_PATH).to_string();
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

pub async fn load_applications() {
    println!("Loading application heatmap...");
    let heatmap = tokio::task::spawn_blocking(load_heatmap)
        .await
        .unwrap_or_default();

    let mut apps = HashMap::new();
    println!("Scanning desktop entries...");
    let desktop_paths = std::env::var("XDG_DATA_DIRS")
        .map(|str| {
            str.split(':')
                .map(|str| format!("{str}/applications"))
                .collect::<Vec<_>>()
        })
        .unwrap_or(vec![
            String::from("/usr/share/applications"),
            String::from("/usr/local/share/applications"),
            String::from("~/.local/share/applications"),
        ]);

    for path in desktop_paths {
        let expanded_path = shellexpand::tilde(&path).to_string();
        if let Ok(entries) = std::fs::read_dir(expanded_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".desktop") {
                        if let Ok(desktop_entry) = parse_entry(entry.path()) {
                            if let Some(app_name) =
                                desktop_entry.section("Desktop Entry").attr("Name")
                            {
                                let exec = desktop_entry
                                    .section("Desktop Entry")
                                    .attr("Exec")
                                    .unwrap_or("")
                                    .to_string();
                                let icon = desktop_entry
                                    .section("Desktop Entry")
                                    .attr("Icon")
                                    .unwrap_or("application-x-executable")
                                    .to_string();
                                let description = desktop_entry
                                    .section("Desktop Entry")
                                    .attr("Comment")
                                    .or_else(|| {
                                        desktop_entry.section("Desktop Entry").attr("GenericName")
                                    })
                                    .unwrap_or("")
                                    .to_string();
                                let launch_count =
                                    heatmap.get(app_name).copied().unwrap_or_default();

                                apps.insert(
                                    app_name.to_string(),
                                    AppEntry {
                                        name: app_name.to_string(),
                                        exec,
                                        icon_name: icon,
                                        description,
                                        path: entry.path().to_string_lossy().to_string(),
                                        launch_count,
                                        entry_type: EntryType::Application,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    println!("Found {} desktop entries", apps.len());
    println!("Scanning PATH for executables...");

    let path = std::env::var("PATH").unwrap_or_default();
    let path_entries: Vec<_> = path.split(':').collect();

    let results: Vec<_> = path_entries
        .par_iter()
        .flat_map(|path_entry| {
            WalkDir::new(path_entry)
                .follow_links(true)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_type().is_file()
                        && e.metadata()
                            .map(|m| m.permissions().mode() & 0o111 != 0)
                            .unwrap_or(false)
                })
                .filter_map(|entry| {
                    entry.file_name().to_str().map(|name| {
                        let name = name.to_string();
                        let path = entry.path().to_string_lossy().to_string();
                        let launch_count = heatmap.get(&name).copied().unwrap_or_default();

                        let icon_name = find_desktop_entry(&name)
                            .map(|e| e.icon_name)
                            .unwrap_or_default();

                        (
                            name.clone(),
                            AppEntry {
                                name,
                                exec: path.clone(),
                                icon_name,
                                description: String::new(),
                                path,
                                launch_count,
                                entry_type: EntryType::Application,
                            },
                        )
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();

    for (name, entry) in results {
        apps.insert(name, entry);
    }

    println!("Found {} total applications", apps.len());
    let mut cache = APP_CACHE.write().await;
    *cache = apps;
}

struct DesktopEntry {
    icon_name: String,
}

fn find_desktop_entry(name: &str) -> Option<DesktopEntry> {
    let paths = [
        "/usr/share/applications",
        "/usr/local/share/applications",
        "~/.local/share/applications",
    ];

    for path in paths {
        let desktop_file = format!("{}/{}.desktop", path, name);
        if let Ok(entry) = parse_entry(&desktop_file) {
            if let Some(icon) = entry.section("Desktop Entry").attr("Icon") {
                if !icon.is_empty() {
                    return Some(DesktopEntry {
                        icon_name: icon.to_string(),
                    });
                }
            }
        }
    }
    None
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

    let (icon_name, exec) = if metadata.is_dir() {
        ("folder", String::new())
    } else if metadata.permissions().mode() & 0o111 != 0 {
        ("application-x-executable", format!("\"{}\"", path))
    } else {
        let mime_type = match std::process::Command::new("file")
            .arg("--mime-type")
            .arg("-b")
            .arg(&path)
            .output()
        {
            Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
            Err(_) => String::from("application/octet-stream"),
        };

        let icon = match mime_type.split('/').next().unwrap_or("") {
            "text" => "text-x-generic",
            "image" => "image-x-generic",
            "audio" => "audio-x-generic",
            "video" => "video-x-generic",
            "application" => match std::path::Path::new(&path)
                .extension()
                .and_then(|s| s.to_str())
            {
                Some("pdf") => "application-pdf",
                _ => "application-x-generic",
            },
            _ => "text-x-generic",
        };

        (icon, format!("xdg-mime query default {} | xargs -I {{}} sh -c 'which {{}} >/dev/null && {{}} \"{}\" || xdg-open \"{}\"'", mime_type, path, path))
    };

    Some(AppEntry {
        name,
        exec,
        icon_name: icon_name.to_string(),
        description: String::new(),
        path,
        launch_count: 0,
        entry_type: EntryType::File,
    })
}
