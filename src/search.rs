use crate::launcher::{self, AppEntry, APP_CACHE};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rayon::prelude::*;
use tokio::sync::oneshot;

pub struct SearchResult {
    pub app: AppEntry,
    pub score: i64,
}

pub async fn search_applications(query: &str) -> Vec<SearchResult> {
    let (tx, rx) = oneshot::channel();
    let query = query.to_lowercase();

    tokio::task::spawn_blocking(move || {
        let cache = APP_CACHE.blocking_read();
        let results = if query.starts_with('~') || query.starts_with('$') || query.starts_with('/')
        {
            let expanded_path =
                shellexpand::full(&query).unwrap_or(std::borrow::Cow::Borrowed(&query));

            if expanded_path.ends_with('/') {
                if let Ok(entries) = std::fs::read_dir(expanded_path.as_ref()) {
                    let mut matches: Vec<_> = entries
                        .filter_map(|entry| entry.ok())
                        .filter_map(|entry| {
                            launcher::create_file_entry(entry.path().to_string_lossy().to_string())
                                .map(|entry| SearchResult {
                                    app: entry,
                                    score: 1000,
                                })
                        })
                        .collect();
                    matches.sort_by(|a, b| {
                        match (a.app.icon_name == "folder", b.app.icon_name == "folder") {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => a.app.name.cmp(&b.app.name),
                        }
                    });
                    matches.truncate(100);
                    matches
                } else {
                    Vec::new()
                }
            } else {
                let path = std::path::Path::new(expanded_path.as_ref());
                if let Some(parent) = path.parent() {
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                        let mut matches: Vec<_> = entries
                            .filter_map(|entry| entry.ok())
                            .filter_map(|entry| {
                                let name = entry.file_name();
                                let name_str = name.to_string_lossy();
                                if name_str.contains(file_name) {
                                    launcher::create_file_entry(
                                        entry.path().to_string_lossy().to_string(),
                                    )
                                    .map(|entry| {
                                        SearchResult {
                                            app: entry,
                                            score: 1000,
                                        }
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();
                        matches.sort_by(|a, b| b.score.cmp(&a.score));
                        matches.truncate(100);
                        matches
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
        } else if query.is_empty() {
            let mut results: Vec<_> = cache
                .values()
                .par_bridge()
                .filter(|app| app.path.contains("/applications/") && app.path.ends_with(".desktop"))
                .map(|app| {
                    let heat_score = if app.launch_count > 0 {
                        (app.launch_count as i64 * 100) + 2000
                    } else {
                        0
                    };

                    let icon_score = if app.icon_name == "application-x-executable" {
                        0
                    } else {
                        1000
                    };

                    SearchResult {
                        app: app.clone(),
                        score: heat_score + icon_score,
                    }
                })
                .collect();

            results.par_sort_unstable_by(|a, b| b.score.cmp(&a.score));
            results.dedup_by(|a, b| a.app.name == b.app.name);
            results.truncate(100);
            results
        } else {
            let matcher = SkimMatcherV2::default().smart_case();
            let cache_vec: Vec<_> = cache.values().collect();

            let mut seen_names = std::collections::HashSet::new();
            let mut seen_execs = std::collections::HashSet::new();
            let mut results: Vec<SearchResult> = Vec::new();

            for app in cache_vec.iter() {
                if let Some(score) = matcher.fuzzy_match(&app.name.to_lowercase(), &query) {
                    let name_lower = app.name.to_lowercase();
                    let exec_name = app
                        .path
                        .split('/')
                        .last()
                        .unwrap_or("")
                        .split('.')
                        .next()
                        .unwrap_or("")
                        .to_lowercase();

                    if !seen_names.contains(&name_lower) && !seen_execs.contains(&exec_name) {
                        seen_names.insert(name_lower);
                        seen_execs.insert(exec_name);

                        let heat_score = if app.launch_count > 0 {
                            (app.launch_count as i64 * 100) + 2000
                        } else {
                            0
                        };

                        let icon_score = if app.icon_name == "application-x-executable" {
                            0
                        } else {
                            1000
                        };

                        results.push(SearchResult {
                            app: (*app).clone(),
                            score: score + heat_score + icon_score,
                        });
                    }
                }
            }

            results.par_sort_unstable_by(|a, b| b.score.cmp(&a.score));
            results.truncate(100);
            results
        };
        let _ = tx.send(results);
    });

    rx.await.unwrap_or_default()
}
