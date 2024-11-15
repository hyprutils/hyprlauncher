use crate::{config::Config, ui::LauncherWindow};
use gtk4::{
    glib::{self, ControlFlow},
    prelude::*,
    Application, ApplicationWindow,
};
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process,
    sync::mpsc,
    time::Duration,
};
use tokio::runtime::Runtime;

pub struct App {
    app: Application,
    rt: Runtime,
}

impl App {
    pub fn new() -> Self {
        println!("Initializing application runtime...");
        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        if !Self::can_create_instance() {
            println!("Another instance is already running, exiting");
            let app = Application::builder()
                .application_id("hyprutils.hyprlauncher")
                .flags(gtk4::gio::ApplicationFlags::ALLOW_REPLACEMENT)
                .build();

            app.register(None::<&gtk4::gio::Cancellable>)
                .expect("Failed to register application");

            app.activate();
            process::exit(0);
        }

        println!("Creating new application instance");
        let app = Application::builder()
            .application_id("hyprutils.hyprlauncher")
            .flags(gtk4::gio::ApplicationFlags::ALLOW_REPLACEMENT)
            .build();

        app.register(None::<&gtk4::gio::Cancellable>)
            .expect("Failed to register application");

        let (tx, rx) = mpsc::channel();
        crate::config::Config::watch_changes(move || {
            let _ = tx.send(());
        });

        let app_clone = app.clone();
        let mut last_config = Config::load();
        let mut last_update = std::time::Instant::now();

        glib::timeout_add_local(Duration::from_millis(100), move || {
            if rx.try_recv().is_ok() {
                let now = std::time::Instant::now();
                if now.duration_since(last_update).as_millis() > 250 {
                    if let Some(window) = app_clone.windows().first() {
                        println!("Loading new config for comparison");
                        let new_config = Config::load();
                        if new_config != last_config {
                            if let Some(launcher_window) =
                                window.downcast_ref::<ApplicationWindow>()
                            {
                                println!("Config changed, updating window");
                                LauncherWindow::update_window_config(launcher_window, &new_config);
                                last_config = new_config;
                                last_update = now;
                            }
                        } else {
                            println!("Config unchanged");
                        }
                    }
                }
            }
            ControlFlow::Continue
        });

        if !app.is_remote() {
            let load_start = std::time::Instant::now();
            rt.block_on(async {
                crate::launcher::load_applications().await;
            });
            println!(
                "Loading applications ({:.3}ms)",
                load_start.elapsed().as_secs_f64() * 1000.0
            );
        }

        Self { app, rt }
    }

    pub fn run(&self) -> i32 {
        let rt_handle = self.rt.handle().clone();

        self.app.connect_activate(move |app| {
            let windows = app.windows();
            if let Some(window) = windows.first() {
                window.present();
            } else {
                let window = LauncherWindow::new(app, rt_handle.clone());
                window.present();
            }
        });

        let status = self.app.run();

        if !self.app.is_remote() {
            self.app.quit();

            if let Some(instance_file) = Self::get_instance_file() {
                let _ = fs::remove_file(instance_file);
            }
        }

        status.into()
    }

    fn get_instance_file() -> Option<PathBuf> {
        let runtime_dir = PathBuf::from("/tmp/hyprlauncher");
        let pid = process::id();
        Some(runtime_dir.join(format!("instance-{}", pid)))
    }

    fn can_create_instance() -> bool {
        let runtime_dir = PathBuf::from("/tmp/hyprlauncher");
        let _ = fs::create_dir_all(&runtime_dir);

        Self::cleanup_stale_instances(&runtime_dir);

        let instances: Vec<_> = fs::read_dir(&runtime_dir)
            .unwrap_or_else(|_| panic!("Failed to read runtime directory"))
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("instance-"))
            .collect();

        if instances.len() >= 2 {
            return false;
        }

        let pid = process::id();
        let instance_file = runtime_dir.join(format!("instance-{}", pid));
        let mut file = File::create(&instance_file).unwrap();
        let _ = writeln!(
            file,
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        let instance_file_clone = instance_file.clone();
        ctrlc::set_handler(move || {
            let _ = fs::remove_file(&instance_file_clone);
            process::exit(0);
        })
        .expect("Error setting Ctrl-C handler");

        true
    }

    fn cleanup_stale_instances(runtime_dir: &PathBuf) {
        if let Ok(entries) = fs::read_dir(runtime_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if let Some(filename) = path.file_name() {
                    if let Some(pid_str) = filename.to_string_lossy().strip_prefix("instance-") {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            if !process_exists(pid) {
                                let _ = fs::remove_file(path);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn process_exists(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{}", pid)).exists()
}
