#![windows_subsystem = "windows"]

// Re-organize use statements for clarity
use crate::app_state::AppState;
use crate::config::AppSettings;
use crate::gui::settings_view::SettingsView;
use crate::gui::AppWindow;
use crate::message::{AppMessage, IndexMessage, SearchMessage, SettingsMessage};
use crate::search::query::SearchResult;
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// Application-specific modules
mod app_state;
mod config;
mod gui;
mod message;
mod search;
mod utils;

// APP结构体定义
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct MyApp {
    // 指定索引目录
    #[serde(skip)]
    index_path: Option<PathBuf>,
    // 指定搜索关键词
    #[serde(skip)]
    search_query: String,
    // 返回的搜索结果
    #[serde(skip)]
    search_results: Vec<SearchResult>,
    #[serde(skip)]
    search_duration: Option<Duration>,
    #[serde(skip)]
    state: AppState,
    #[serde(skip)]
    cancellation_token: Option<Arc<AtomicBool>>,
    settings: AppSettings,

    #[serde(skip)]
    sender: Sender<AppMessage>,
    #[serde(skip)]
    receiver: Receiver<AppMessage>,
    #[serde(skip)]
    windows: Vec<AppWindow>,
}

impl Default for MyApp {
    fn default() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            index_path: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_duration: None,
            state: AppState::default(),
            cancellation_token: None,
            settings: AppSettings::default(),
            sender,
            receiver,
            windows: Vec::new(),
        }
    }
}

impl MyApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        // Load previous app state (if any).
        let mut app: Self = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let (sender, receiver) = unbounded();
        app.sender = sender;
        app.receiver = receiver;

        app
    }
}

enum Action {
    None,
    OpenContext(String),
    OpenFile(String),
}

impl eframe::App for MyApp {
    // 窗口更新函数
    // 消息处理 异常处理+不阻塞
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(self.settings.get_visuals());

        // --- Draw Windows ---
        for window in self.windows.iter_mut() {
            window.draw(ctx, &self.search_query);
        }
        self.windows.retain(|w| *w.is_open());

        // 每次窗口更新，都会首先检查是否收到新消息
        if let Ok(msg) = self.receiver.try_recv() {
            // 信息类型的模式匹配
            match msg {
                AppMessage::Index(index_msg) => match index_msg {
                    IndexMessage::Progress(p) => {
                        self.state = AppState::Indexing { progress: p };
                    }
                    // 索引完成
                    // APP状态切回空闲
                    IndexMessage::Finished => {
                        self.state = AppState::Idle;
                    }
                    IndexMessage::Error(e) => {
                        eprintln!("Indexing Error: {}", e);
                        self.state = AppState::Idle;
                    }
                },
                AppMessage::Search(search_msg) => {
                    self.cancellation_token = None; // Clear token on any search result
                    match search_msg {
                        // 搜索完成
                        SearchMessage::Finished { results, duration } => {
                            self.search_results = results;
                            self.search_duration = Some(duration);
                            self.state = AppState::Idle;
                        }
                        SearchMessage::Cancelled => {
                            self.state = AppState::Idle;
                        }
                        SearchMessage::Error(e) => {
                            eprintln!("Search Error: {}", e);
                            self.search_results = vec![SearchResult {
                                path: e,
                                snippet_html: "".to_string(),
                            }];
                            self.state = AppState::Idle;
                        }
                    }
                }
                AppMessage::Settings(settings_msg) => match settings_msg {
                    // 主题变更
                    SettingsMessage::ThemeChanged(theme) => {
                        self.settings.theme = theme;
                    }
                },
            }
        }

        // --- UI Rendering ---

        // Left Panel for Controls
        egui::SidePanel::left("control_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Mytxt");

                ui.separator();

