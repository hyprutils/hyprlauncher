use crate::launcher;
use crate::ui::LauncherWindow;
use gtk4::prelude::*;
use gtk4::Application;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct App {
    app: Application,
    rt: Arc<Runtime>,
}

impl App {
    pub fn new() -> Self {
        let rt = Arc::new(Runtime::new().unwrap());

        let pre_gtk = std::time::Instant::now();
        println!("Before GTK app creation");

        let app = Application::builder()
            .application_id("hyprutils.hyprlauncher")
            .build();

        println!(
            "After GTK app creation ({:.3}ms)",
            pre_gtk.elapsed().as_secs_f64() * 1000.0
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

        let pre_connect = std::time::Instant::now();
        let rt = self.rt.handle().clone();
        self.app.connect_activate(move |app| {
            println!(
                "Inside activate callback ({:.3}ms from start)",
                pre_connect.elapsed().as_secs_f64() * 1000.0
            );

            let window_start = std::time::Instant::now();
            let window = LauncherWindow::new(app, rt.clone());
            println!(
                "After window creation ({:.3}ms)",
                window_start.elapsed().as_secs_f64() * 1000.0
            );

            let present_start = std::time::Instant::now();
            window.present();
            println!(
                "After window present ({:.3}ms)",
                present_start.elapsed().as_secs_f64() * 1000.0
            );

            println!(
                "\nGrand total time: {:.3}ms",
                grand_total.elapsed().as_secs_f64() * 1000.0
            );
        });

        println!(
            "Before app.run() ({:.3}ms)",
            grand_total.elapsed().as_secs_f64() * 1000.0
        );

        self.app.run();
    }
}
