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

        let gtk_start = std::time::Instant::now();
        let app = Application::builder()
            .application_id("hyprutils.hyprlauncher")
            .build();
        println!(
            "GTK initialization ({:.3}ms)",
            gtk_start.elapsed().as_secs_f64() * 1000.0
        );

        let load_start = std::time::Instant::now();
        rt.block_on(launcher::load_applications());
        println!(
            "Loading applications ({:.3}ms)",
            load_start.elapsed().as_secs_f64() * 1000.0
        );

        Self { app, rt }
    }

    pub fn run(&self) {
        let grand_total = std::time::Instant::now();
        println!(
            "Starting hyprlauncher ({:.3}ms)",
            grand_total.elapsed().as_secs_f64() * 1000.0
        );
        let rt = self.rt.handle().clone();

        self.app.connect_activate(move |app| {
            let window = LauncherWindow::new(app, rt.clone());
            window.present();
            println!(
                "\nGrand total time: {:.3}ms",
                grand_total.elapsed().as_secs_f64() * 1000.0
            );
        });

        self.app.run();
    }
}
