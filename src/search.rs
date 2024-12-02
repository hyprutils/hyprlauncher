use crate::{
    config::{Config, SearchEngine},
    launcher::{self, AppEntry, EntryType, APP_CACHE},
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use tokio::sync::oneshot;

const BONUS_SCORE_LAUNCH_COUNT: i64 = 100;
const BONUS_SCORE_ICON_NAME: i64 = 1000;
const BONUS_SCORE_BINARY: i64 = 3000;
const BONUS_SCORE_FOLDER: i64 = 2000;
const BONUS_SCORE_KEYWORD_MATCH: i64 = 2500;
const BONUS_SCORE_CATEGORY_MATCH: i64 = 2000;
const BONUS_SCORE_WEB_SEARCH: i64 = -1000;

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

fn should_exclude_web_search(query: &str) -> bool {
    let excluded_terms = ["__config_reload__", "__refresh__"];
    excluded_terms.iter().any(|term| query == *term)
}

pub async fn search_applications(
    query: &str,
    config: &Config,
) -> Result<Vec<SearchResult>, std::io::Error> {
    let (tx, rx) = oneshot::channel();
    let query = query.to_lowercase();
    let max_results = config.window.max_entries;
    let web_search_config = config.web_search.clone();

    tokio::task::spawn_blocking(move || {
        let cache = APP_CACHE.blocking_read();
        let mut results = match query.chars().next() {
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
                        seen_names.insert(name_key.clone());

                        for action in &app.actions {
                            let mut action_app = app.clone();
                            action_app.name = format!("{} - {}", app.name, action.name);
                            action_app.exec = action.exec.clone();
                            if let Some(icon) = &action.icon_name {
                                action_app.icon_name = icon.clone();
                            }
                            results.push(SearchResult {
                                app: action_app,
                                score: BONUS_SCORE_BINARY + calculate_bonus_score(app) - 100,
                            });
                        }
                        continue;
                    }

                    if let Some(filename) = get_filename_without_extension(&app.path) {
                        if filename == query {
                            results.push(SearchResult {
                                app: app.clone(),
                                score: BONUS_SCORE_BINARY + calculate_bonus_score(app),
                            });
                            seen_names.insert(name_key.clone());
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
                        seen_names.insert(name_key.clone());
                        continue;
                    }

                    if app.categories.iter().any(|c| c.to_lowercase() == query) {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: BONUS_SCORE_CATEGORY_MATCH + calculate_bonus_score(app),
                        });
                        seen_names.insert(name_key.clone());
                        continue;
                    }

                    if let Some(score) = matcher.fuzzy_match(&name_lower, &query) {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: score + calculate_bonus_score(app),
                        });

                        for action in &app.actions {
                            let action_name =
                                format!("{} - {}", app.name, action.name).to_lowercase();
                            if let Some(action_score) = matcher.fuzzy_match(&action_name, &query) {
                                let mut action_app = app.clone();
                                action_app.name = format!("{} - {}", app.name, action.name);
                                action_app.exec = action.exec.clone();
                                if let Some(icon) = &action.icon_name {
                                    action_app.icon_name = icon.clone();
                                }
                                results.push(SearchResult {
                                    app: action_app,
                                    score: action_score + calculate_bonus_score(app) - 100,
                                });
                            }
                        }
                        seen_names.insert(name_key.clone());
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

                if results.is_empty()
                    && web_search_config.enabled
                    && !should_exclude_web_search(&query)
                {
                    results.push(create_web_search_entry(&query, &web_search_config.engine));
                }

                results.sort_unstable_by_key(|item| -item.score);
                if results.len() > max_results {
                    results.truncate(max_results);
                }
                results
            }
        };

        if web_search_config.enabled
            && !query.is_empty()
            && !should_exclude_web_search(&query)
            && !results
                .iter()
                .any(|r| r.app.categories.contains(&String::from("Web Search")))
        {
            results.push(create_web_search_entry(&query, &web_search_config.engine));
            results.sort_unstable_by_key(|item| -item.score);
            if results.len() > max_results {
                results.truncate(max_results);
            }
        }

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
                actions: Vec::new(),
            },
            score: BONUS_SCORE_BINARY,
        })
}