                // --- Indexing Section ---
                ui.collapsing("索引", |ui| {
                    //  “检查 self.index_path。如果它为空，path_str 就是 "请选择目录"。如果它有值，就把它转换成字符串；如果转换失败，path_str 就是个空字符串。”
                    // 太tm优雅了
                    let path_str = self
                        .index_path
                        .as_ref()
                        .map_or("请选择目录", |p| p.to_str().unwrap_or_default());
                    ui.label(format!("目标目录: {}", path_str));

                    // 添加状态驱动的组件
                    // 返回值为组件状态
                    if ui
                        .add_enabled(self.state == AppState::Idle, egui::Button::new("选择目录"))
                        .clicked()
                    {
                        // 检测输入框是否有东西
                        // 有东西就赋值给索引路径
                        // 执行顺序为 先打开选择文件对话框 然后赋值
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.index_path = Some(path);
                        }
                    }

                    // awesome
                    // 先看有索引路径，再能让索引按钮能点
                    let index_button_enabled =
                        self.index_path.is_some() && self.state == AppState::Idle;
                    // 索引按钮
                    if ui
                        .add_enabled(index_button_enabled, egui::Button::new("开始索引"))
                        .clicked()
                    {
                        // 转换状态
                        // 方便加载索引动画
                        self.state = AppState::Indexing { progress: 0.0 };
                        let path = self.index_path.clone().unwrap();
                        let sender = self.sender.clone();
                        // 多线程处理索引
                        // 使用另一个线程进行索引来防止卡顿
                        // 多线程使用闭包来执行
                        // 并发可以使用thread::sleep(Duration::from_millis(1));
                        // Move的存在可以让此线程单独获得所有变量的所有权，因为update后，所有变量都可能会销毁，但索引可能会继续进行
                        // 内存安全
                        thread::spawn(move || {
                            if let Err(e) =
                                crate::search::indexer::index_directory(&path, sender.clone())
                            {
                                sender
                                    .send(AppMessage::Index(IndexMessage::Error(e.to_string())))
                                    .unwrap();
                            }
                        });
                    }
                });

                ui.separator();

                // --- Search Section ---
                ui.collapsing("搜索", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("关键词: ");
                        // 因为 add_enabled 不仅可以控制单个组件，它还可以控制一组任意复杂的UI组件
                        // 后面传入闭包包裹一段UI代码
                        ui.add_enabled_ui(self.state == AppState::Idle, |ui| {
                            // 可编辑文本框
                            ui.text_edit_singleline(&mut self.search_query);
                        });
                    });

                    // 搜索按钮
                    let search_button_enabled =
                        !self.search_query.is_empty() && self.state == AppState::Idle;
                    if ui
                        .add_enabled(search_button_enabled, egui::Button::new("搜索"))
                        .clicked()
                    {
                        self.state = AppState::Searching;
                        let query = self.search_query.clone();
                        let sender = self.sender.clone();
                        let token = Arc::new(AtomicBool::new(false));
                        self.cancellation_token = Some(token.clone());

                        // 多线程搜索
                        thread::spawn(move || {
                            let sender_clone = sender.clone();
                            let result = std::panic::catch_unwind(move || {
                                crate::search::query::search(&query, sender_clone, token)
                            });

                            match result {
                                Ok(Ok(_)) => {
                                    // Search completed successfully (sent its own message)
                                }
                                Ok(Err(e)) => {
                                    // Search returned a normal error
                                    sender
                                        .send(AppMessage::Search(SearchMessage::Error(e.to_string())))
                                        .unwrap();
                                }
                                Err(panic_payload) => {
                                    // Search panicked
                                    eprintln!("Search thread panicked: {:?}", panic_payload);
                                    let error_msg = "A critical error occurred in the search engine, possibly due to a corrupt file.".to_string();
                                    sender
                                        .send(AppMessage::Search(SearchMessage::Error(error_msg)))
                                        .unwrap();
                                }
                            }
                        });
                    }
                });

                ui.separator();

                // --- Status Display ---
                match &self.state {
                    AppState::Idle => {
                        ui.label("状态: 空闲");
                    }
                    AppState::Indexing { progress } => {
                        ui.add(egui::ProgressBar::new(*progress).show_percentage());
                        ui.label(format!("正在索引... {:.0}%", progress * 100.0));
                    }
                    AppState::Searching => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("正在搜索...");
                            if ui.button("停止").clicked() {
                                if let Some(token) = &self.cancellation_token {
                                    token.store(true, Ordering::SeqCst);
                                }
                            }
                        });
                    }
                }
            });

        // Central Panel for Results
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut action = Action::None;

            // 结果部分
            ui.horizontal(|ui| {
                ui.heading("搜索结果");
                if let Some(duration) = self.search_duration {
                    ui.label(
                        egui::RichText::new(format!("({:.2?})", duration)).color(egui::Color32::GRAY),
                    );
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.search_results.is_empty() {
                    ui.label("无结果");
                }

                for result in &self.search_results {
                    let item_response = egui::Frame::group(ui.style())
                        .show(ui, |ui| {
                            // 文本按钮
                            // let button =
                            //     egui::Button::new(egui::RichText::new(&result.path).strong())
                            //         .frame(false);
                            // if ui.add(button).double_clicked() {
                            //     action = Action::OpenFile(result.path.clone());
                            // }
                            let text = egui::RichText::new(&result.path).strong();
                            ui.label(text);
                            // ui.label(egui::RichText::new(&result.snippet_html).small());
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                ui.style_mut().override_text_style = Some(egui::TextStyle::Small);
                                let parts = result.snippet_html.split("<b>");
                                let mut first = true;
                                for part in parts {
                                    if first && part.is_empty() {
                                        first = false;
                                        continue;
                                    }
                                    if let Some(highlighted_part) = part.find("</b>") {
                                        let (highlight, rest) = part.split_at(highlighted_part);
                                        ui.label(
                                            egui::RichText::new(highlight)
                                                .color(egui::Color32::RED),
                                        );
                                        ui.label(&rest[4..]); // a little ugly
                                    } else {
                                        ui.label(part);
                                    }
                                    first = false;
                                }
                            });
                        })
                        .response;

                    item_response.context_menu(|ui| {
                        if ui.button("打开文件").clicked() {
                            action = Action::OpenFile(result.path.clone());
                            ui.close_menu();
                        }
                        if ui.button("预览关键词上下文").clicked() {
                            action = Action::OpenContext(result.path.clone());
                            ui.close_menu();
                        }
                        if ui.button("打开文件所在文件夹").clicked() {
                            if let Some(parent) = std::path::Path::new(&result.path).parent() {
                                if let Err(e) = opener::open(parent) {
                                    eprintln!("Failed to open folder: {}", e);
                                }
                            }
                            ui.close_menu();
                        }
                    });

                    ui.separator();
                }
            });

            // Perform the action after the loop
            if let Action::OpenFile(path) = action {
                if let Err(e) = opener::open(&path) {
                    eprintln!("Failed to open file: {}", e);
                }
            } else if let Action::OpenContext(path) = action {
                if self
                    .windows
                    .iter()
                    .any(|w| matches!(w, AppWindow::Context(v) if v.path == path))
                {
                    println!("Window for {} is already open.", &path);
                } else {
                    match crate::utils::file_utils::read_file_content(std::path::Path::new(&path)) {
                        Ok(content) => {
                            self.windows.push(AppWindow::Context(
                                crate::gui::context_view::ContextView::new(path, content),
                            ));
                        }
                        Err(e) => {
                            eprintln!("Failed to read file for context view: {}", e);
                        }
                    }
                }
            }
        });

        // Floating Settings Button
        egui::Area::new("settings_button_area".into())
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .show(ctx, |ui| {
                // Allocate a fixed-size space for our button
                let size = egui::vec2(40.0, 40.0);
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

                // Check for interaction
                if response.clicked() {
                    // check if window already open
                    let is_already_open =
                        self.windows.iter().any(|w| matches!(w, AppWindow::Settings(_)));

                    if !is_already_open {
                        let view = SettingsView::new(self.sender.clone(), self.settings.theme);
                        self.windows.push(AppWindow::Settings(view));
                    }
                }

                // Draw the button background
                let visuals = ui.style().interact(&response);
                let bg_color = visuals.bg_fill;
                let rounding = rect.height() / 2.0;
                ui.painter().rect_filled(rect, rounding, bg_color);

                // Draw the gear icon in the center
                let icon_color = visuals.fg_stroke.color;
                ui.painter().text(
                    rect.center() + egui::vec2(0.0, 4.0), // Apply a slight vertical offset for visual centering
                    egui::Align2::CENTER_CENTER,
                    "⚙",
                    egui::FontId::proportional(24.0),
                    icon_color,
                );
            });

        if self.state != AppState::Idle || !self.windows.is_empty() {
            ctx.request_repaint();
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

fn main() -> Result<(), eframe::Error> {
    // 窗口基础配置
    let options = eframe::NativeOptions {
        // 窗口大小控制
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        // 语法糖，其他默认
        ..Default::default()
    };
    // eframe窗口创建
    eframe::run_native(
        "MyTxt",
        options,
        // 用于初始状态的闭包
        Box::new(|cc| {
            // 初始化获得空字体配置清单
            let mut fonts = egui::FontDefinitions::default();
            // 模式匹配
            // read读取文件，获得字体ttc文件
            //返回值是result，使用ok进行异常处理
            if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
                // 将msyh加入字体清单
                // 前面的msyh是自己起的名
                fonts
                    .font_data
                    .insert("msyh".to_owned(), egui::FontData::from_owned(font_data));
                // 将 "msyh" 设置为首选的“比例字体”（大部分普通文本）
                if let Some(proportional) = fonts.families.get_mut(&egui::FontFamily::Proportional)
                {
                    proportional.insert(0, "msyh".to_owned());
                }
                // 将 "msyh" 也设置为首选的“等宽字体”（常用于代码显示）
                if let Some(monospace) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    monospace.insert(0, "msyh".to_owned());
                }
            }
            cc.egui_ctx.set_fonts(fonts);
            Box::new(MyApp::new(cc))
        }),
    )
}
