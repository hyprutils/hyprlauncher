use crate::{
    config::Config,
    launcher::{self, AppEntry, EntryType, APP_CACHE},
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use tokio::sync::oneshot;

const BONUS_SCORE_LAUNCH_COUNT: i64 = 100;
const BONUS_SCORE_ICON_NAME: i64 = 1000;
const BONUS_SCORE_BINARY: i64 = 3000;
const BONUS_SCORE_FOLDER: i64 = 2000;
const BONUS_SCORE_KEYWORD_MATCH: i64 = 2500;
const BONUS_SCORE_CATEGORY_MATCH: i64 = 2000;

pub struct SearchResult {
    pub app: AppEntry,
    pub score: i64,
}

fn get_filename_without_extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
}

pub async fn search_applications(
    query: &str,
    config: &Config,
) -> Result<Vec<SearchResult>, std::io::Error> {
    let (tx, rx) = oneshot::channel();
    let query = query.to_lowercase();
    let max_results = config.window.max_entries;

    tokio::task::spawn_blocking(move || {
        let cache = APP_CACHE.blocking_read();

        let results = match query.chars().next() {
            Some('~' | '$' | '/') => handle_path_search(&query),

            None => {
                let mut results = Vec::with_capacity(max_results);
                for app in cache.values() {
                    if app.path.ends_with(".desktop") {
                        results.push(SearchResult {
                            score: calculate_bonus_score(app),
                            app: app.clone(),
                        });

                        if results.len() >= max_results {
                            break;
                        }
                    }
                }
                results.sort_unstable_by_key(|item| -item.score);
                results
            }

            Some(_) => {
                let matcher = SkimMatcherV2::default().smart_case();
                let mut results = Vec::with_capacity(max_results);
                let mut seen_names = std::collections::HashSet::new();

                for app in cache.values() {
                    let name_lower = app.name.to_lowercase();
                    let name_key = name_lower.clone();

                    if name_lower == query {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: BONUS_SCORE_BINARY + calculate_bonus_score(app),
                        });
                        seen_names.insert(name_key);
                        continue;
                    }

                    if let Some(filename) = get_filename_without_extension(&app.path) {
                        if filename == query {
                            results.push(SearchResult {
                                app: app.clone(),
                                score: BONUS_SCORE_BINARY + calculate_bonus_score(app),
                            });
                            seen_names.insert(name_key);
                            continue;
                        }

                        if let Some(score) = matcher.fuzzy_match(&filename, &query) {
                            results.push(SearchResult {
                                app: app.clone(),
                                score: score + calculate_bonus_score(app),
                            });
                            seen_names.insert(name_key.clone());
                            continue;
                        }
                    }

                    if app.keywords.iter().any(|k| k.to_lowercase() == query) {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: BONUS_SCORE_KEYWORD_MATCH + calculate_bonus_score(app),
                        });
                        seen_names.insert(name_key);
                        continue;
                    }

                    if app.categories.iter().any(|c| c.to_lowercase() == query) {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: BONUS_SCORE_CATEGORY_MATCH + calculate_bonus_score(app),
                        });
                        seen_names.insert(name_key);
                        continue;
                    }

                    if let Some(score) = matcher.fuzzy_match(&name_lower, &query) {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: score + calculate_bonus_score(app),
                        });
                        seen_names.insert(name_key);
                        continue;
                    }

                    for keyword in &app.keywords {
                        if let Some(score) = matcher.fuzzy_match(&keyword.to_lowercase(), &query) {
                            results.push(SearchResult {
                                app: app.clone(),
                                score: score + calculate_bonus_score(app),
                            });
                            seen_names.insert(name_key.clone());
                            break;
                        }
                    }

                    for category in &app.categories {
                        if let Some(score) = matcher.fuzzy_match(&category.to_lowercase(), &query) {
                            results.push(SearchResult {
                                app: app.clone(),
                                score: score + calculate_bonus_score(app),
                            });
                            seen_names.insert(name_key.clone());
                            break;
                        }
                    }
                }

                if !seen_names.contains(&query) {
                    if let Some(result) = check_binary(&query) {
                        results.push(result);
                    }
                }

                results.sort_unstable_by_key(|item| -item.score);
                if results.len() > max_results {
                    results.truncate(max_results);
                }
                results
            }
        };

        tx.send(results)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Failed to send results"))
    });

    rx.await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Failed to receive results"))
}

#[inline(always)]
fn calculate_bonus_score(app: &AppEntry) -> i64 {
    (app.launch_count as i64 * BONUS_SCORE_LAUNCH_COUNT)
        + if app.icon_name == "application-x-executable" {
            0
        } else {
            BONUS_SCORE_ICON_NAME
        }
}

#[inline(always)]
fn check_binary(query: &str) -> Option<SearchResult> {
    let parts: Vec<&str> = query.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let bin_path = format!("/usr/bin/{}", parts[0]);
    std::fs::metadata(&bin_path)
        .ok()
        .filter(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .map(|_| SearchResult {
            app: AppEntry {
                name: query.to_string(),
                description: String::new(),
                path: bin_path.clone(),
                exec: if parts.len() > 1 {
                    format!("{} {}", bin_path, parts[1..].join(" "))
                } else {
                    bin_path
                },
                icon_name: String::from("application-x-executable"),
                launch_count: 0,
                entry_type: EntryType::File,
                score_boost: BONUS_SCORE_BINARY,
                keywords: Vec::new(),
                categories: Vec::new(),
                terminal: false,
            },
            score: BONUS_SCORE_BINARY,
        })
}

#[inline(always)]
fn handle_path_search(query: &str) -> Vec<SearchResult> {
    let config = Config::load();
    let expanded_path = shellexpand::full(query).unwrap_or(std::borrow::Cow::Borrowed(query));
    let path = std::path::Path::new(expanded_path.as_ref());

    let dir = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/"))
    };

    std::fs::read_dir(&dir)
        .ok()
        .map(|entries| {
            let mut results: Vec<SearchResult> = Vec::new();

            if let Some(parent_dir) = dir.parent() {
                if let Some(mut app_entry) =
                    launcher::create_file_entry(parent_dir.to_string_lossy().into_owned())
                {
                    app_entry.name = String::from("..");
                    app_entry.score_boost = BONUS_SCORE_FOLDER;
                    results.push(SearchResult {
                        app: app_entry,
                        score: BONUS_SCORE_FOLDER,
                    });
                }
            }

            for entry in entries.filter_map(Result::ok) {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                if !config.finder.show_hidden
                    && file_name_str.starts_with('.')
                    && file_name_str != ".."
                {
                    continue;
                }

                if let Some(app_entry) =
                    launcher::create_file_entry(entry.path().to_string_lossy().into_owned())
                {
                    let score = if app_entry.icon_name == "folder" {
                        BONUS_SCORE_FOLDER
                    } else {
                        0
                    };
                    results.push(SearchResult {
                        app: app_entry,
                        score,
                    });
                }
            }

            results.sort_unstable_by_key(|item| (-item.score, item.app.name.clone()));
            results
        })
        .unwrap_or_default()
}
