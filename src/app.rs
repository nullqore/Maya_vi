use arboard::Clipboard;
use eframe::egui;
use sled::Db;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use url::Url;

use crate::file_processing::{self, Progress};
use crate::file_saver::{self, SaveProgress};
use crate::network::{self, NetworkResult};
use crate::syntax_highlighter::CodeTheme;

#[derive(Default, Clone, PartialEq)]
enum AppMode {
    #[default]
    Main,
    FilePicker,
}

enum Action {
    Select(Vec<String>),
    Delete(Vec<String>),
    Copy(String),
    SendRequest(String),
    ShowSaveDialog,
    SaveToFile(String),
    SendToProxy(String),
    ShowProxyWindow,
    SaveDisplayedUrls,
    SendDisplayedUrlsToProxy(u32),
    ShowThreadWindow,
}

#[derive(Clone, Default)]
enum RightPanelView {
    #[default]
    Empty,
    Loading,
    Response(String, String, String, String, String, bool, u64, u128),
}


pub struct SiteMapperApp {
    app_mode: AppMode,
    db: Option<Arc<Db>>,
    file_receiver: Option<Receiver<Progress>>,
    save_receiver: Option<Receiver<SaveProgress>>,
    is_loading_file: bool,
    is_saving_file: bool,
    progress: f32,
    time_remaining: Option<std::time::Duration>,
    total_url_count: usize,
    selected_path: Option<Vec<String>>,
    error_message: Option<String>,
    file_picker_path: PathBuf,
    file_picker_error: Option<String>,
    clipboard: Option<Clipboard>,
    right_panel_view: RightPanelView,
    network_receiver: Option<Receiver<NetworkResult>>,
    highlighter: CodeTheme,
    show_save_dialog: bool,
    save_file_name: String,
    proxy_address: String,
    proxy_receiver: Option<Receiver<Result<(), String>>>,
    proxy_progress_receiver: Option<Receiver<crate::proxy::ProxyProgress>>,
    show_proxy_window: bool,
    proxy_protocol: String,
    proxy_ip: String,
    proxy_port: String,
    proxy_threads: u32,
    show_thread_window: bool,
    action_sender: std::sync::mpsc::Sender<Action>,
    action_receiver: std::sync::mpsc::Receiver<Action>,
}

impl Default for SiteMapperApp {
    fn default() -> Self {
        let (action_sender, action_receiver) = std::sync::mpsc::channel();
        Self {
            app_mode: AppMode::default(),
            db: None,
            file_receiver: None,
            save_receiver: None,
            is_loading_file: false,
            is_saving_file: false,
            progress: 0.0,
            time_remaining: None,
            total_url_count: 0,
            selected_path: None,
            error_message: None,
            file_picker_path: env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            file_picker_error: None,
            clipboard: Clipboard::new().ok(),
            right_panel_view: RightPanelView::default(),
            network_receiver: None,
            highlighter: CodeTheme::default(),
            show_save_dialog: false,
            save_file_name: "sitemap.txt".to_string(),
            proxy_address: "http://127.0.0.1:8080".to_string(),
            proxy_receiver: None,
            proxy_progress_receiver: None,
            show_proxy_window: false,
            proxy_protocol: "http".to_string(),
            proxy_ip: "127.0.0.1".to_string(),
            proxy_port: "8080".to_string(),
            proxy_threads: 1,
            show_thread_window: false,
            action_sender,
            action_receiver,
        }
    }
}


impl eframe::App for SiteMapperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_file_receiver(ctx);
        self.handle_save_receiver(ctx);
        self.handle_network_receiver(ctx);
        self.handle_proxy_receiver(ctx);
        self.handle_proxy_progress_receiver(ctx);

        let current_mode = self.app_mode.clone();
        match current_mode {
            AppMode::Main => {
                self.draw_main_ui(ctx, true);
            }
            AppMode::FilePicker => {
                self.draw_main_ui(ctx, false);
                self.show_file_picker_window(ctx);
            }
        }

        if self.show_save_dialog {
            self.show_save_dialog(ctx);
        }

        if self.show_proxy_window {
            self.show_proxy_window(ctx);
        }

        if self.show_thread_window {
            self.show_thread_window(ctx);
        }

        if let Ok(action) = self.action_receiver.try_recv() {
            self.execute_action(action);
        }
    }
}


