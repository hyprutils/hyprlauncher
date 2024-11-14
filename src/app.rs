use crate::launcher;
use crate::ui::LauncherWindow;
use gtk4::prelude::*;
use gtk4::Application;
use tokio::runtime::Runtime;

pub struct App {
    app: Application,
    rt: Runtime,
}

impl App {
    pub fn new() -> Self {
        println!("Pre-initializing hyprlauncher");
        let rt = Runtime::new().unwrap();

        let app = Application::builder()
            .application_id("hyprutils.hyprlauncher")
            .build();

        println!("Loading applications...");
        rt.block_on(launcher::load_applications());
        println!("Applications loaded successfully");

        Self { app, rt }
    }

    pub fn run(&self) {
        println!("Starting hyprlauncher");
        let rt = self.rt.handle().clone();

        self.app.connect_activate(move |app| {
            let window = LauncherWindow::new(app, rt.clone());
            window.present();
        });

        self.app.run();
    }
}
