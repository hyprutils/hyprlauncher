use crate::{
    config::Config,
    log,
    ui::{create_error_overlay, LauncherWindow},
};
use gtk4::{
    glib::{self, ControlFlow},
    prelude::*,
    Application, ApplicationWindow,
};
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    process,
    sync::mpsc,
    time::{self, Duration, Instant},
};
use tokio::runtime::Runtime;

pub struct App {
    app: Application,
    rt: Runtime,
    entries: Option<Vec<String>>,
}

impl App {
    pub fn new() -> Self {
        log!("Initializing application runtime...");
        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        if !Self::can_create_instance() {
            log!("Another instance is already running, exiting");
            let app = Application::builder()
                .application_id("hyprutils.hyprlauncher")
                .flags(gtk4::gio::ApplicationFlags::ALLOW_REPLACEMENT)
                .build();

            app.register(None::<&gtk4::gio::Cancellable>)
                .expect("Failed to register application");

            app.activate();
            process::exit(0);
        }

        log!("Creating new application instance");
        let app = Application::builder()
            .application_id("hyprutils.hyprlauncher")
            .flags(gtk4::gio::ApplicationFlags::ALLOW_REPLACEMENT)
            .build();

        app.register(None::<&gtk4::gio::Cancellable>)
            .expect("Failed to register application");

        let (_tx, rx) = mpsc::channel::<()>();
        crate::config::Config::watch_changes(move || {
            let _ = _tx.send(());
        });

        let app_clone = app.clone();
        let mut last_update = Instant::now();

        glib::timeout_add_local(Duration::from_millis(100), move || {
            if rx.try_recv().is_ok() {
                let now = Instant::now();
                if now.duration_since(last_update).as_millis() > 250 {
                    if let Some(window) = app_clone.windows().first() {
                        if let Some(window) = window.downcast_ref::<ApplicationWindow>() {
                            let new_config = Config::load();
                            let error = Config::get_current_error();

                            if let Some(main_box) = window.first_child() {
                                if let Some(main_box) = main_box.downcast_ref::<gtk4::Box>() {
                                    if let Some(first_child) = main_box.first_child() {
                                        if first_child
                                            .css_classes()
                                            .iter()
                                            .any(|class| class == "error-overlay")
                                        {
                                            main_box.remove(&first_child);
                                        }
                                    }
                                }
                            }

                            if let Some(error) = error {
                                if let Some(main_box) = window.first_child() {
                                    if let Some(main_box) = main_box.downcast_ref::<gtk4::Box>() {
                                        let error_overlay = create_error_overlay(&error);
                                        main_box.prepend(&error_overlay);
                                    }
                                }
                            }

                            LauncherWindow::update_window_config(window, &new_config);
                        }
                    }
                    last_update = now;
                }
            }
            ControlFlow::Continue
        });

        if !app.is_remote() {
            let load_start = Instant::now();
            rt.block_on(async {
                crate::launcher::load_applications().await.unwrap();
            });
            log!(
                "Loading applications ({:.3}ms)",
                load_start.elapsed().as_secs_f64() * 1000.0
            );
        }

        Self {
            app,
            rt,
            entries: None,
        }
    }

    pub fn new_dmenu(entries: Vec<String>) -> Self {
        log!("Initializing dmenu application runtime...");
        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        log!("Creating new dmenu application instance");
        let app = Application::builder()
            .application_id("hyprutils.hyprlauncher.dmenu")
            .flags(
                gtk4::gio::ApplicationFlags::NON_UNIQUE
                    | gtk4::gio::ApplicationFlags::HANDLES_COMMAND_LINE,
            )
            .build();

        app.register(None::<&gtk4::gio::Cancellable>)
            .expect("Failed to register application");

        let rt_handle = rt.handle().clone();
        let entries_clone = entries.clone();

        app.connect_activate(move |app| {
            let window = LauncherWindow::new_dmenu(app, rt_handle.clone(), entries_clone.clone());
            window.present();
        });

        app.connect_command_line(|app, _cmdline| {
            app.activate();
            0
        });

        Self {
            app,
            rt,
            entries: Some(entries),
        }
    }

    pub fn run(&self) -> i32 {
        let rt_handle = self.rt.handle().clone();
        let entries = self.entries.clone();

        self.app.connect_activate(move |app| {
            let windows = app.windows();
            if let Some(window) = windows.first() {
                window.present();
            } else {
                let window = if let Some(entries) = &entries {
                    LauncherWindow::new_dmenu(app, rt_handle.clone(), entries.clone())
                } else {
                    LauncherWindow::new(app, rt_handle.clone())
                };
                window.present();
            }
        });

        let status = self.app.run();

        if self.entries.is_none() {
            if let Some(instance_file) = Self::get_instance_file() {
                let _ = fs::remove_file(instance_file);
            }
        }

        status.into()
    }

    fn get_runtime_dir() -> PathBuf {
        let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or(String::from("/tmp"));
        PathBuf::from(format!("{}/hyprlauncher", xdg_runtime_dir))
    }

    fn get_instance_file() -> Option<PathBuf> {
        let runtime_dir = Self::get_runtime_dir();
        let pid = process::id();
        Some(runtime_dir.join(format!("instance-{}", pid)))
    }

    fn can_create_instance() -> bool {
        let runtime_dir = Self::get_runtime_dir();
        fs::create_dir_all(&runtime_dir)
            .unwrap_or_else(|_| panic!("Failed to create runtime directory"));

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
            time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
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
