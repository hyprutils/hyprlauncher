use crate::{
    config::{Config, WebSearch},
    launcher::{self, AppEntry, EntryType, APP_CACHE},
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rink_core::{one_line, simple_context};
use std::{
    collections::HashMap,
    os::unix::fs::PermissionsExt,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::oneshot;
use x11rb::{
    connection::Connection,
    protocol::xproto::{self, ConnectionExt},
};

const BONUS_SCORE_ICON_NAME: i64 = 1000;
const BONUS_SCORE_BINARY: i64 = 3000;
const BONUS_SCORE_KEYWORD_MATCH: i64 = 2500;
const BONUS_SCORE_CATEGORY_MATCH: i64 = 2000;
const BONUS_SCORE_WEB_SEARCH: i64 = -1000;
const BONUS_SCORE_CALC: i64 = 1000;
const OPEN_WINDOW_PENALTY: i64 = -500;

pub struct SearchResult {
    pub app: AppEntry,
    pub score: i64,
}
pub struct HistoryEntry {
    last_used: u64,
    use_count: i64,
}

fn should_exclude_web_search(query: &str) -> bool {
    let excluded_terms = ["__config_reload__", "__refresh__"];
    excluded_terms
        .iter()
        .any(|term| query.eq_ignore_ascii_case(term))
}

fn get_active_window_classes() -> Vec<String> {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    let mut classes = Vec::new();

    if let Ok(tree) = conn.query_tree(screen.root).unwrap().reply() {
        for window in tree.children {
            if let Ok(cookie) = conn.get_property(
                false,
                window,
                xproto::AtomEnum::WM_CLASS,
                xproto::AtomEnum::STRING,
                0,
                1024,
            ) {
                if let Ok(props) = cookie.reply() {
                    if let Ok(class) = String::from_utf8(props.value) {
                        classes.push(class.to_lowercase());
                    }
                }
            }
        }
    }

    classes
}

pub async fn search_applications(
    query: &str,
    config: &Config,
) -> Result<Vec<SearchResult>, std::io::Error> {
    let (tx, rx) = oneshot::channel();
    let query = query.to_owned();
    let query_lower = query.to_lowercase();
    let max_results = config.window.max_entries;
    let calculator_enabled = config.modes.calculator;
    let web_search_config = config.web_search.clone();
    let show_actions = config.window.show_actions;

    tokio::task::spawn_blocking(move || {
        let cache = APP_CACHE.blocking_read();

        let mut results = match query.chars().next() {
            None => {
                let history = load_history();
                let mut heatmap_results = Vec::new();
                let mut alphabetical_results = Vec::new();

                for app in cache.values() {
                    if app.path.ends_with(".desktop") {
                        let result = SearchResult {
                            score: calculate_bonus_score(app),
                            app: app.clone(),
                        };

                        if history.contains_key(&app.name) {
                            heatmap_results.push(result);
                        } else {
                            alphabetical_results.push(result);
                        }
                    }
                }

                heatmap_results.sort_unstable_by_key(|item| -item.score);
                alphabetical_results
                    .sort_by(|a, b| a.app.name.to_lowercase().cmp(&b.app.name.to_lowercase()));

                let mut results = heatmap_results;
                results.extend(alphabetical_results);
                results.truncate(max_results);
                results
            }
            Some(_) => {
                let matcher = SkimMatcherV2::default();
                let mut results = Vec::with_capacity(max_results);
                let mut seen_names = std::collections::HashSet::new();

                for app in cache.values() {
                    let name_lower = app.name.to_lowercase();
                    let name_key = name_lower.clone();
                    let mut added = false;

                    if name_lower.eq_ignore_ascii_case(&query_lower) {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: BONUS_SCORE_BINARY + calculate_bonus_score(app),
                        });
                        seen_names.insert(name_key.clone());
                        added = true;
                    }

                    if app.keywords.iter().any(|k| k.eq_ignore_ascii_case(&query)) && !added {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: BONUS_SCORE_KEYWORD_MATCH + calculate_bonus_score(app),
                        });
                        seen_names.insert(name_key.clone());
                        added = true;
                    }

                    if app
                        .categories
                        .iter()
                        .any(|c| c.eq_ignore_ascii_case(&query))
                        && !added
                    {
                        results.push(SearchResult {
                            app: app.clone(),
                            score: BONUS_SCORE_CATEGORY_MATCH + calculate_bonus_score(app),
                        });
                        seen_names.insert(name_key.clone());
                        added = true;
                    }

                    if let Some(score) = matcher.fuzzy_match(&name_lower, &query) {
                        if !added {
                            results.push(SearchResult {
                                app: app.clone(),
                                score: score + calculate_bonus_score(app),
                            });
                            seen_names.insert(name_key.clone());
                            added = true;
                        }
                    }

                    if show_actions {
                        for action in &app.actions {
                            let mut action_app = app.clone();
                            action_app.name = format!("{} - {}", app.name, action.name);
                            action_app.exec = action.exec.clone();
                            if let Some(icon) = &action.icon_name {
                                action_app.icon_name = icon.clone();
                            }

                            let action_name = action.name.to_lowercase();
                            if query.is_empty()
                                || action_name.contains(&query_lower)
                                || matcher.fuzzy_match(&action_name, &query).is_some()
                            {
                                results.push(SearchResult {
                                    app: action_app,
                                    score: calculate_bonus_score(app) - 100,
                                });
                            }
                        }
                    }

                    if !added {
                        for keyword in &app.keywords {
                            if let Some(score) =
                                matcher.fuzzy_match(&keyword.to_lowercase(), &query)
                            {
                                results.push(SearchResult {
                                    app: app.clone(),
                                    score: score + calculate_bonus_score(app),
                                });
                                seen_names.insert(name_key.clone());
                                break;
                            }
                        }

                        for category in &app.categories {
                            if let Some(score) =
                                matcher.fuzzy_match(&category.to_lowercase(), &query)
                            {
                                results.push(SearchResult {
                                    app: app.clone(),
                                    score: score + calculate_bonus_score(app),
                                });
                                seen_names.insert(name_key.clone());
                                break;
                            }
                        }
                    }
                }

                if !seen_names.contains(&query_lower) {
                    if let Some(result) = check_binary(&query) {
                        results.push(result);
                    }
                }

                if results.is_empty()
                    && web_search_config.enabled
                    && !should_exclude_web_search(&query)
                {
                    results.push(create_web_search_entry(&query, &web_search_config));
                }

                if results.is_empty()
                    && calculator_enabled
                    && query.chars().next().unwrap().is_ascii_digit()
                {
                    results.push(create_calc_entry(&query));
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
            results.push(create_web_search_entry(&query, &web_search_config));
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
    let mut score = 0;
    let history = load_history();

    if let Some(entry) = history.get(&app.name) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let seconds_since_used = (now - entry.last_used) as i64;
        score = 10000 - (seconds_since_used / 10);

        score += (entry.use_count * 20).min(200);
    } else {
        score += (app.launch_count as i64 * 20).min(200);
    }

    if app.icon_name != "application-x-executable" {
        score += BONUS_SCORE_ICON_NAME;
    }

    let active_windows = get_active_window_classes();
    if active_windows.iter().any(|class| {
        app.name.to_lowercase().contains(class) || app.exec.to_lowercase().contains(class)
    }) {
        score += OPEN_WINDOW_PENALTY;
    }

    score
}

#[inline(always)]
fn check_binary(query: &str) -> Option<SearchResult> {
    let parts: Vec<&str> = query.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let bin_path = format!("/usr/bin/{}", parts[0]);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

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
                last_used: Some(now),
                entry_type: EntryType::Application,
                score_boost: BONUS_SCORE_BINARY,
                keywords: Vec::new(),
                categories: Vec::new(),
                terminal: false,
                actions: Vec::new(),
            },
            score: BONUS_SCORE_BINARY,
        })
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

