use crate::config::Config;
use crate::launcher::{self, AppEntry, EntryType};
use crate::search;
use gtk4::gdk::Key;
use gtk4::glib::{self, clone};
use gtk4::prelude::*;
use gtk4::ListBoxRow;
use gtk4::{Application, ApplicationWindow, Label, ListBox, ScrolledWindow, SearchEntry};
use gtk4::{Box as GtkBox, CssProvider, Orientation, STYLE_PROVIDER_PRIORITY_APPLICATION};
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

pub struct LauncherWindow {
    window: ApplicationWindow,
    search_entry: SearchEntry,
    results_list: ListBox,
    app_data_store: Rc<RefCell<Vec<AppEntry>>>,
}

impl LauncherWindow {
    pub fn new(app: &Application) -> Self {
        let config = Config::load();
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(config.width)
            .default_height(config.height)
            .title("HyprLauncher")
            .decorated(false)
            .resizable(false)
            .modal(true)
            .build();

        let main_box = GtkBox::new(Orientation::Vertical, 0);
        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Press / to start searching"));

        let focus_controller = gtk4::EventControllerFocus::new();
        focus_controller.connect_enter(clone!(@strong search_entry => move |_| {
            search_entry.set_placeholder_text(None);
        }));

        focus_controller.connect_leave(clone!(@strong search_entry => move |_| {
            search_entry.set_placeholder_text(Some("Press / to start searching"));
        }));

        search_entry.add_controller(focus_controller);

        let scrolled = ScrolledWindow::new();
        let results_list = ListBox::new();

        scrolled.set_vexpand(true);
        results_list.set_selection_mode(gtk4::SelectionMode::Single);

        scrolled.set_child(Some(&results_list));
        main_box.append(&search_entry);
        main_box.append(&scrolled);
        window.set_child(Some(&main_box));

        let css = CssProvider::new();
        css.load_from_data(&Config::load_css());

        let display = window.native().unwrap().display();
        gtk4::style_context_add_provider_for_display(
            &display,
            &css,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let launcher = Self {
            window,
            search_entry,
            results_list,
            app_data_store: Rc::new(RefCell::new(Vec::new())),
        };

        launcher.setup_signals();
        launcher.load_applications();
        launcher
    }

    fn setup_signals(&self) {
        let results_list = self.results_list.clone();
        let app_data_store = self.app_data_store.clone();
        let search_counter = Rc::new(RefCell::new(0u32));

        self.search_entry.connect_changed(
            clone!(@strong results_list, @strong app_data_store, @strong search_counter => move |entry| {
                let current_counter = {
                    let mut counter = search_counter.borrow_mut();
                    *counter = counter.wrapping_add(1);
                    *counter
                };

                let query = entry.text().to_string();
                glib::timeout_add_local(std::time::Duration::from_millis(150),
                    clone!(@strong results_list, @strong app_data_store, @strong search_counter => move || {
                        if current_counter != *search_counter.borrow() {
                            return glib::ControlFlow::Break;
                        }

                        let query = query.clone();
                        glib::spawn_future_local(clone!(@strong results_list, @strong app_data_store => async move {
                            let results = search::search_applications(&query).await;
                            update_results_list(&results_list, results, &app_data_store);
                        }));
                        glib::ControlFlow::Break
                    }),
                );
            }),
        );

        let search_controller = gtk4::EventControllerKey::new();
        search_controller.connect_key_pressed(clone!(@strong results_list => move |_, key, _, _| {
            match key {
                Key::Escape => {
                    if let Some(row) = results_list.first_child() {
                        if let Some(list_row) = row.downcast_ref::<ListBoxRow>() {
                            results_list.select_row(Some(list_row));
                            list_row.grab_focus();
                        }
                    }
                    glib::Propagation::Stop
                },
                _ => glib::Propagation::Proceed
            }
        }));
        self.search_entry.add_controller(search_controller);

        let window_controller = gtk4::EventControllerKey::new();
        window_controller.connect_key_pressed(clone!(@strong results_list,
            @strong self.window as window,
            @strong self.search_entry as search_entry,
            @strong app_data_store => move |_, key, _, _| {
            match key {
                Key::Escape => {
                    if search_entry.has_focus() {
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
                },
                Key::slash => {
                    search_entry.grab_focus();
                    glib::Propagation::Stop
                },
                Key::Up | Key::k => {
                    if !search_entry.has_focus() {
                        select_previous(&results_list);
                    }
                    glib::Propagation::Stop
                },
                Key::Down | Key::j => {
                    if !search_entry.has_focus() {
                        select_next(&results_list);
                    }
                    glib::Propagation::Stop
                },
                _ => glib::Propagation::Proceed
            }
        }));
        self.window.add_controller(window_controller);

        self.results_list
            .connect_row_activated(clone!(@strong self.window as window,
                @strong self.search_entry as search_entry,
                @strong app_data_store => move |_, row| {
                if let Some(app_data) = get_app_data(row.index() as usize, &app_data_store) {
                    if launch_application(&app_data, &search_entry) {
                        window.close();
                    }
                }
            }));

        self.search_entry.connect_activate(
            clone!(@strong results_list, @strong self.window as window,
                  @strong self.search_entry as search_entry,
                  @strong app_data_store => move |_| {
                if let Some(row) = results_list.selected_row() {
                    if let Some(app_data) = get_app_data(row.index() as usize, &app_data_store) {
                        if launch_application(&app_data, &search_entry) {
                            window.close();
                        }
                    }
                }
            }),
        );
    }

    fn load_applications(&self) {
        let results_list = self.results_list.clone();
        let app_data_store = self.app_data_store.clone();

        glib::spawn_future_local(
            clone!(@strong results_list, @strong app_data_store => async move {
                launcher::load_applications().await;
                let results = search::search_applications("").await;
                update_results_list(&results_list, results, &app_data_store);
            }),
        );
    }

    pub fn present(&self) {
        self.window.present();
        self.search_entry.grab_focus();
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

    for result in results {
        store.push(result.app.clone());
        let row = create_result_row(&result.app);
        list.append(&row);
    }

    if let Some(first_row) = list.row_at_index(0) {
        list.select_row(Some(&first_row));
    }
}

fn create_result_row(app: &AppEntry) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    let box_row = GtkBox::new(Orientation::Horizontal, 12);
    box_row.set_margin_start(12);
    box_row.set_margin_end(12);
    box_row.set_margin_top(8);
    box_row.set_margin_bottom(8);

    let icon = gtk4::Image::from_icon_name(&app.icon_name);
    icon.set_pixel_size(32);
    icon.set_margin_end(8);
    box_row.append(&icon);

    let text_box = GtkBox::new(Orientation::Vertical, 4);
    text_box.set_hexpand(true);

    let name_label = Label::new(Some(&app.name));
    name_label.set_halign(gtk4::Align::Start);
    name_label.set_wrap(true);
    name_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    name_label.set_max_width_chars(50);
    name_label.add_css_class("app-name");
    text_box.append(&name_label);

    if !app.description.is_empty() {
        let desc_label = Label::new(Some(&app.description));
        desc_label.set_halign(gtk4::Align::Start);
        desc_label.set_wrap(true);
        desc_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        desc_label.set_max_width_chars(50);
        desc_label.add_css_class("app-description");
        text_box.append(&desc_label);
    }

    let path_label = Label::new(Some(&app.path));
    path_label.set_halign(gtk4::Align::Start);
    path_label.set_wrap(true);
    path_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    path_label.set_max_width_chars(50);
    path_label.add_css_class("app-path");
    text_box.append(&path_label);

    box_row.append(&text_box);
    row.set_child(Some(&box_row));
    row
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

            glib::spawn_future_local(async move {
                let _ = Command::new("sh").arg("-c").arg(exec).spawn();
            });

            true
        }
        EntryType::File => {
            if app.icon_name == "folder" {
                let path = if app.path.ends_with('/') {
                    app.path.clone()
                } else {
                    format!("{}/", app.path)
                };
                search_entry.set_text(&path);
                search_entry.set_position(-1);

                false
            } else {
                let exec = app.exec.clone();
                glib::spawn_future_local(async move {
                    let _ = Command::new("sh").arg("-c").arg(exec).spawn();
                });

                true
            }
        }
    }
}
