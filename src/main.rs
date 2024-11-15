mod app;
mod config;
mod launcher;
mod search;
mod ui;

use app::App;

fn main() {
    let app = App::new();
    std::process::exit(app.run());
}
