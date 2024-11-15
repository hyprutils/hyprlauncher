mod app;
mod config;
mod launcher;
mod search;
mod ui;

use app::App;

fn main() {
    println!("Starting Hyprlauncher...");
    let app = App::new();
    std::process::exit(app.run());
}
