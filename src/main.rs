mod app;
mod config;
mod launcher;
mod search;
mod ui;

use crate::app::App;
use std::io::{self, BufRead};
use tokio::runtime::Runtime;

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        if $crate::config::LOGGING_ENABLED.load(std::sync::atomic::Ordering::SeqCst) {
            println!($($arg)*);
        }
    }};
}

fn run_dmenu_mode() -> i32 {
    let stdin = io::stdin();
    let lines: Vec<String> = stdin.lock().lines().map_while(Result::ok).collect();
    let config = config::Config::load();

    let rt = Runtime::new().expect("Failed to create runtime");

    match rt.block_on(search::search_dmenu(String::new(), lines, config)) {
        Ok(results) => {
            for result in results {
                println!("{}", result);
            }
            0
        }
        Err(_) => 1,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_dmenu = args.len() > 1 && (args[1] == "--dmenu" || args[1] == "-d");

    if is_dmenu {
        let stdin = io::stdin();
        let lines: Vec<String> = stdin.lock().lines().map_while(Result::ok).collect();

        if atty::is(atty::Stream::Stdout) {
            let app = App::new_dmenu(lines);
            std::process::exit(app.run());
        } else {
            std::process::exit(run_dmenu_mode());
        }
    }

    if args.len() > 1 {
        eprintln!("Unknown option: {}", args[1]);
        std::process::exit(1);
    }

    let app = App::new();
    std::process::exit(app.run());
}
