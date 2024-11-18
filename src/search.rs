use crate::{
    config::Config,
    launcher::{self, AppEntry, EntryType, APP_CACHE},
};
use evalexpr::eval;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use tokio::sync::oneshot;

pub struct SearchResult {
    pub app: AppEntry,
    pub score: i64,
}

pub async fn search_applications(query: &str, config: &Config) -> Vec<SearchResult> {
    let (tx, rx) = oneshot::channel();
    let query = query.to_lowercase();
    let max_results = config.window.max_entries;

    tokio::task::spawn_blocking(move || {
        let cache = APP_CACHE.blocking_read();

        let results = match query.chars().next() {
            Some('~' | '$' | '/') => handle_path_search(&query),
            Some('=') => handle_calculation(&query),

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
                    if name_lower == query {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: 10000 + calculate_bonus_score(app),
                        });
                        seen_names.insert(name_lower);
                        continue;
                    }

                    if let Some(score) = matcher.fuzzy_match(&name_lower, &query) {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: score + calculate_bonus_score(app),
                        });
                        seen_names.insert(name_lower);
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

        let _ = tx.send(results);
    });

    rx.await.unwrap_or_default()
}

#[inline(always)]
fn calculate_bonus_score(app: &AppEntry) -> i64 {
    (app.launch_count as i64 * 100)
        + if app.icon_name == "application-x-executable" {
            0
        } else {
            1000
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
                score_boost: 3000,
            },
            score: 3000,
        })
}

#[inline(always)]
fn handle_path_search(query: &str) -> Vec<SearchResult> {
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
                    app_entry.score_boost = 3000;
                    results.push(SearchResult {
                        app: app_entry,
                        score: 3000,
                    });
                }
            }

            let mut entries: Vec<_> = entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let path = entry.path().to_string_lossy().into_owned();
                    launcher::create_file_entry(path).map(|mut app| {
                        let score = if app.icon_name == "folder" {
                            2000
                        } else {
                            1000
                        };
                        app.score_boost = score;
                        SearchResult { app, score }
                    })
                })
                .collect();

            entries.sort_by(|a, b| {
                let a_is_folder = a.app.icon_name == "folder";
                let b_is_folder = b.app.icon_name == "folder";

                match (a_is_folder, b_is_folder) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.app.name.to_lowercase().cmp(&b.app.name.to_lowercase()),
                }
            });

            results.extend(entries);
            results
        })
        .unwrap_or_default()
}

#[inline(always)]
fn handle_calculation(query: &str) -> Vec<SearchResult> {
    println!("query is {}", query);
    let query = &query[1..];
    let res = match query {
        "" => evalexpr::Value::from_int(0),
        _ => match eval(query) {
            Ok(res) => res,
            Err(e) => {
                println!("Error: {}", e);
                evalexpr::Value::from_int(0)
            }
        },
    };

    let entry = launcher::create_calc_entry(res.to_string()).unwrap();

    let enteries: Vec<_> = vec![SearchResult {
        app: entry,
        score: 5000,
    }];

    enteries
}
