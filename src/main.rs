mod app;
mod config;
mod launcher;
mod search;
mod ui;

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        if $crate::config::LOGGING_ENABLED.load(std::sync::atomic::Ordering::SeqCst) {
            println!($($arg)*);
        }
    }};
}

fn main() {
    log!("Starting Hyprlauncher...");
    let app = app::App::new();
    std::process::exit(app.run());
}