#[inline(always)]
fn handle_path_search(query: &str) -> Vec<SearchResult> {
    let config = Config::load();
    let expanded_path = shellexpand::full(query).unwrap_or(std::borrow::Cow::Borrowed(query));
    let path = std::path::Path::new(expanded_path.as_ref());

    let (dir, filter) = if path.is_dir() {
        (path.to_path_buf(), String::new())
    } else {
        (
            path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("/")),
            path.file_name()
                .map(|f| f.to_string_lossy().to_lowercase())
                .unwrap_or_default(),
        )
    };

    let matcher = SkimMatcherV2::default().smart_case();

    std::fs::read_dir(&dir)
        .ok()
        .map(|entries| {
            let mut results: Vec<SearchResult> = Vec::new();
            let mut parent_entry = None;

            if let Some(parent_dir) = dir.parent() {
                if let Some(mut app_entry) =
                    launcher::create_file_entry(parent_dir.to_string_lossy().into_owned())
                {
                    app_entry.name = String::from("..");
                    app_entry.score_boost = BONUS_SCORE_FOLDER;
                    parent_entry = Some(SearchResult {
                        app: app_entry,
                        score: i64::MAX,
                    });
                }
            }

            for entry in entries.filter_map(Result::ok) {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                let file_name_lower = file_name_str.to_lowercase();

                if !config.finder.show_hidden
                    && file_name_str.starts_with('.')
                    && file_name_str != ".."
                {
                    continue;
                }

                if !filter.is_empty() {
                    if let Some(score) = matcher.fuzzy_match(&file_name_lower, &filter) {
                        if let Some(app_entry) =
                            launcher::create_file_entry(entry.path().to_string_lossy().into_owned())
                        {
                            let base_score = if app_entry.icon_name == "folder" {
                                BONUS_SCORE_FOLDER
                            } else {
                                0
                            };
                            results.push(SearchResult {
                                app: app_entry,
                                score: score + base_score,
                            });
                        }
                    }
                } else if let Some(app_entry) =
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

            if let Some(parent) = parent_entry {
                results.insert(0, parent);
            }

            results
        })
        .unwrap_or_default()
}

pub async fn search_dmenu(
    query: String,
    lines: Vec<String>,
    config: Config,
) -> Result<Vec<String>, std::io::Error> {
    let (tx, rx) = oneshot::channel();
    let query = if config.dmenu.case_sensitive {
        query
    } else {
        query.to_lowercase()
    };
    let max_results = config.window.max_entries;

    tokio::task::spawn_blocking(move || {
        let matcher = SkimMatcherV2::default().smart_case();
        let mut results: Vec<(String, i64)> = lines
            .iter()
            .filter_map(|line| {
                let compare_line = if config.dmenu.case_sensitive {
                    line.clone()
                } else {
                    line.to_lowercase()
                };

                matcher
                    .fuzzy_match(&compare_line, &query)
                    .map(|score| (line.clone(), score))
            })
            .collect();

        if results.is_empty() && config.dmenu.allow_invalid {
            results.push((query, 0));
        }

        results.sort_unstable_by_key(|&(_, score)| -score);
        results.truncate(max_results);

        let results = results.into_iter().map(|(line, _)| line).collect();
        tx.send(results)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Failed to send results"))
    });

    rx.await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Failed to receive results"))
}

fn create_web_search_entry(query: &str, engine: &SearchEngine) -> SearchResult {
    SearchResult {
        app: AppEntry {
            name: format!("Search '{}' on the web", query),
            description: String::from("Open in default web browser"),
            path: String::new(),
            exec: format!(
                "xdg-open \"{}{}\"",
                engine.get_url(),
                utf8_percent_encode(query, NON_ALPHANUMERIC)
            ),
            icon_name: String::from("web-browser"),
            launch_count: 0,
            entry_type: EntryType::Application,
            score_boost: 0,
            keywords: Vec::new(),
            categories: vec![String::from("Web Search")],
            terminal: false,
            actions: Vec::new(),
        },
        score: BONUS_SCORE_WEB_SEARCH,
    }
}