impl SiteMapperApp {
    fn draw_main_ui(&mut self, ctx: &egui::Context, is_enabled: bool) {
        let top_action = self.show_top_panel(ctx, is_enabled);
        let sitemap_action = self.show_sitemap_panel(ctx, is_enabled);
        self.show_bottom_panel(ctx, is_enabled);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(is_enabled);
            let top_panel_height = ui.available_height() * 0.33;
            ui.group(|ui| {
                ui.set_height(top_panel_height);
                if let (Some(selected_path), Some(db)) = (&self.selected_path, &self.db) {
                    let key = selected_path.join("/");
                    if let Some(_node_value) = file_processing::get_node_value(db, &key) {
                        let mut all_children = Vec::new();
                        get_all_children(db, &key, &mut all_children);
                        all_children.sort(); 
                        if all_children.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label("No endpoints in this node.");
                            });
                        } else {
                            egui::ScrollArea::both()
                                .id_source("endpoints_scroll")
                                .auto_shrink([false, false])
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                )
                                .show(ui, |ui| {
                                    let grid = egui::Grid::new(selected_path.join("/"));
                                    grid.num_columns(3)
                                        .striped(true)
                                        .min_col_width(100.0)
                                        .max_col_width(1150.0)
                                        .show(ui, |ui| {
                                            ui.label(format!("URL ({})", all_children.len()));
                                            ui.set_min_width(100.0);
                                            ui.label("Extension");
                                            ui.set_min_width(100.0);
                                            ui.label("Parameters");
                                            ui.end_row();

                                            
                                            for endpoint in &all_children {
                                                let full_url = endpoint.to_string();
                                                let extension = self
                                                    .get_extension_from_url(&full_url)
                                                    .unwrap_or("");
                                                let params = self
                                                    .get_parameters_from_url(&full_url);

                                                let response = ui.add(egui::SelectableLabel::new(false, &full_url));
                                                response.context_menu(|ui| {
                                                    if ui.button("Send Request").clicked() {
                                                        let _ = self.action_sender.send(Action::SendRequest(full_url.clone()));
                                                        ui.close_menu();
                                                    }
                                                    if ui.button("Copy URL").clicked() {
                                                        let _ = self.action_sender.send(Action::Copy(full_url.clone()));
                                                        ui.close_menu();
                                                    }
                                                    if ui.button("Delete").clicked() {
                                                        let _ = self.action_sender.send(Action::Delete(vec![endpoint.clone()]));
                                                        ui.close_menu();
                                                    }
                                                    if ui.button("Send to Proxy").clicked() {
                                                        let _ = self.action_sender.send(Action::SendToProxy(full_url.clone()));
                                                        ui.close_menu();
                                                    }
                                                });

                                                ui.label(extension);
                                                if !params.is_empty() {
                                                    ui.label("✔");
                                                } else {
                                                    ui.label("");
                                                }
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Select a node to see its endpoints");
                        });
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Select a node to see its endpoints");
                    });
                }
            });

            ui.add_space(10.0);

