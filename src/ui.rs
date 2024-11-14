use crate::config::{Config, WindowAnchor};
use crate::launcher::{self, AppEntry, EntryType};
use crate::search;
use gtk4::gdk::Key;
use gtk4::glib::{self};
use gtk4::prelude::*;
use gtk4::ListBoxRow;
use gtk4::{Application, ApplicationWindow, Label, ListBox, ScrolledWindow, SearchEntry};
use gtk4::{Box as GtkBox, CssProvider, Orientation, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use tokio::runtime::Handle;

pub struct LauncherWindow {
    window: ApplicationWindow,
    search_entry: SearchEntry,
    results_list: ListBox,
    app_data_store: Rc<RefCell<Vec<AppEntry>>>,
    rt: Handle,
}

#[allow(non_camel_case_types)]
impl LauncherWindow {
    pub fn new(app: &Application, rt: Handle) -> Self {
        let window_start = std::time::Instant::now();
        println!(
            "Creating launcher window ({:.3}ms)",
            window_start.elapsed().as_secs_f64() * 1000.0
        );

        let config = Config::load();
        let window = ApplicationWindow::builder()
            .application(app)
            .title("HyprLauncher")
            .default_width(config.window.width)
            .default_height(config.window.height)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_keyboard_mode(KeyboardMode::Exclusive);
        Self::setup_window_anchoring(&window, &config);
        Self::apply_window_margins(&window, &config);

        let main_box = GtkBox::new(Orientation::Vertical, 0);
        let search_entry = SearchEntry::new();
        let scrolled = ScrolledWindow::new();
        let results_list = ListBox::new();

        scrolled.set_vexpand(true);
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::External);
        results_list.set_selection_mode(gtk4::SelectionMode::Single);

        scrolled.set_child(Some(&results_list));
        if config.window.show_search {
            main_box.append(&search_entry);
        }
        main_box.append(&scrolled);
        window.set_child(Some(&main_box));

        let css_start = std::time::Instant::now();
        let css_provider = CssProvider::new();
        css_provider.load_from_data(&config.get_css());
        if let Some(native) = window.native() {
            gtk4::style_context_add_provider_for_display(
                &native.display(),
                &css_provider,
                STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        println!(
            "CSS loading and application ({:.3}ms)",
            css_start.elapsed().as_secs_f64() * 1000.0
        );

        let launcher = Self {
            window,
            search_entry,
            results_list,
            app_data_store: Rc::new(RefCell::new(Vec::with_capacity(50))),
            rt,
        };

        launcher.setup_signals();

        let results_list = launcher.results_list.clone();
        let app_data_store = launcher.app_data_store.clone();
        let rt = launcher.rt.clone();

        let search_start = std::time::Instant::now();
        glib::spawn_future_local(async move {
            let results = rt.block_on(search::search_applications(""));
            update_results_list(&results_list, results, &app_data_store);
        });
        println!(
            "Initial search population ({:.3}ms)",
            search_start.elapsed().as_secs_f64() * 1000.0
        );

        launcher
    }

    pub fn present(&self) {
        let present_start = std::time::Instant::now();
        println!(
            "Presenting launcher window ({:.3}ms)",
            present_start.elapsed().as_secs_f64() * 1000.0
        );

        self.window.present();

        if Config::load().window.show_search {
            self.search_entry.grab_focus();
        }
    }

    fn setup_window_anchoring(window: &ApplicationWindow, config: &Config) {
        let anchors = match config.window.anchor {
            WindowAnchor::center => [false; 4],
            WindowAnchor::top => [true, false, false, false],
            WindowAnchor::bottom => [false, false, true, false],
            WindowAnchor::left => [false, false, false, true],
            WindowAnchor::right => [false, true, false, false],
            WindowAnchor::top_left => [true, false, false, true],
            WindowAnchor::top_right => [true, true, false, false],
            WindowAnchor::bottom_left => [false, false, true, true],
            WindowAnchor::bottom_right => [false, true, true, true],
        };
        window.set_anchors(anchors);
    }

    fn apply_window_margins(window: &ApplicationWindow, config: &Config) {
        window.set_margin(Edge::Top, config.window.margin_top);
        window.set_margin(Edge::Bottom, config.window.margin_bottom);
        window.set_margin(Edge::Left, config.window.margin_left);
        window.set_margin(Edge::Right, config.window.margin_right);
    }

    fn setup_signals(&self) {
        let config = Config::load();

        if config.window.show_search {
            let search_entry = self.search_entry.clone();
            let search_entry_for_enter = search_entry.clone();
            let search_entry_for_leave = search_entry.clone();
            let search_entry_for_controller = search_entry.clone();

            let focus_controller = gtk4::EventControllerFocus::new();

            focus_controller.connect_enter(move |_| {
                search_entry_for_enter.set_placeholder_text(None);
            });

            focus_controller.connect_leave(move |_| {
                search_entry_for_leave.set_placeholder_text(Some("Press / to start searching"));
            });

            search_entry_for_controller.add_controller(focus_controller);

            let results_list_for_search = self.results_list.clone();
            let app_data_store_for_search = self.app_data_store.clone();
            let rt_for_search = self.rt.clone();

            self.search_entry.connect_changed(move |entry| {
                let query = entry.text().to_string();
                let results_list = results_list_for_search.clone();
                let app_data_store = app_data_store_for_search.clone();
                let rt = rt_for_search.clone();

                glib::spawn_future_local(async move {
                    let results = rt.block_on(search::search_applications(&query));
                    update_results_list(&results_list, results, &app_data_store);
                });
            });

            let results_list_for_search_key = self.results_list.clone();
            let search_controller = gtk4::EventControllerKey::new();
            search_controller.connect_key_pressed(move |_, key, _, _| {
                let results_list = results_list_for_search_key.clone();
                match key {
                    Key::Escape => {
                        if let Some(row) = results_list.first_child() {
                            if let Some(list_row) = row.downcast_ref::<ListBoxRow>() {
                                results_list.select_row(Some(list_row));
                                list_row.grab_focus();
                            }
                        }
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            });
            self.search_entry.add_controller(search_controller);
        }

        let results_list_for_window = self.results_list.clone();
        let window_for_window = self.window.clone();
        let search_entry_for_window = self.search_entry.clone();

        let window_controller = gtk4::EventControllerKey::new();
        window_controller.connect_key_pressed(move |_, key, _, _| {
            let config = Config::load();
            let results_list = results_list_for_window.clone();
            let window = window_for_window.clone();
            let search_entry = search_entry_for_window.clone();

            match key {
                Key::Escape => {
                    if config.window.show_search && search_entry.has_focus() {
                        if search_entry.text().is_empty() {
                            if let Some(row) = results_list.first_child() {
                                if let Some(list_row) = row.downcast_ref::<ListBoxRow>() {
                                    results_list.select_row(Some(list_row));
                                    list_row.grab_focus();
                                }
                            }
                        } else {
                            search_entry.set_text("");
                        }
                    } else {
                        window.close();
                    }
                    glib::Propagation::Stop
                }
                Key::slash if config.window.show_search => {
                    search_entry.grab_focus();
                    glib::Propagation::Stop
                }
                Key::Up | Key::k if config.window.vim_keys || key == Key::Up => {
                    if !search_entry.has_focus() {
                        select_previous(&results_list);
                    }
                    glib::Propagation::Stop
                }
                Key::Down | Key::j if config.window.vim_keys || key == Key::Down => {
                    if !search_entry.has_focus() {
                        select_next(&results_list);
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        self.window.add_controller(window_controller);

        let window_for_row = self.window.clone();
        let search_entry_for_row = self.search_entry.clone();
        let app_data_store_for_row = self.app_data_store.clone();

        self.results_list.connect_row_activated(move |_, row| {
            if let Some(app_data) = get_app_data(row.index() as usize, &app_data_store_for_row) {
                if launch_application(&app_data, &search_entry_for_row) {
                    window_for_row.close();
                }
            }
        });

        let results_list_for_activate = self.results_list.clone();
        let window_for_activate = self.window.clone();
        let search_entry_for_activate = self.search_entry.clone();
        let app_data_store_for_activate = self.app_data_store.clone();

        self.search_entry.connect_activate(move |_| {
            if let Some(row) = results_list_for_activate.selected_row() {
                if let Some(app_data) =
                    get_app_data(row.index() as usize, &app_data_store_for_activate)
                {
                    if launch_application(&app_data, &search_entry_for_activate) {
                        window_for_activate.close();
                    }
                }
            }
        });
    }
}

fn get_app_data(index: usize, store: &Rc<RefCell<Vec<AppEntry>>>) -> Option<AppEntry> {
    store.borrow().get(index).cloned()
}

fn update_results_list(
    list: &ListBox,
    results: Vec<search::SearchResult>,
    store: &Rc<RefCell<Vec<AppEntry>>>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let mut store = store.borrow_mut();
    store.clear();
    store.reserve(50);

    if results.is_empty() {
        let empty_row = gtk4::ListBoxRow::new();
        empty_row.set_visible(true);
        empty_row.set_selectable(false);
        empty_row.add_css_class("invisible-row");
        empty_row.set_child(Some(&Label::new(Some(""))));
        list.append(&empty_row);
        return;
    }

    let results = if results.len() > 50 {
        &results[..50]
    } else {
        &results
    };

    let mut rows = Vec::with_capacity(results.len());
    store.extend(results.iter().map(|r| r.app.clone()));

    let config = Config::load();
    for result in results {
        rows.push(create_result_row(&result.app, &config));
    }

    for row in rows {
        list.append(&row);
    }

    if let Some(first_row) = list.row_at_index(0) {
        list.select_row(Some(&first_row));
    }
}

#[inline]
fn create_result_row(app: &AppEntry, config: &Config) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    let box_row = GtkBox::new(Orientation::Horizontal, 12);

    box_row.set_margin_start(12);
    box_row.set_margin_end(12);
    box_row.set_margin_top(8);
    box_row.set_margin_bottom(8);

    if config.window.show_icons {
        let icon = if !app.icon_name.is_empty() && app.icon_name != "application-x-executable" {
            gtk4::Image::from_icon_name(&app.icon_name)
        } else {
            gtk4::Image::new()
        };

        icon.set_pixel_size(32);
        icon.set_margin_end(8);
        box_row.append(&icon);
    }

    let text_box = GtkBox::new(Orientation::Vertical, 4);
    text_box.set_hexpand(true);

    let name_label = create_label(&app.name, "app-name", true);
    text_box.append(&name_label);

    if config.window.show_descriptions && !app.description.is_empty() {
        let desc_label = create_label(&app.description, "app-description", true);
        text_box.append(&desc_label);
    }

    if config.window.show_paths {
        let path_label = create_label(&app.path, "app-path", true);
        text_box.append(&path_label);
    }

    box_row.append(&text_box);
    row.set_child(Some(&box_row));
    row
}

#[inline]
fn create_label(text: &str, css_class: &str, wrap: bool) -> Label {
    let label = Label::new(Some(text));
    label.set_halign(gtk4::Align::Start);
    if wrap {
        label.set_wrap(true);
        label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        label.set_max_width_chars(50);
    }
    label.add_css_class(css_class);
    label
}

fn select_next(list: &ListBox) {
    if let Some(current) = list.selected_row() {
        if let Some(next) = list.row_at_index(current.index() + 1) {
            list.select_row(Some(&next));
            next.grab_focus();
        }
    }
}

fn select_previous(list: &ListBox) {
    if let Some(current) = list.selected_row() {
        if current.index() > 0 {
            if let Some(prev) = list.row_at_index(current.index() - 1) {
                list.select_row(Some(&prev));
                prev.grab_focus();
            }
        }
    }
}

fn launch_application(app: &AppEntry, search_entry: &SearchEntry) -> bool {
    match app.entry_type {
        EntryType::Application => {
            println!("Launching application: {}", app.name);
            let exec = app
                .exec
                .replace("%f", "")
                .replace("%F", "")
                .replace("%u", "")
                .replace("%U", "")
                .replace("%i", "")
                .replace("%c", &app.name)
                .trim()
                .to_string();

            launcher::increment_launch_count(app);

            Command::new("sh").arg("-c").arg(&exec).spawn().is_ok()
        }
        EntryType::File => {
            if app.icon_name == "folder" {
                println!("Opening folder: {}", app.path);
                let path = if app.path.ends_with('/') {
                    app.path.clone()
                } else {
                    format!("{}/", app.path)
                };
                search_entry.set_text(&path);
                search_entry.set_position(-1);

                false
            } else {
                println!("Opening file: {}", app.path);
                Command::new("sh").arg("-c").arg(&app.exec).spawn().is_ok()
            }
        }
    }
}

trait WindowAnchoring {
    fn set_anchors(&self, anchors: [bool; 4]);
}

impl WindowAnchoring for ApplicationWindow {
    fn set_anchors(&self, anchors: [bool; 4]) {
        self.set_anchor(Edge::Top, anchors[0]);
        self.set_anchor(Edge::Right, anchors[1]);
        self.set_anchor(Edge::Bottom, anchors[2]);
        self.set_anchor(Edge::Left, anchors[3]);
    }
}