fn create_web_search_entry(query: &str, config: &WebSearch) -> SearchResult {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if let Some(colon_pos) = query.find(':') {
        let (prefix, search_term) = query.split_at(colon_pos);
        let search_term = &search_term[1..];

        if let Some(prefix_config) = config.prefixes.iter().find(|p| p.prefix == prefix) {
            return SearchResult {
                app: AppEntry {
                    name: format!("Search '{}' on {}", search_term, prefix),
                    description: String::from("Open in default web browser"),
                    path: String::new(),
                    exec: format!(
                        "xdg-open \"{}{}\"",
                        prefix_config.url,
                        utf8_percent_encode(search_term, NON_ALPHANUMERIC)
                    ),
                    icon_name: String::from("web-browser"),
                    launch_count: 0,
                    last_used: Some(now),
                    entry_type: EntryType::Application,
                    score_boost: 0,
                    keywords: Vec::new(),
                    categories: vec![String::from("Web Search")],
                    terminal: false,
                    actions: Vec::new(),
                },
                score: BONUS_SCORE_WEB_SEARCH,
            };
        }
    }

    SearchResult {
        app: AppEntry {
            name: format!("Search '{}' on the web", query),
            description: String::from("Open in default web browser"),
            path: String::new(),
            exec: format!(
                "xdg-open \"{}{}\"",
                config.engine.get_url(),
                utf8_percent_encode(query, NON_ALPHANUMERIC)
            ),
            icon_name: String::from("web-browser"),
            launch_count: 0,
            last_used: Some(now),
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

fn load_history() -> HashMap<String, HistoryEntry> {
    let mut history = HashMap::new();

    if let Ok(heatmap) = launcher::load_heatmap() {
        for (name, entry) in heatmap {
            history.insert(
                name,
                HistoryEntry {
                    last_used: entry.last_used,
                    use_count: entry.count.into(),
                },
            );
        }
    }

    history
}

fn create_calc_entry(query: &str) -> SearchResult {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let res = handle_calculation(query);

    SearchResult {
        app: AppEntry {
            name: res.clone(),
            description: String::from("Copy to clipboard"),
            path: String::new(),
            exec: format!("wl-copy -t text/plain \"{}\"", &res),
            icon_name: String::from("calculator"),
            launch_count: 0,
            last_used: Some(now),
            entry_type: EntryType::Application,
            score_boost: 0,
            keywords: Vec::new(),
            categories: vec![String::from("Calculation")],
            terminal: false,
            actions: Vec::new(),
        },
        score: BONUS_SCORE_CALC,
    }
}

#[inline(always)]
fn handle_calculation(query: &str) -> String {
    let mut ctx = simple_context().unwrap();

    let res = match one_line(&mut ctx, query) {
        Ok(res) => res,
        Err(_e) => "0".to_string(),
    };

    match res.find("(") {
        Some(pos) => res[..pos - 1].to_string(),
        None => res,
    }
}