            ui.group(|ui| {
                match &mut self.right_panel_view {
                    RightPanelView::Empty => {
                        ui.centered_and_justified(|ui| {
                            ui.label("Select an endpoint and click 'Send Request'");
                        });
                    }
                    RightPanelView::Loading => {
                        ui.centered_and_justified(|ui| {
                            ui.spinner();
                            ui.label("Fetching response...");
                        });
                    }
                    RightPanelView::Response(
                        request,
                        headers,
                        raw_body,
                        pretty_body,
                        language,
                        is_pretty,
                        content_length,
                        elapsed_ms,
                    ) => {
                        ui.heading("Request");
                        egui::ScrollArea::vertical()
                            .id_source("request_scroll")
                            .show(ui, |ui| {
                                ui.code(request);
                            });
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.heading("Response");
                            if *language == "json" {
                                if ui.button("Beautify").clicked() {
                                    if let Ok(json) =
                                        serde_json::from_str::<serde_json::Value>(raw_body)
                                    {
                                        if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                                            *pretty_body = pretty;
                                        }
                                    }
                                }
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.selectable_label(*is_pretty, "Pretty").clicked() {
                                    *is_pretty = true;
                                }
                                if ui.selectable_label(!*is_pretty, "Raw").clicked() {
                                    *is_pretty = false;
                                }
                            });
                        });
                        egui::ScrollArea::vertical()
                            .id_source("response_scroll")
                            .show(ui, |ui| {
                                ui.code(headers);
                                let body_to_show = if *is_pretty { pretty_body } else { raw_body };
                                let job = self.highlighter.highlight(ui, language, body_to_show);
                                ui.label(job);
                            });
                        ui.separator();
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{} bytes | {} ms", content_length, elapsed_ms));
                        });
                    }
                }
            });
        });

        if let Some(action) = top_action.or(sitemap_action) {
            self.execute_action(action);
        }
    }

    fn show_sitemap_panel(&mut self, ctx: &egui::Context, is_enabled: bool) -> Option<Action> {
        egui::SidePanel::left("sitemap_panel")
            .default_width(ctx.available_rect().width() / 4.0)
            .show(ctx, |ui| {
                ui.set_enabled(is_enabled);
                ui.heading(format!("Sitemap ({} URLs)", self.total_url_count));
                ui.separator();

                if let Some(db) = self.db.clone() {
                    egui::ScrollArea::vertical()
                        .id_source("sitemap_scroll")
                        .show(ui, |ui| {
                            let mut path = Vec::new();
                            self.show_db_tree(ui, &mut path, &db, "__ROOT__")
                        })
                        .inner
                } else {
                    None
                }
            })
            .inner
    }

    fn handle_proxy_receiver(&mut self, _ctx: &egui::Context) {
        if let Some(receiver) = &self.proxy_receiver {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(_) => {
                        self.error_message =
                            Some("Request sent to proxy successfully.".to_string());
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to send to proxy: {}", e));
                    }
                }
                self.proxy_receiver = None;
            }
        }
    }

    fn handle_proxy_progress_receiver(&mut self, _ctx: &egui::Context) {
        if let Some(receiver) = &self.proxy_progress_receiver {
            if let Ok(progress) = receiver.try_recv() {
                match progress {
                    crate::proxy::ProxyProgress::Advanced(percent) => {
                        self.progress = percent;
                    }
                    crate::proxy::ProxyProgress::Finished => {
                        self.proxy_progress_receiver = None;
                        self.error_message = Some("Sent all URLs to proxy.".to_string());
                    }
                    crate::proxy::ProxyProgress::Errored(err) => {
                        self.proxy_progress_receiver = None;
                        self.error_message = Some(format!("Failed to send to proxy: {}", err));
                    }
                }
            }
        }
    }

    fn handle_network_receiver(&mut self, _ctx: &egui::Context) {
        if let Some(receiver) = &self.network_receiver {
            if let Ok((
                request,
                headers,
                raw_body,
                pretty_body,
                language,
                content_length,
                elapsed_ms,
            )) = receiver.try_recv()
            {
                self.right_panel_view = RightPanelView::Response(
                    request,
                    headers,
                    raw_body,
                    pretty_body,
                    language,
                    true,
                    content_length,
                    elapsed_ms,
                );
                self.network_receiver = None;
            }
        }
    }

    fn handle_save_receiver(&mut self, _ctx: &egui::Context) {
        if let Some(receiver) = &self.save_receiver {
            if let Ok(progress) = receiver.try_recv() {
                match progress {
                    SaveProgress::Finished => {
                        self.is_saving_file = false;
                        self.error_message = Some("File saved successfully.".to_string());
                    }
                    SaveProgress::Errored(err) => {
                        self.is_saving_file = false;
                        self.error_message = Some(format!("Failed to save file: {}", err));
                    }
                }
                self.save_receiver = None;
            }
        }
    }

    fn handle_file_receiver(&mut self, _ctx: &egui::Context) {
        if let Some(receiver) = &self.file_receiver {
            if let Ok(progress) = receiver.try_recv() {
                match progress {
                    Progress::Advanced(percent, time, count) => {
                        self.progress = percent;
                        self.time_remaining = time;
                        self.total_url_count = count;
                    }
                    Progress::Finished(db, count) => {
                        self.db = Some(db);
                        self.total_url_count = count;
                        self.is_loading_file = false;
                    }
                    Progress::Errored(err) => {
                        self.error_message = Some(err);
                        self.is_loading_file = false;
                    }
                }
            }
        }
    }

    fn execute_action(&mut self, action: Action) {
        match action {
            Action::Select(path) => {
                self.selected_path = Some(path);
            }
            Action::Delete(path) => {
                if let Some(db) = &self.db {
                    match delete_node_from_db(db, &path) {
                        Ok(deleted_count) => {
                            self.total_url_count -= deleted_count;
                            self.error_message = Some(format!("Deleted {} URLs.", deleted_count));
                            if self.selected_path.as_ref() == Some(&path) {
                                self.selected_path = None;
                            }
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Failed to delete: {}", e));
                        }
                    }
                }
            }
            Action::Copy(url) => {
                if let Some(clipboard) = &mut self.clipboard {
                    if let Err(err) = clipboard.set_text(url) {
                        self.error_message = Some(format!("Failed to copy URL: {}", err));
                    }
                }
            }
            Action::SendRequest(url) => {
                self.right_panel_view = RightPanelView::Loading;
                self.network_receiver = Some(network::spawn_request_thread(url));
            }
            Action::ShowSaveDialog => {
                self.show_save_dialog = true;
            }
            Action::SaveToFile(file_name) => {
                if let Some(db) = &self.db {
                    self.is_saving_file = true;
                    self.error_message = None;
                    let path = PathBuf::from(file_name);
                    self.save_receiver = Some(file_saver::spawn_file_saving_thread(Arc::clone(db), path));
                }
                self.show_save_dialog = false;
            }
            Action::SendToProxy(url) => {
                let proxy_address = self.proxy_address.clone();
                let (sender, receiver) = std::sync::mpsc::channel();
                self.proxy_receiver = Some(receiver);
                self.error_message = Some("Sending to proxy...".to_string());
                std::thread::spawn(move || {
                    let result = crate::proxy::send_to_proxy(&url, &proxy_address);
                    let _ = sender.send(result);
                });
            }
            Action::ShowProxyWindow => {
                self.show_proxy_window = true;
            }
            Action::ShowThreadWindow => {
                self.show_thread_window = true;
            }
            Action::SaveDisplayedUrls => {
                if let (Some(selected_path), Some(db)) = (&self.selected_path, &self.db) {
                    let key = selected_path.join("/");
                    let mut all_children = Vec::new();
                    get_all_children(db, &key, &mut all_children);
                    all_children.sort();

                    let mut file_name = self.save_file_name.clone();
                    if !file_name.ends_with(".txt") {
                        file_name.push_str(".txt");
                    }

                    match std::fs::File::create(&file_name) {
                        Ok(mut file) => {
                            for url in all_children {
                                if let Err(e) = writeln!(file, "{}", url) {
                                    self.error_message = Some(format!("Failed to write to file: {}", e));
                                    return;
                                }
                            }
                            self.error_message = Some(format!("Saved to {}", file_name));
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Failed to create file: {}", e));
                        }
                    }
                }
            }
            Action::SendDisplayedUrlsToProxy(threads) => {
                if let (Some(selected_path), Some(db)) = (&self.selected_path, &self.db) {
                    let key = selected_path.join("/");
                    let mut all_children = Vec::new();
                    get_all_children(db, &key, &mut all_children);

                    self.proxy_progress_receiver = Some(crate::proxy::spawn_proxy_thread(
                        all_children,
                        self.proxy_address.clone(),
                        threads,
                    ));
                }
            }
        }
    }

    fn get_parameters_from_url(&self, url_str: &str) -> String {
        if let Ok(url) = Url::parse(url_str) {
            if let Some(query) = url.query() {
                return query.to_string();
            }
        }
        String::new()
    }

    fn get_extension_from_url<'a>(&self, url: &'a str) -> Option<&'a str> {
        let path = url.split('/').last().unwrap_or("");
        if !path.contains('.') {
            return None;
        }
        let parts: Vec<&str> = path.split('.').collect();
        if parts.len() > 1 {
            let ext = parts.last().unwrap_or(&"");
            if ext.contains('?') {
                let ext_parts: Vec<&str> = ext.split('?').collect();
                return Some(ext_parts[0]);
            }
            if ext.contains('#') {
                let ext_parts: Vec<&str> = ext.split('#').collect();
                return Some(ext_parts[0]);
            }
            return Some(ext);
        }
        None
    }

    fn start_file_processing(&mut self, path: PathBuf) {
        self.is_loading_file = true;
        self.error_message = None;
        self.progress = 0.0;
        self.total_url_count = 0;
        self.db = None; 
        self.file_receiver = Some(file_processing::spawn_file_processing_thread(path));
        self.app_mode = AppMode::Main;
    }

    
    fn show_top_panel(&mut self, ctx: &egui::Context, is_enabled: bool) -> Option<Action> {
        let mut action = None;
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.set_enabled(is_enabled);
            ui.horizontal(|ui| {
                if ui.button("Load URL File").clicked() {
                    self.app_mode = AppMode::FilePicker;
                    self.file_picker_path =
                        env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
                }

                if ui.add_enabled(self.db.is_some(), egui::Button::new("Save")).clicked() {
                    action = Some(Action::SaveDisplayedUrls);
                }

                if ui.add_enabled(self.db.is_some(), egui::Button::new("Send to Proxy")).clicked() {
                    action = Some(Action::SendDisplayedUrlsToProxy(self.proxy_threads));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_enabled(self.db.is_some(), egui::Button::new("Save All")).clicked() {
                        action = Some(Action::ShowSaveDialog);
                    }
                    if ui.button("Setup Proxy").clicked() {
                        action = Some(Action::ShowProxyWindow);
                    }
                    ui.label(format!("Proxy: {}", self.proxy_address));
                    if ui.button("Set Thread").clicked() {
                        action = Some(Action::ShowThreadWindow);
                    }
                    ui.label("Threads:");
                    ui.add_enabled(false, egui::DragValue::new(&mut self.proxy_threads).speed(1));
                });
            });
        });
        action
    }

    fn show_bottom_panel(&mut self, ctx: &egui::Context, is_enabled: bool) {
        if self.is_loading_file || self.is_saving_file || self.proxy_progress_receiver.is_some() || self.error_message.is_some() {
            egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                ui.set_enabled(is_enabled);
                ui.vertical(|ui| {
                    if let Some(err) = &self.error_message {
                        ui.colored_label(ui.visuals().error_fg_color, err);
                    }

                    if self.is_loading_file {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(time) = self.time_remaining {
                                ui.label(format!("{:.0}s remaining", time.as_secs_f32()));
                            }
                            ui.add(egui::ProgressBar::new(self.progress / 100.0).show_percentage());
                        });
                    } else if self.is_saving_file {
                         ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spinner();
                            ui.label("Saving file...");
                        });
                    } else if self.proxy_progress_receiver.is_some() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(egui::ProgressBar::new(self.progress / 100.0).show_percentage());
                            ui.label("Sending to proxy...");
                        });
                    }
                });
            });
        }
    }

    fn show_save_dialog(&mut self, ctx: &egui::Context) {
        let mut action = None;
        egui::Window::new("Save All URLs")
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("File name:");
                    ui.text_edit_singleline(&mut self.save_file_name);
                });
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        action = Some(Action::SaveToFile(self.save_file_name.clone()));
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_save_dialog = false;
                    }
                });
            });

        if let Some(action) = action {
            self.execute_action(action);
        }
    }

    fn show_proxy_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Setup Proxy")
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Protocol:");
                    ui.text_edit_singleline(&mut self.proxy_protocol);
                });
                ui.horizontal(|ui| {
                    ui.label("IP Address:");
                    ui.text_edit_singleline(&mut self.proxy_ip);
                });
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.proxy_port);
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        self.proxy_address = format!(
                            "{}://{}:{}",
                            self.proxy_protocol, self.proxy_ip, self.proxy_port
                        );
                        self.show_proxy_window = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_proxy_window = false;
                    }
                });
            });
    }

    fn show_thread_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Set Threads")
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Threads:");
                    ui.add(egui::DragValue::new(&mut self.proxy_threads).speed(1));
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        self.show_thread_window = false;
                    }
                });
            });
    }

    fn show_file_picker_window(&mut self, ctx: &egui::Context) {
        let mut is_open = true;
        let mut file_to_load: Option<PathBuf> = None;

        egui::Window::new("File Picker")
            .open(&mut is_open)
            .vscroll(false)
            .resizable(true)
            .default_width(400.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                ui.label(format!("Current Path: {}", self.file_picker_path.display()));
                if let Some(err) = &self.file_picker_error {
                    ui.colored_label(ui.visuals().error_fg_color, err);
                }
                ui.separator();

                if ui.button("⬆ Up").clicked() {
                    if let Some(parent) = self.file_picker_path.parent() {
                        self.file_picker_path = parent.to_path_buf();
                    }
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    match fs::read_dir(&self.file_picker_path) {
                        Ok(entries) => {
                            self.file_picker_error = None;
                            let mut files = Vec::new();
                            let mut dirs = Vec::new();
                            for entry in entries.flatten() {
                                if let Ok(meta) = entry.metadata() {
                                    if meta.is_dir() {
                                        dirs.push(entry);
                                    } else {
                                        files.push(entry);
                                    }
                                }
                            }
                            dirs.sort_by_key(|a| a.file_name());
                            files.sort_by_key(|a| a.file_name());

                            for dir in dirs {
                                let name = dir.file_name().to_string_lossy().to_string();
                                if ui.button(format!("📁 {}", name)).clicked() {
                                    self.file_picker_path.push(name);
                                }
                            }
                            for file in files {
                                let name = file.file_name().to_string_lossy().to_string();
                                let is_selectable =
                                    name.ends_with(".txt") || name.ends_with(".list");
                                if ui
                                    .add_enabled(is_selectable, egui::Button::new(format!("📄 {}", name)))
                                    .clicked()
                                {
                                    file_to_load = Some(file.path());
                                }
                            }
                        }
                        Err(e) => self.file_picker_error = Some(e.to_string()),
                    }
                });
            });

        if !is_open {
            self.app_mode = AppMode::Main;
        }
        if let Some(path) = file_to_load {
            self.start_file_processing(path);
        }
    }

    fn show_db_tree(
        &mut self,
        ui: &mut egui::Ui,
        current_path: &mut Vec<String>,
        db: &Db,
        key: &str,
    ) -> Option<Action> {
        let mut children = file_processing::get_children(db, key);
        children.sort();

        for name in children {
            current_path.push(name.clone());

            let response = ui.scope(|ui| {
                let mut requested_action = None;
                let path_clone = current_path.clone();
                let new_key = path_clone.join("/");
                let has_children = file_processing::get_node_value(db, &new_key)
                    .map_or(false, |v| !v.children.is_empty());

                let is_selected = self.selected_path.as_ref() == Some(current_path);

                let response = if has_children {
                    let (icon, mut color) = ("📁", egui::Color32::from_rgb(255, 215, 100));

                    if key == "__ROOT__" {
                        if let Some(node) = file_processing::get_node_value(db, &new_key) {
                            if let Some(scheme) = &node.scheme {
                                if scheme == "http" {
                                    color = egui::Color32::from_rgb(255, 180, 180); // Light red
                                } else if scheme == "https" {
                                    color = egui::Color32::from_rgb(180, 255, 180); // Light green
                                }
                            }
                        }
                    }

                    let label = format!("{} {}", icon, name);
                    let mut rich_text = egui::RichText::new(label).size(14.0).color(color);
                    if is_selected {
                        rich_text = rich_text.background_color(ui.visuals().selection.bg_fill);
                    }

                    let header = egui::CollapsingHeader::new(rich_text)
                        .default_open(false)
                        .show(ui, |ui| {
                            self.show_db_tree(ui, current_path, db, &new_key)
                        });

                    if let Some(Some(inner_action)) = header.body_returned {
                        requested_action = Some(inner_action);
                    }
                    header.header_response
                } else {
                    let extension = get_extension(&name);
                    let (icon, color) = match extension {
                        Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "webp") => {
                            ("🖼️", egui::Color32::from_rgb(200, 120, 255))
                        }
                        Some("js" | "css" | "json" | "xml" | "html") => {
                            ("⚙️", egui::Color32::from_gray(180))
                        }
                        _ => ("📄", egui::Color32::from_rgb(150, 200, 255)),
                    };
                    let label = format!("{} {}", icon, name);
                    let mut rich_text = egui::RichText::new(label).size(14.0).color(color);
                    if is_selected {
                        rich_text = rich_text.background_color(ui.visuals().selection.bg_fill);
                    }

                    ui.add(egui::SelectableLabel::new(is_selected, rich_text))
                };

                if response.hovered() {
                    response.clone().highlight();
                }

                if response.clicked() {
                    requested_action = Some(Action::Select(path_clone.clone()));
                }

                response.context_menu(|ui| {
                    let key = path_clone.join("/");
                    let scheme = file_processing::get_node_value(db, &key)
                        .and_then(|n| n.scheme)
                        .unwrap_or_else(|| "https".to_string());
                    let url = format!("{}://{}", scheme, &key);
                    if ui.button("Send Request").clicked() {
                        requested_action = Some(Action::SendRequest(url.clone()));
                        ui.close_menu();
                    }
                    if ui.button("Copy URL").clicked() {
                        requested_action = Some(Action::Copy(url.clone()));
                        ui.close_menu();
                    }
                    if ui.button("Delete").clicked() {
                        requested_action = Some(Action::Delete(path_clone.clone()));
                        ui.close_menu();
                    }
                    if ui.button("Send to Proxy").clicked() {
                        requested_action = Some(Action::SendToProxy(url.clone()));
                        ui.close_menu();
                    }
                });

                requested_action
            });

            if let Some(action) = response.inner {
                return Some(action);
            }

            current_path.pop();
        }

        None
    }
}

