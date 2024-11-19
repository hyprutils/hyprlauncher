use crate::{
    config::{Config, WindowAnchor},
    launcher::{self, AppEntry, EntryType},
    log, search,
};
use gtk4::{
    gdk::Key,
    gio,
    glib::{self},
    prelude::*,
    subclass::prelude::*,
    Application, ApplicationWindow, Box as GtkBox, CssProvider, Label, ListView, Orientation,
    ScrolledWindow, SearchEntry, SignalListItemFactory, SingleSelection,
    STYLE_PROVIDER_PRIORITY_USER,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{cell::RefCell, process::Command, rc::Rc};
use tokio::runtime::Handle;

pub struct LauncherWindow {
    window: ApplicationWindow,
    search_entry: SearchEntry,
    list_view: ListView,
    app_data_store: Rc<RefCell<Vec<AppEntry>>>,
    rt: Handle,
}

impl LauncherWindow {
    pub fn new(app: &Application, rt: Handle) -> Self {
        let window_start = std::time::Instant::now();
        log!(
            "Creating launcher window ({:.3}ms)",
            window_start.elapsed().as_secs_f64() * 1000.0
        );

        let search_start = std::time::Instant::now();
        let config = Config::load();
        let initial_results = rt.block_on(async { search::search_applications("", &config).await });
        log!(
            "Initial search population ({:.3}ms)",
            search_start.elapsed().as_secs_f64() * 1000.0
        );

        let window = ApplicationWindow::builder()
            .application(app)
            .title("HyprLauncher")
            .default_width(config.window.width)
            .default_height(config.window.height)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_keyboard_mode(if config.debug.disable_auto_focus {
            KeyboardMode::OnDemand
        } else {
            KeyboardMode::Exclusive
        });
        Self::setup_window_anchoring(&window, &config);
        Self::apply_window_margins(&window, &config);

        let main_box = GtkBox::new(Orientation::Vertical, 0);
        let search_entry = SearchEntry::new();
        let scrolled = ScrolledWindow::new();

        let model = gio::ListStore::new::<AppEntryObject>();
        let selection_model = SingleSelection::new(Some(model.clone()));
        let factory = SignalListItemFactory::new();
        let list_view = ListView::new(Some(selection_model.clone()), Some(factory.clone()));

        factory.connect_setup(move |_, list_item| {
            let config = Config::load();
            let box_row = GtkBox::builder()
                .orientation(Orientation::Horizontal)
                .spacing(12)
                .margin_start(12)
                .margin_end(12)
                .margin_top(6)
                .margin_bottom(6)
                .build();

            if config.window.show_icons {
                let icon = gtk4::Image::builder()
                    .icon_size(gtk4::IconSize::Large)
                    .build();
                box_row.append(&icon);
            }

            let text_box = GtkBox::builder()
                .orientation(Orientation::Vertical)
                .spacing(3)
                .build();

            let name_label = Label::builder()
                .halign(gtk4::Align::Start)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build();
            name_label.add_css_class("app-name");
            text_box.append(&name_label);

            if config.window.show_descriptions {
                let desc_label = Label::builder()
                    .halign(gtk4::Align::Start)
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .build();
                desc_label.add_css_class("app-description");
                text_box.append(&desc_label);
            }

            if config.window.show_paths {
                let path_label = Label::builder()
                    .halign(gtk4::Align::Start)
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .build();
                path_label.add_css_class("app-path");
                text_box.append(&path_label);
            }

            box_row.append(&text_box);
            list_item.set_child(Some(&box_row));
        });

        factory.connect_bind(move |_, list_item| {
            let config = Config::load();
            if let Some(app_entry) = list_item.item().and_downcast::<AppEntryObject>() {
                if let Some(box_row) = list_item.child().and_downcast::<GtkBox>() {
                    if config.window.show_icons {
                        if let Some(icon) = box_row.first_child().and_downcast::<gtk4::Image>() {
                            icon.set_icon_name(Some(app_entry.imp().icon_name()));
                        }
                    }

                    let text_box = box_row
                        .last_child()
                        .and_downcast::<GtkBox>()
                        .expect("Last child must be a GtkBox");

                    let name_label = text_box
                        .first_child()
                        .and_downcast::<Label>()
                        .expect("First child must be a Label");
                    name_label.set_text(app_entry.imp().name());

                    if config.window.show_descriptions {
                        let desc = app_entry.imp().description();
                        if !desc.is_empty() {
                            if let Some(desc_label) = text_box
                                .first_child()
                                .and_then(|w| w.next_sibling())
                                .and_downcast::<Label>()
                            {
                                desc_label.set_text(desc);
                                desc_label.set_visible(true);
                            }
                        } else if let Some(desc_label) = text_box
                            .first_child()
                            .and_then(|w| w.next_sibling())
                            .and_downcast::<Label>()
                        {
                            desc_label.set_visible(false);
                        }
                    }

                    if config.window.show_paths {
                        let path = app_entry.imp().path();
                        if !path.is_empty() {
                            let path_label = if config.window.show_descriptions {
                                text_box
                                    .first_child()
                                    .and_then(|w| w.next_sibling())
                                    .and_then(|w| w.next_sibling())
                            } else {
                                text_box.first_child().and_then(|w| w.next_sibling())
                            };

                            if let Some(path_label) = path_label.and_downcast::<Label>() {
                                path_label.set_text(path);
                                path_label.set_visible(true);
                            }
                        } else {
                            let path_label = if config.window.show_descriptions {
                                text_box
                                    .first_child()
                                    .and_then(|w| w.next_sibling())
                                    .and_then(|w| w.next_sibling())
                            } else {
                                text_box.first_child().and_then(|w| w.next_sibling())
                            };

                            if let Some(path_label) = path_label.and_downcast::<Label>() {
                                path_label.set_visible(false);
                            }
                        }
                    }
                }
            }
        });

        scrolled.set_vexpand(true);
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::External);
        list_view.set_single_click_activate(true);

        scrolled.set_child(Some(&list_view));
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
                STYLE_PROVIDER_PRIORITY_USER,
            );
        }
        log!(
            "CSS loading and application ({:.3}ms)",
            css_start.elapsed().as_secs_f64() * 1000.0
        );

        let app_data_store = Rc::new(RefCell::new(Vec::with_capacity(50)));
        update_results_list(&list_view, initial_results.unwrap(), &app_data_store);

        let launcher = Self {
            window,
            search_entry,
            list_view,
            app_data_store,
            rt: rt.clone(),
        };

        launcher.setup_signals();
        launcher
    }

    pub fn present(&self) {
        let present_start = std::time::Instant::now();
        log!(
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
            let list_view_for_key = self.list_view.clone();

            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, key, _, _| match key {
                Key::Up => {
                    select_previous(&list_view_for_key);
                    glib::Propagation::Stop
                }
                Key::Down => {
                    select_next(&list_view_for_key);
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            });
            search_entry_for_controller.add_controller(key_controller);

            let focus_controller = gtk4::EventControllerFocus::new();

            focus_controller.connect_enter(move |_| {
                search_entry_for_enter.set_placeholder_text(None);
            });

            focus_controller.connect_leave(move |_| {
                search_entry_for_leave.set_placeholder_text(Some("Press / to start searching"));
            });

            search_entry_for_controller.add_controller(focus_controller);

            let list_view_for_search = self.list_view.clone();
            let app_data_store_for_search = self.app_data_store.clone();
            let rt_handle = self.rt.clone();

            self.search_entry.connect_changed(move |entry| {
                let query = entry.text().to_string();
                let list_view = list_view_for_search.clone();
                let app_data_store = app_data_store_for_search.clone();
                let rt_handle = rt_handle.clone();

                glib::MainContext::default().spawn_local(async move {
                    let config = Config::load();
                    let results = rt_handle
                        .spawn(async move { search::search_applications(&query, &config).await })
                        .await
                        .unwrap()
                        .unwrap_or_default();
                    update_results_list(&list_view, results, &app_data_store);
                });
            });

            let window_for_search = self.window.clone();
            let search_entry_for_search = self.search_entry.clone();
            let search_controller = gtk4::EventControllerKey::new();

            search_controller.connect_key_pressed(move |_, key, _, _| {
                let window = window_for_search.clone();
                let search_entry = search_entry_for_search.clone();

                match key {
                    Key::Escape => {
                        search_entry.set_text("");
                        window.hide();
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            });
            self.search_entry.add_controller(search_controller);
        }

        let list_view_for_window = self.list_view.clone();
        let window_for_window = self.window.clone();
        let search_entry_for_window = self.search_entry.clone();

        let window_controller = gtk4::EventControllerKey::new();
        window_controller.connect_key_pressed(move |_, key, _, _| {
            let config = Config::load();
            let list_view = list_view_for_window.clone();
            let window = window_for_window.clone();
            let search_entry = search_entry_for_window.clone();

            match key.name().as_deref() {
                Some(key_name) => {
                    if key_name == config.window.custom_navigate_keys.up {
                        select_previous(&list_view);
                        glib::Propagation::Stop
                    } else if key_name == config.window.custom_navigate_keys.down {
                        select_next(&list_view);
                        glib::Propagation::Stop
                    } else if key_name == config.window.custom_navigate_keys.delete_word {
                        let text = search_entry.text();
                        let cursor_pos = search_entry.position() as usize;
                        if let Some((new_text, new_pos)) = delete_word(&text, cursor_pos) {
                            search_entry.set_text(&new_text);
                            search_entry.set_position(new_pos as i32);
                        }
                        glib::Propagation::Stop
                    } else {
                        match key {
                            Key::Escape => {
                                window.hide();
                                glib::Propagation::Stop
                            }
                            _ => glib::Propagation::Proceed,
                        }
                    }
                }
                None => glib::Propagation::Proceed,
            }
        });
        self.window.add_controller(window_controller);

        let window_for_row = self.window.clone();
        let search_entry_for_row = self.search_entry.clone();

        self.list_view.connect_activate(move |list_view, position| {
            if let Some(model) = list_view.model() {
                if let Some(item) = model.item(position) {
                    if let Some(app_entry) = item.downcast_ref::<AppEntryObject>() {
                        if launch_application(app_entry.imp().app_entry(), &search_entry_for_row) {
                            window_for_row.hide();
                        }
                    }
                }
            }
        });

        let list_view_for_activate = self.list_view.clone();
        let window_for_activate = self.window.clone();
        let search_entry_for_activate = self.search_entry.clone();

        self.search_entry.connect_activate(move |_| {
            if let Some(selected) = get_selected_item(&list_view_for_activate) {
                if let Some(app_entry) = selected.downcast_ref::<AppEntryObject>() {
                    if launch_application(app_entry.imp().app_entry(), &search_entry_for_activate) {
                        window_for_activate.hide();
                    }
                }
            }
        });

        let search_entry_for_hide = self.search_entry.clone();
        self.window.connect_hide(move |_| {
            search_entry_for_hide.set_text("");
            search_entry_for_hide.grab_focus();
        });
    }

    pub fn update_window_config(window: &ApplicationWindow, config: &Config) {
        window.set_default_width(config.window.width);
        window.set_default_height(config.window.height);
        window.set_keyboard_mode(if config.debug.disable_auto_focus {
            KeyboardMode::OnDemand
        } else {
            KeyboardMode::Exclusive
        });

        Self::setup_window_anchoring(window, config);
        Self::apply_window_margins(window, config);

        if let Some(native) = window.native() {
            let css_provider = CssProvider::new();
            css_provider.load_from_data(&config.get_css());
            gtk4::style_context_add_provider_for_display(
                &native.display(),
                &css_provider,
                STYLE_PROVIDER_PRIORITY_USER,
            );
        }

        if let Some(main_box) = window.first_child() {
            if let Some(main_box) = main_box.downcast_ref::<gtk4::Box>() {
                let search_entry = main_box
                    .first_child()
                    .and_then(|child| child.downcast::<gtk4::SearchEntry>().ok());

                if let Some(search_entry) = search_entry {
                    if config.window.show_search {
                        search_entry.set_visible(true);
                        search_entry.set_text("__config_reload__");
                        search_entry.set_text("");
                    } else {
                        search_entry.set_visible(false);
                    }
                }

                if let Some(scrolled) = main_box.last_child().and_downcast::<ScrolledWindow>() {
                    if let Some(list_view) = scrolled.child().and_downcast::<ListView>() {
                        if let Some(selection_model) =
                            list_view.model().and_downcast::<SingleSelection>()
                        {
                            if let Some(model) =
                                selection_model.model().and_downcast::<gio::ListStore>()
                            {
                                let items: Vec<_> =
                                    (0..model.n_items()).filter_map(|i| model.item(i)).collect();
                                model.remove_all();
                                for item in items {
                                    model.append(&item);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn update_results_list(
    list_view: &ListView,
    results: Vec<search::SearchResult>,
    store: &Rc<RefCell<Vec<AppEntry>>>,
) {
    if let Some(selection_model) = list_view.model().and_downcast::<SingleSelection>() {
        if let Some(model) = selection_model.model().and_downcast::<gio::ListStore>() {
            let config = Config::load();
            let max_entries = config.window.max_entries;
            let mut store = store.borrow_mut();

            model.remove_all();
            store.clear();
            store.reserve(max_entries);

            let results = if results.len() > max_entries {
                &results[..max_entries]
            } else {
                &results
            };

            store.extend(results.iter().map(|r| r.app.clone()));
            model.extend_from_slice(
                &results
                    .iter()
                    .map(|r| AppEntryObject::new(r.app.clone()))
                    .collect::<Vec<_>>(),
            );
        }
    }
}

fn select_next(list_view: &ListView) {
    if let Some(selection_model) = list_view.model().and_downcast::<SingleSelection>() {
        let n_items = selection_model.n_items();
        if n_items == 0 {
            return;
        }
        let current_pos = selection_model.selected();
        if current_pos < n_items - 1 {
            let next_pos = current_pos + 1;
            selection_model.set_selected(next_pos);
            list_view
                .activate_action("list.scroll-to-item", Some(&next_pos.to_variant()))
                .unwrap_or_default();
        }
    }
}

fn select_previous(list_view: &ListView) {
    if let Some(selection_model) = list_view.model().and_downcast::<SingleSelection>() {
        let n_items = selection_model.n_items();
        if n_items == 0 {
            return;
        }
        let current_pos = selection_model.selected();
        if current_pos > 0 {
            let prev_pos = current_pos - 1;
            selection_model.set_selected(prev_pos);
            list_view
                .activate_action("list.scroll-to-item", Some(&prev_pos.to_variant()))
                .unwrap_or_default();
        }
    }
}

fn launch_application(app: &AppEntry, search_entry: &SearchEntry) -> bool {
    match app.entry_type {
        EntryType::Application => {
            log!("Launching application: {}", app.name);
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

            launcher::increment_launch_count(app).unwrap();

            Command::new("sh").arg("-c").arg(&exec).spawn().is_ok()
        }
        EntryType::File => {
            if app.icon_name == "folder" {
                log!("Opening folder: {}", app.path);
                let path = if app.path.ends_with('/') {
                    app.path.clone()
                } else {
                    format!("{}/", app.path)
                };
                search_entry.set_text(&path);
                search_entry.set_position(-1);

                false
            } else {
                log!("Opening file: {}", app.path);
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

glib::wrapper! {
    pub struct AppEntryObject(ObjectSubclass<imp::AppEntryObject>);
}

mod imp {
    use super::*;
    use once_cell::sync::OnceCell;

    #[derive(Default)]
    pub struct AppEntryObject {
        pub(crate) name: OnceCell<String>,
        pub(crate) description: OnceCell<String>,
        pub(crate) path: OnceCell<String>,
        pub(crate) icon_name: OnceCell<String>,
        pub(crate) app_entry: OnceCell<AppEntry>,
    }

    impl AppEntryObject {
        pub fn name(&self) -> &str {
            self.name.get().unwrap()
        }

        pub fn description(&self) -> &str {
            self.description.get().unwrap()
        }

        pub fn path(&self) -> &str {
            self.path.get().unwrap()
        }

        pub fn icon_name(&self) -> &str {
            self.icon_name.get().unwrap()
        }

        pub fn app_entry(&self) -> &AppEntry {
            self.app_entry.get().unwrap()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppEntryObject {
        const NAME: &'static str = "AppEntryObject";
        type Type = super::AppEntryObject;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for AppEntryObject {}
}

impl AppEntryObject {
    pub fn new(app_entry: AppEntry) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();
        imp.name.set(app_entry.name.clone()).unwrap();
        imp.description.set(app_entry.description.clone()).unwrap();
        imp.path.set(app_entry.path.clone()).unwrap();
        imp.icon_name.set(app_entry.icon_name.clone()).unwrap();
        imp.app_entry.set(app_entry).unwrap();
        obj
    }
}

fn get_selected_item(list_view: &ListView) -> Option<AppEntryObject> {
    list_view
        .model()
        .and_downcast::<SingleSelection>()
        .and_then(|selection| {
            let position = selection.selected();
            selection
                .model()
                .and_then(|model| model.item(position))
                .and_downcast::<AppEntryObject>()
        })
}

fn delete_word(text: &str, cursor_pos: usize) -> Option<(String, usize)> {
    if text.is_empty() || cursor_pos == 0 {
        return None;
    }

    let mut chars: Vec<char> = text.chars().collect();
    let mut new_pos = cursor_pos;

    while new_pos > 0 && chars[new_pos - 1].is_whitespace() {
        new_pos -= 1;
    }

    while new_pos > 0 && !chars[new_pos - 1].is_whitespace() {
        new_pos -= 1;
    }

    chars.drain(new_pos..cursor_pos);
    let result: String = chars.into_iter().collect();
    let trimmed = result.trim_end().to_string();
    let new_pos = new_pos.min(trimmed.len());

    Some((trimmed, new_pos))
}
