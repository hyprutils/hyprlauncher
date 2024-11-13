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
        let rt = Runtime::new().unwrap();

        let app = Application::builder()
            .application_id("hyprutils.hyprlauncher")
            .build();

        rt.block_on(launcher::load_applications());

        Self { app, rt }
    }

    pub fn run(&self) {
        let rt = self.rt.handle().clone();

        self.app.connect_activate(move |app| {
            let window = LauncherWindow::new(app, rt.clone());
            window.present();
        });

        self.app.run();
    }
}