fn get_all_children(db: &Db, key: &str, all_children: &mut Vec<String>) {
    if let Some(node_value) = file_processing::get_node_value(db, key) {
        if node_value.is_endpoint {
            if let Some(scheme) = file_processing::get_node_value(db, key).and_then(|n| n.scheme) {
                all_children.push(format!("{}://{}", scheme, key));
            } else {
                all_children.push(key.to_string());
            }
        }
        for child in node_value.children {
            let new_key = if key == "__ROOT__" {
                child.clone()
            } else {
                format!("{}/{}", key, child)
            };
            get_all_children(db, &new_key, all_children);
        }
    }
}

fn get_extension(name: &str) -> Option<&str> {
    name.rsplit_once('.').map(|(_, ext)| ext)
}

fn delete_node_from_db(db: &Db, path: &[String]) -> Result<usize, Box<dyn std::error::Error>> {
    let key = path.join("/");
    let mut deleted_count = 0;

    if let Some(node_value) = file_processing::get_node_value(db, &key) {
        if node_value.is_endpoint {
            deleted_count += 1;
        }
        for child in node_value.children {
            let mut child_path = path.to_vec();
            child_path.push(child);
            deleted_count += delete_node_from_db(db, &child_path)?;
        }
    }

    db.remove(&key)?;

    if path.len() > 1 {
        let parent_path = &path[0..path.len() - 1];
        let parent_key = parent_path.join("/");
        if let Some(mut parent_node_value) = file_processing::get_node_value(db, &parent_key) {
            if parent_node_value.children.remove(&path[path.len() - 1]) {
                let encoded = serde_json::to_vec(&parent_node_value)?;
                db.insert(parent_key.as_bytes(), encoded)?;
            }
        }
    } else {
        // It's a root domain
        if let Some(mut root_node_value) = file_processing::get_node_value(db, "__ROOT__") {
            if root_node_value.children.remove(&key) {
                let encoded = serde_json::to_vec(&root_node_value)?;
                db.insert("__ROOT__", encoded)?;
            }
        }
    }

    Ok(deleted_count)
}