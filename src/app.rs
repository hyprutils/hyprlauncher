use crate::{launcher, ui::LauncherWindow};
use gtk4::{prelude::*, Application, gio::Cancellable};
use tokio::runtime::Runtime;

pub struct App {
    app: Application,
    rt: Runtime,
}

impl App {
    pub fn new() -> Self {
        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        let app = Application::builder()
            .application_id("hyprutils.hyprlauncher")
            .flags(gtk4::gio::ApplicationFlags::ALLOW_REPLACEMENT)
            .build();

        app.register(None::<&Cancellable>)
            .expect("Failed to register application");

        if !app.is_remote() {
            let load_start = std::time::Instant::now();
            rt.block_on(async {
                launcher::load_applications().await;
            });
            println!(
                "Loading applications ({:.3}ms)",
                load_start.elapsed().as_secs_f64() * 1000.0
            );
        }

        Self { app, rt }
    }

    pub fn run(&self) {
        let rt_handle = self.rt.handle().clone();

        if self.app.is_remote() {
            self.app.activate();
            return;
        }

        self.app.connect_activate(move |app| {
            let window = LauncherWindow::new(app, rt_handle.clone());
            window.present();
        });

        self.app.run();
    }
}
