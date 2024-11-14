use once_cell::sync::Lazy;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tokio::sync::RwLock;

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

pub static DESKTOP_PATHS: &[&str] = &[
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

pub fn get_desktop_paths() -> Vec<String> {
    let mut paths = Vec::new();

    if let Ok(xdg_dirs) = std::env::var("XDG_DATA_DIRS") {
        paths.extend(
            xdg_dirs
                .split(':')
                .map(|dir| format!("{}/applications", dir)),
        );
    }

    paths.extend(DESKTOP_PATHS.iter().map(|&path| path.to_string()));

    paths
}

pub async fn load_applications() {
    let start_time = std::time::Instant::now();

    let heatmap_future = tokio::task::spawn_blocking(load_heatmap);

    let mut apps = HashMap::with_capacity(2000);
    let desktop_paths = get_desktop_paths();

    let desktop_entries: Vec<_> = desktop_paths
        .par_iter()
        .flat_map(|path| {
            let expanded_path = shellexpand::tilde(path).to_string();
            std::fs::read_dir(expanded_path)
                .map(|entries| {
                    entries
                        .par_bridge()
                        .filter_map(Result::ok)
                        .filter(|e| {
                            e.path().extension().and_then(|ext| ext.to_str()) == Some("desktop")
                        })
                        .filter_map(|entry| {
                            let path = entry.path();
                            let path_str = path.to_string_lossy();

                            if let Ok(contents) = std::fs::read_to_string(&path) {
                                let mut name = None;
                                let mut exec = None;
                                let mut icon = None;
                                let mut desc = None;

                                for line in contents.lines() {
                                    if let Some(stripped) = line.strip_prefix("Name=") {
                                        name = Some(stripped.to_string());
                                    } else if let Some(stripped) = line.strip_prefix("Exec=") {
                                        exec = Some(stripped.to_string());
                                    } else if let Some(stripped) = line.strip_prefix("Icon=") {
                                        icon = Some(stripped.to_string());
                                    } else if line.starts_with("Comment=")
                                        || line.starts_with("GenericName=")
                                    {
                                        desc = Some(
                                            line.split_once('=')
                                                .map(|x| x.1)
                                                .unwrap_or("")
                                                .to_string(),
                                        );
                                    }
                                }

                                name.map(|name| AppEntry {
                                    name: name.clone(),
                                    exec: exec.unwrap_or_default(),
                                    icon_name: icon
                                        .unwrap_or_else(|| "application-x-executable".to_string()),
                                    description: desc.unwrap_or_default(),
                                    path: path_str.into_owned(),
                                    launch_count: 0,
                                    entry_type: EntryType::Application,
                                })
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect();

    let heatmap = heatmap_future.await.unwrap_or_default();

    for mut entry in desktop_entries {
        if let Some(count) = heatmap.get(&entry.name) {
            entry.launch_count = *count;
        }
        apps.insert(entry.name.clone(), entry);
    }

    let mut cache = APP_CACHE.write().await;
    *cache = apps;

    println!(
        "Found {} total applications ({:.3}ms)",
        cache.len(),
        start_time.elapsed().as_secs_f64() * 1000.0
    );
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
