use crate::ui::LauncherWindow;
use gtk4::prelude::*;
use gtk4::Application;

pub struct App {
    app: Application,
}

impl App {
    pub fn new() -> Self {
        let app = Application::builder()
            .application_id("nnyyxxxx.hyprlauncher")
            .build();

        Self { app }
    }

    pub fn run(&self) {
        self.app.connect_activate(move |app| {
            let window = LauncherWindow::new(app);
            window.present();
        });

        self.app.run();
    }
}
