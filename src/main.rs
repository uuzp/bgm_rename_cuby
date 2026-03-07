#![windows_subsystem = "windows"]
use fltk::dialog;
use fltk::enums::Event;
use fltk::{
    app,
    browser::HoldBrowser,
    button::Button,
    enums,
    frame::Frame,
    group::Flex,
    input::Input,
    menu::MenuButton,
    prelude::*,
    window::Window,
};
#[cfg(not(target_os = "windows"))]
use std::process::Command;

// Some bundled FLTK builds reference GDI+ symbols; ensure we link the import lib.
#[cfg(target_os = "windows")]
#[link(name = "gdiplus")]
unsafe extern "system" {}

// 引入模块
mod bangumi_api;

// --- 命令行参数定义 ---
#[derive(Debug, Clone, Default)]
struct CliArgs {
    /// 源文件路径（包含视频文件的文件夹）
    base_path: Option<String>,

    /// 目标文件路径（重命名后文件存放的文件夹）
    anime_path: Option<String>,
}

fn print_help() {
    println!(
        "{name} {ver}\n\n用法:\n  {name}.exe -b <源文件夹> -a <目标文件夹>\n\n选项:\n  -b, --base-path, --base_path   源文件夹路径\n  -a, --anime-path, --anime_path 目标文件夹路径\n  -h, --help                    显示帮助\n  -V, --version                 显示版本\n",
        name = env!("CARGO_PKG_NAME"),
        ver = env!("CARGO_PKG_VERSION")
    );
}

fn parse_cli_args() -> CliArgs {
    let mut args_iter = std::env::args().skip(1);
    let mut parsed = CliArgs::default();

    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-b" | "--base-path" | "--base_path" => {
                let Some(value) = args_iter.next() else {
                    eprintln!("缺少 -b/--base-path 参数值\n");
                    print_help();
                    std::process::exit(2);
                };
                parsed.base_path = Some(value);
            }
            "-a" | "--anime-path" | "--anime_path" => {
                let Some(value) = args_iter.next() else {
                    eprintln!("缺少 -a/--anime-path 参数值\n");
                    print_help();
                    std::process::exit(2);
                };
                parsed.anime_path = Some(value);
            }
            _ => {
                eprintln!("未知参数: {arg}\n");
                print_help();
                std::process::exit(2);
            }
        }
    }

    parsed
}

#[derive(Clone)]
enum Message {
    Register,
    Logout,
    About,
    Exit,
    ButtonA,
    ButtonB,
    Search,
    Start,

    FileBrowserPush,
    SearchBrowserPush,
    SearchBrowserDoubleClick,
    SearchBrowserRightClick,
    SearchCompleted {
        query: String,
        result: Result<Vec<bangumi_api::Subject>, String>,
    },
    EpisodesCompleted {
        subject: bangumi_api::Subject,
        result: Result<bangumi_api::Episodes, String>,
    },
}

#[derive(Clone)]
enum UiMode {
    Idle,
    SearchResults { subjects: Vec<bangumi_api::Subject> },
    EpisodeList {
        subject: bangumi_api::Subject,
        episodes: bangumi_api::Episodes,
    },
}

struct Cuby {
    app: app::App,
    sender: app::Sender<Message>,
    wind: Window,
    file_browser: HoldBrowser,
    search_browser: HoldBrowser,
    search_input: Input,
    info_frame: Frame,
    receiver: app::Receiver<Message>,

    base_path: String,
    anime_path: String,

    ui_mode: UiMode,
    pending_search_query: Option<String>,
    pending_episode_subject_id: Option<u64>,
}

struct FileOperationReport {
    successful_links: usize,
    failed_links: usize,
    skipped_files: usize,
    subtitle_copied: usize,
    subtitle_failed: usize,
    subtitle_skipped: usize,
    details: Vec<String>,
    cancelled: bool,
    cancel_message: Option<String>,
    transfer_mode: VideoTransferMode,
}

impl FileOperationReport {
    fn new() -> Self {
        Self {
            successful_links: 0,
            failed_links: 0,
            skipped_files: 0,
            subtitle_copied: 0,
            subtitle_failed: 0,
            subtitle_skipped: 0,
            details: Vec::new(),
            cancelled: false,
            cancel_message: None,
            transfer_mode: VideoTransferMode::HardLink,
        }
    }

    fn is_clean_success(&self) -> bool {
        !self.cancelled
            && self.failed_links == 0
            && self.skipped_files == 0
            && self.subtitle_failed == 0
            && self.subtitle_skipped == 0
    }

    fn summary(&self) -> String {
        if self.cancelled {
            return self
                .cancel_message
                .clone()
                .unwrap_or_else(|| "已取消".to_string());
        }

        let mut parts = vec![
            format!("{} {}", self.transfer_mode.success_label(), self.successful_links),
            format!("{} {}", self.transfer_mode.failure_label(), self.failed_links),
        ];

        if self.skipped_files > 0 {
            parts.push(format!("跳过 {}", self.skipped_files));
        }
        if self.subtitle_copied > 0 {
            parts.push(format!("字幕复制 {}", self.subtitle_copied));
        }
        if self.subtitle_failed > 0 {
            parts.push(format!("字幕失败 {}", self.subtitle_failed));
        }
        if self.subtitle_skipped > 0 {
            parts.push(format!("字幕跳过 {}", self.subtitle_skipped));
        }

        format!("完成: {}", parts.join("，"))
    }

    fn detail_message(&self) -> Option<String> {
        if self.details.is_empty() {
            return None;
        }

        let max_lines = 12;
        let mut lines: Vec<String> = self.details.iter().take(max_lines).cloned().collect();
        if self.details.len() > max_lines {
            lines.push(format!("... 其余 {} 条省略", self.details.len() - max_lines));
        }

        Some(format!("{}\n\n{}", self.summary(), lines.join("\n")))
    }
}

enum OperationOutcome {
    Success(String),
    Partial(FileOperationReport),
    Cancelled(String),
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum VideoTransferMode {
    HardLink,
    Copy,
}

impl VideoTransferMode {
    fn success_label(self) -> &'static str {
        match self {
            VideoTransferMode::HardLink => "视频硬链接成功",
            VideoTransferMode::Copy => "视频复制成功",
        }
    }

    fn failure_label(self) -> &'static str {
        match self {
            VideoTransferMode::HardLink => "视频硬链接失败",
            VideoTransferMode::Copy => "视频复制失败",
        }
    }
}

impl Cuby {
    fn new(cli_args: &CliArgs) -> Self {
        // 创建应用和窗口
        let app = app::App::default();
        let (sender, receiver) = app::channel::<Message>();
        let mut wind = Window::default()
            .with_size(1200, 600)
            .with_label("BGM Rename Cuby");
        
        // 创建主Flex布局，垂直排列(上中下三区域)
        let mut main_flex = Flex::default_fill().column();
        
        // 上区域：菜单按钮、按钮B、按钮A、搜索框和搜索按钮
        let mut top_flex = Flex::default().row();

        let sender_menu = sender.clone();
        
        let mut menu_btn = MenuButton::default().with_label("菜单");
        menu_btn.add_choice("注册");
        menu_btn.add_choice("注销");
        menu_btn.add_choice("关于");
        menu_btn.add_choice("退出");
        menu_btn.set_callback(move |m| {
            match m.choice().unwrap().as_str() {
                "注册" => sender_menu.send(Message::Register),
                "注销" => sender_menu.send(Message::Logout),
                "关于" => sender_menu.send(Message::About),
                "退出" => sender_menu.send(Message::Exit),
                _ => (),
            }
        });
        
        let mut btn_b = Button::default().with_label("|🌀|");
        btn_b.emit(sender.clone(), Message::ButtonB);
        let mut btn_a = Button::default().with_label("|🎬|");
        btn_a.emit(sender.clone(), Message::ButtonA);
        
        let mut search_input = Input::default(); 
        let mut search_btn = Button::default().with_label("🔍");
        search_btn.emit(sender.clone(), Message::Search);

        top_flex.fixed(&search_input, 345); 
        top_flex.fixed(&search_btn, 50);
        top_flex.fixed(&menu_btn, 60);
        top_flex.end();
        main_flex.fixed(&top_flex, 30);
        
        // 中区域：文件列表和搜索列表
        let mut mid_flex = Flex::default().row();
        
        let mut file_browser = HoldBrowser::default();
        file_browser.set_selection_color(enums::Color::from_hex_str("#9999FF").unwrap()); 

        {
            let sender_file = sender.clone();
            let mut drag_item: i32 = -1;
            file_browser.handle(move |browser, event| match event {
                Event::Push => {
                    sender_file.send(Message::FileBrowserPush);
                    true
                }
                Event::Drag | Event::Released => handle_browser_events(browser, event, &mut drag_item),
                _ => false,
            });
        }
        
        let mut search_browser = HoldBrowser::default();
        search_browser.set_selection_color(enums::Color::from_hex_str("#39C5BB").unwrap()); 

        {
            let sender_search = sender.clone();
            search_browser.handle(move |_browser, event| match event {
                Event::Push => {
                    if app::event_button() == 3 {
                        sender_search.send(Message::SearchBrowserRightClick);
                    } else if app::event_clicks() {
                        sender_search.send(Message::SearchBrowserDoubleClick);
                    } else {
                        sender_search.send(Message::SearchBrowserPush);
                    }
                    true
                }
                _ => false,
            });
        }
        mid_flex.fixed(&search_browser, 400);
        mid_flex.end();          
        // 下区域：路径信息Frame和开始按钮
        let mut bottom_flex = Flex::default().row();        
        let mut info_frame = Frame::default().with_label("@");
        info_frame.set_align(enums::Align::Left | enums::Align::Inside);
       
        // 创建一个垂直布局来包含按钮和底部边距
        let mut btn_container = Flex::default().column();
        let mut start_btn = Button::default().with_label("开始");
        start_btn.emit(sender.clone(), Message::Start);
        
        let bottom_margin = Frame::default();
        btn_container.fixed(&bottom_margin, 2);
        btn_container.end();
        
        let right_margin = Frame::default();
        
        // 开始按钮布局
        bottom_flex.fixed(&btn_container, 60);
        bottom_flex.fixed(&right_margin, 5);
        bottom_flex.end();
        
        main_flex.fixed(&bottom_flex, 30);
        main_flex.end();
        wind.resizable(&main_flex);
        wind.end();        
        wind.show();
        
        // 窗口图标由 Windows 资源(ico.rc)提供，避免引入图片解码以减小体积

        let base_path = cli_args.base_path.clone().unwrap_or_default();
        let anime_path = cli_args.anime_path.clone().unwrap_or_default();

        if !base_path.is_empty() {
            load_files_to_file_browser(&base_path, &mut file_browser);
            if let Some(folder_name) = std::path::Path::new(&base_path)
                .file_name()
                .and_then(|n| n.to_str())
            {
                if let Some(extracted_name) = extract_anime_name_regex(folder_name) {
                    search_input.set_value(&extracted_name);
                }
            }
        }
        
        Self {
            app,
            sender,
            wind,
            file_browser,
            search_browser,
            search_input,
            info_frame,
            receiver,
            base_path,
            anime_path,
            ui_mode: UiMode::Idle,
            pending_search_query: None,
            pending_episode_subject_id: None,
        }
    }

    fn run(mut self) {
        while self.app.wait() {
            if let Some(msg) = self.receiver.recv() {
                if !self.handle_message(msg) {
                    break;
                }
            }
        }
    }

    fn handle_message(&mut self, msg: Message) -> bool {
        let should_continue = match msg {
            Message::Register => {
                self.handle_register_menu();
                true
            }
            Message::Logout => {
                unregister_context_menu();
                true
            }
            Message::About => {
                handle_about_menu();
                true
            }
            Message::Exit => {
                self.wind.hide();
                false
            }
            Message::ButtonA => {
                self.handle_button_a();
                true
            }
            Message::ButtonB => {
                self.handle_button_b();
                true
            }
            Message::Search => {
                self.handle_search_button();
                true
            }
            Message::Start => {
                self.handle_start_button();
                true
            }
            Message::FileBrowserPush => {
                self.handle_browser_click(true);
                true
            }
            Message::SearchBrowserPush => {
                self.handle_browser_click(false);
                true
            }
            Message::SearchBrowserDoubleClick => {
                self.handle_search_results_double_click();
                true
            }
            Message::SearchBrowserRightClick => {
                self.handle_search_results_right_click();
                true
            }
            Message::SearchCompleted { query, result } => {
                self.handle_search_completed(query, result);
                true
            }
            Message::EpisodesCompleted { subject, result } => {
                self.handle_episodes_completed(subject, result);
                true
            }
        };

        if should_continue {
            self.redraw_ui();
        }

        should_continue
    }

    fn redraw_ui(&mut self) {
        self.info_frame.redraw();
        self.wind.redraw();
    }

    fn set_info(&mut self, message: &str) {
        self.info_frame.set_label(message);
    }

    fn reset_search_ui(&mut self) {
        self.search_browser.clear();
        self.ui_mode = UiMode::Idle;
    }

    fn on_operation_success(&mut self, message: &str) {
        self.set_info(message);
        self.base_path.clear();
        self.file_browser.clear();
        self.search_input.set_value("");
        self.reset_search_ui();
    }

    fn handle_register_menu(&mut self) {
        if self.anime_path.is_empty() {
            self.set_info("错误: 未设置目标位置路径（A按钮）");
            return;
        }
        register_context_menu(&self.anime_path);
    }

    fn show_invalid_selection(&mut self) {
        self.search_browser.clear();
        self.search_browser.add("选择无效或数据不一致");
        self.set_info("选择无效");
    }

    fn selected_subject_in_search_results(&mut self) -> Option<bangumi_api::Subject> {
        let UiMode::SearchResults { subjects } = &self.ui_mode else {
            self.set_info("当前不是番剧搜索结果列表");
            return None;
        };

        let Some(idx) = self.selected_search_index() else {
            self.set_info("请选择一个番剧");
            return None;
        };

        match subjects.get(idx).cloned() {
            Some(subject) => Some(subject),
            None => {
                self.show_invalid_selection();
                None
            }
        }
    }

    fn render_subject_list(&mut self, subjects: &[bangumi_api::Subject]) {
        self.search_browser.clear();
        for subject in subjects {
            if subject.name_cn.is_empty() {
                self.search_browser.add(&subject.name);
            } else {
                self.search_browser
                    .add(&format!("{} ({})", subject.name_cn, subject.name));
            }
        }
    }

    fn render_episode_list(&mut self, episodes: &bangumi_api::Episodes) -> usize {
        self.search_browser.clear();

        if episodes.items.is_empty() {
            self.search_browser.add("未能获取到剧集信息或剧集列表为空");
            return 0;
        }

        for ep in &episodes.items {
            let name = if ep.name_cn.is_empty() {
                ep.name.as_str()
            } else {
                ep.name_cn.as_str()
            };
            let display_text = format!("Ep.{:02} - {}", ep.sort, name);
            self.search_browser.add(&display_text);
        }

        episodes.items.len()
    }

    fn selected_search_index(&self) -> Option<usize> {
        let line = self.search_browser.value();
        if line <= 0 || line > self.search_browser.size() {
            return None;
        }
        Some((line as usize) - 1)
    }

    /// 处理按钮A点击事件
    fn handle_button_a(&mut self) {
        if update_path(&mut self.anime_path, "select ANIME_PATH") {
            self.set_info(&format!("@ {}", self.anime_path));
        }
    }

    /// 处理按钮B点击事件
    fn handle_button_b(&mut self) {
        if update_path(&mut self.base_path, "select BASE_PATH") {
            self.set_info(&format!("@ {}", self.base_path));
            load_files_to_file_browser(&self.base_path, &mut self.file_browser);
            // 从路径中提取番剧名并填充到搜索框
            if let Some(folder_name) = std::path::Path::new(&self.base_path)
                .file_name()
                .and_then(|n| n.to_str())
            {
                if let Some(extracted_name) = extract_anime_name_regex(folder_name) {
                    self.search_input.set_value(&extracted_name);
                }
            }
        }
    }

    /// 处理搜索按钮点击事件
    fn handle_search_button(&mut self) {
        let query = self.search_input.value();
        if query.is_empty() {
            self.set_info("无关键词");
        } else {
            self.start_search(query);
        }
    }

    /// 处理浏览器单击事件
    fn handle_browser_click(&mut self, is_file_browser: bool) {
        if is_file_browser {
            self.set_info(&format!("@ {}", self.base_path));
        } else {
            self.set_info(&format!("@ {}", self.anime_path));
        }
    }

    fn start_search(&mut self, query: String) {
        self.pending_search_query = Some(query.clone());
        self.pending_episode_subject_id = None;
        self.ui_mode = UiMode::Idle;
        self.search_browser.clear();
        self.search_browser.add("搜索中...");
        self.set_info(&format!("正在搜索: {}", query));

        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = bangumi_api::search_subjects(&query).map_err(|err| err.to_string());
            sender.send(Message::SearchCompleted { query, result });
        });
    }

    fn handle_search_completed(
        &mut self,
        query: String,
        result: Result<Vec<bangumi_api::Subject>, String>,
    ) {
        if self.pending_search_query.as_deref() != Some(query.as_str()) {
            return;
        }
        self.pending_search_query = None;

        match result {
            Ok(subjects) => {
                if subjects.is_empty() {
                    self.reset_search_ui();
                    self.set_info(&format!("未找到与\"{}\"相关的番剧", query));
                } else {
                    self.render_subject_list(&subjects);
                    self.ui_mode = UiMode::SearchResults { subjects };
                    self.set_info(&format!("找到 {} 个搜索结果", self.search_browser.size()));
                }
            }
            Err(err_msg) => {
                self.reset_search_ui();
                self.set_info(&format!("搜索失败: {}", err_msg));
            }
        }
    }

    /// 处理搜索结果双击事件
    fn handle_search_results_double_click(&mut self) {
        // 仅在“番剧搜索结果列表”状态下生效
        let Some(selected_subject) = self.selected_subject_in_search_results() else {
            return;
        };

        self.pending_episode_subject_id = Some(selected_subject.id);
        self.set_info(&format!(
            "正在加载《{}》的剧集信息...",
            selected_subject.display_name()
        ));

        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = bangumi_api::get_episodes(&selected_subject).map_err(|err| err.to_string());
            sender.send(Message::EpisodesCompleted {
                subject: selected_subject,
                result,
            });
        });
    }

    fn handle_episodes_completed(
        &mut self,
        subject: bangumi_api::Subject,
        result: Result<bangumi_api::Episodes, String>,
    ) {
        if self.pending_episode_subject_id != Some(subject.id) {
            return;
        }
        self.pending_episode_subject_id = None;

        match result {
            Ok(episodes) => {
                let rendered = self.render_episode_list(&episodes);
                if rendered == 0 {
                    self.set_info("剧集列表为空");
                } else {
                    self.set_info(&format!("已加载 {} 集剧集信息", rendered));
                }

                self.ui_mode = UiMode::EpisodeList { subject, episodes };
            }
            Err(err_msg) => {
                self.set_info(&format!("获取剧集信息失败: {}", err_msg));
            }
        }
    }

    /// 处理搜索结果右键事件：打开 Bangumi 番剧网页
    fn handle_search_results_right_click(&mut self) {
        let Some(subject) = self.selected_subject_in_search_results() else {
            return;
        };

        let url = format!("https://bgm.tv/subject/{}", subject.id);
        if let Err(e) = open_url(&url) {
            self.set_info(&format!("打开 Bangumi 网页失败: {}", e));
        }
    }

    /// 处理完成按钮逻辑（简化版）
    fn handle_start_button(&mut self) {
        let result = self.execute_operation();
        match result {
            Ok(OperationOutcome::Success(message)) => {
                self.on_operation_success(&message);
            }
            Ok(OperationOutcome::Partial(report)) => {
                let summary = report.summary();
                self.set_info(&summary);
                if let Some(detail_message) = report.detail_message() {
                    dialog::message_default(&detail_message);
                }
            }
            Ok(OperationOutcome::Cancelled(message)) => {
                self.set_info(&message);
            }
            Err(error) => {
                // 操作失败时，不清空列表，只显示错误信息
                self.set_info(&error);
            }
        }
    }

    /// 执行完整操作流程
    fn execute_operation(&mut self) -> Result<OperationOutcome, String> {
        // 统一验证
        let (base_path_str, anime_path_str) = self.is_check()?;

        // 收集源文件
        let source_files = collect_source_files_from_browser(&self.file_browser);

        let UiMode::EpisodeList { subject, episodes } = &self.ui_mode else {
            return Err("错误: 请先搜索并选择番剧".to_string());
        };

        let year = episodes.year.to_string();
        let anime_display_name = subject.display_name().to_string();

        // 创建目标目录
        let target_anime_dir = self.prepare_target_directory(&anime_path_str, &anime_display_name, &year)?;
        let transfer_mode = self.choose_video_transfer_mode(&base_path_str, &target_anime_dir)?;

        // 执行操作
        let report = self.execute_file_operations(
            &source_files,
            &base_path_str,
            episodes,
            &target_anime_dir,
            transfer_mode,
        );

        if report.cancelled {
            return Ok(OperationOutcome::Cancelled(report.summary()));
        }

        if report.is_clean_success() {
            Ok(OperationOutcome::Success(report.summary()))
        } else {
            Ok(OperationOutcome::Partial(report))
        }
    }

    /// 统一验证：路径是否已设置
    fn is_check(&self) -> Result<(String, String), String> {
        if self.base_path.is_empty() {
            return Err("错误: 未设置源文件路径（B按钮）".to_string());
        }
        if self.anime_path.is_empty() {
            return Err("错误: 未设置目标位置路径（A按钮）".to_string());
        }

        let base_path_str = self.base_path.clone();
        let anime_path_str = self.anime_path.clone();

        Ok((base_path_str, anime_path_str))
    }

    /// 执行文件操作
    fn execute_file_operations(
        &self,
        source_files: &[String],
        base_path_str: &str,
        episodes: &bangumi_api::Episodes,
        target_anime_dir: &std::path::Path,
        transfer_mode: VideoTransferMode,
    ) -> FileOperationReport {
        let formatted_episode_names = episodes.formatted_names();
        self.execute_file_renaming(
            source_files,
            base_path_str,
            &formatted_episode_names,
            target_anime_dir,
            transfer_mode,
        )
    }

    /// 准备目标目录
    fn prepare_target_directory(&self, anime_path_root_str: &str, anime_display_name: &str, year: &str) -> Result<std::path::PathBuf, String> {
        let cleaned_anime_name_for_folder = clean_filename(anime_display_name);
        let target_anime_folder_name = format!("{}({})", cleaned_anime_name_for_folder, year);
        let target_anime_dir = std::path::Path::new(anime_path_root_str).join(target_anime_folder_name);

        std::fs::create_dir_all(&target_anime_dir)
            .map_err(|e| format!("错误: 创建目标文件夹失败: {}", e))?;
        
        Ok(target_anime_dir)
    }    /// 执行文件硬链接
    fn execute_file_renaming(
        &self,
        source_files: &[String],
        base_path_str: &str,
        formatted_episode_names: &[String],
        target_anime_dir: &std::path::Path,
        transfer_mode: VideoTransferMode,
    ) -> FileOperationReport {
        let mut report = FileOperationReport::new();
        report.transfer_mode = transfer_mode;

        if source_files.len() > formatted_episode_names.len() {
            let msg = format!(
                "警告: 选择的文件数量 ({}) 多于剧集数量 ({}). 是否继续?",
                source_files.len(),
                formatted_episode_names.len()
            );
            if dialog::choice2_default(&msg, "继续", "取消", "") != Some(0) {
                report.cancelled = true;
                report.cancel_message = Some("已取消：文件数量多于剧集数量".to_string());
                return report;
            }
        }

        let mut active_transfer_mode = transfer_mode;

        for (i, source_file_name_str) in source_files.iter().enumerate() {
            if i >= formatted_episode_names.len() {
                report.skipped_files += 1;
                report.details.push(format!("跳过文件 '{}': 超出剧集范围", source_file_name_str));
                continue;
            }

            let source_file_path = std::path::Path::new(base_path_str).join(source_file_name_str);
            let original_extension = source_file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
            let cleaned_episode_name_part = clean_filename(&formatted_episode_names[i]);
            let new_file_name_str = if original_extension.is_empty() {
                cleaned_episode_name_part.clone()
            } else {
                format!("{}.{}", cleaned_episode_name_part, original_extension)
            };
            
            let target_file_path = target_anime_dir.join(&new_file_name_str);

            if source_file_path == target_file_path {
                report.skipped_files += 1;
                report
                    .details
                    .push(format!("跳过文件 '{}': 源与目标相同", source_file_name_str));
                continue;
            }
            if target_file_path.exists() {
                report.skipped_files += 1;
                report
                    .details
                    .push(format!("跳过文件 '{}': 目标已存在", source_file_name_str));
                continue;
            }

            match copy_or_link_file(&source_file_path, &target_file_path, active_transfer_mode) {
                Ok(_) => {
                    report.successful_links += 1;
                    self.copy_matching_subtitles(
                        &mut report,
                        source_file_name_str,
                        base_path_str,
                        &cleaned_episode_name_part,
                        target_anime_dir,
                    );
                }
                Err(e) => {
                    if active_transfer_mode == VideoTransferMode::HardLink
                        && is_cross_device_link_error(&e)
                    {
                        if confirm_cross_volume_copy(
                            std::path::Path::new(base_path_str),
                            target_anime_dir,
                        ) {
                            active_transfer_mode = VideoTransferMode::Copy;
                            report.transfer_mode = VideoTransferMode::Copy;

                            match copy_or_link_file(
                                &source_file_path,
                                &target_file_path,
                                active_transfer_mode,
                            ) {
                                Ok(_) => {
                                    report.successful_links += 1;
                                    self.copy_matching_subtitles(
                                        &mut report,
                                        source_file_name_str,
                                        base_path_str,
                                        &cleaned_episode_name_part,
                                        target_anime_dir,
                                    );
                                    continue;
                                }
                                Err(copy_err) => {
                                    report.failed_links += 1;
                                    report.details.push(format!(
                                        "失败: '{}', 错误: {}",
                                        source_file_name_str, copy_err
                                    ));
                                    continue;
                                }
                            }
                        }

                        report.cancelled = true;
                        report.cancel_message = Some("已取消：跨盘复制未确认".to_string());
                        return report;
                    }

                    report.failed_links += 1;
                    report
                        .details
                        .push(format!("失败: '{}', 错误: {}", source_file_name_str, e));
                }
            }
        }

        report
    }

    fn choose_video_transfer_mode(
        &self,
        base_path_str: &str,
        target_anime_dir: &std::path::Path,
    ) -> Result<VideoTransferMode, String> {
        let source_root = std::path::Path::new(base_path_str);
        if !paths_on_different_volumes(source_root, target_anime_dir) {
            return Ok(VideoTransferMode::HardLink);
        }

        if confirm_cross_volume_copy(source_root, target_anime_dir) {
            Ok(VideoTransferMode::Copy)
        } else {
            Err("已取消：跨盘复制未确认".to_string())
        }
    }

    fn copy_matching_subtitles(
        &self,
        report: &mut FileOperationReport,
        source_file_name_str: &str,
        base_path_str: &str,
        cleaned_episode_name_part: &str,
        target_anime_dir: &std::path::Path,
    ) {
        let subtitle_files = find_matching_subtitle_files(source_file_name_str, base_path_str);
        for (subtitle_file_name, subtitle_ext) in subtitle_files {
            let source_subtitle_path = std::path::Path::new(base_path_str).join(&subtitle_file_name);

            let subtitle_new_name = if subtitle_file_name.starts_with(&format!(
                "{}.",
                source_file_name_str
                    .rsplit_once('.')
                    .map(|(base, _)| base)
                    .unwrap_or(source_file_name_str)
            )) {
                let video_base = source_file_name_str
                    .rsplit_once('.')
                    .map(|(base, _)| base)
                    .unwrap_or(source_file_name_str);
                let subtitle_base = subtitle_file_name
                    .rsplit_once('.')
                    .map(|(base, _)| base)
                    .unwrap_or(&subtitle_file_name);
                let language_part = &subtitle_base[video_base.len()..];
                format!("{}{}.{}", cleaned_episode_name_part, language_part, subtitle_ext)
            } else {
                format!("{}.{}", cleaned_episode_name_part, subtitle_ext)
            };

            let target_subtitle_path = target_anime_dir.join(&subtitle_new_name);
            if !target_subtitle_path.exists() {
                match std::fs::copy(&source_subtitle_path, &target_subtitle_path) {
                    Ok(_) => {
                        report.subtitle_copied += 1;
                    }
                    Err(e) => {
                        report.subtitle_failed += 1;
                        report.details.push(format!(
                            "字幕文件复制失败: '{}', 错误: {}",
                            subtitle_file_name, e
                        ));
                    }
                }
            } else {
                report.subtitle_skipped += 1;
                report
                    .details
                    .push(format!("跳过字幕文件 '{}': 目标已存在", subtitle_file_name));
            }
        }
    }
}
// main函数
fn main() {
    // 解析命令行参数
    let cli_args = parse_cli_args();
    
    let app = Cuby::new(&cli_args);
    app.run();
}
// 
use std::env;

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT, REG_SZ,
};

#[cfg(target_os = "windows")]
fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// 支持的视频文件扩展名常量
static VIDEO_EXTENSIONS: &[&str] = &["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm"];

// 支持的字幕文件扩展名常量
static SUBTITLE_EXTENSIONS: &[&str] = &["srt", "ass", "ssa", "vtt", "sub", "idx", "sup"];

fn ext_in_list_ignore_ascii_case(ext: &str, list: &[&str]) -> bool {
    for &known in list {
        if ext.eq_ignore_ascii_case(known) {
            return true;
        }
    }
    false
}

fn copy_or_link_file(
    source_file_path: &std::path::Path,
    target_file_path: &std::path::Path,
    transfer_mode: VideoTransferMode,
) -> std::io::Result<()> {
    match transfer_mode {
        VideoTransferMode::HardLink => std::fs::hard_link(source_file_path, target_file_path),
        VideoTransferMode::Copy => std::fs::copy(source_file_path, target_file_path).map(|_| ()),
    }
}

fn confirm_cross_volume_copy(
    source_root: &std::path::Path,
    target_root: &std::path::Path,
) -> bool {
    let message = format!(
        "检测到源目录和目标目录不在同一磁盘。\n\n跨盘无法创建硬链接，只能改为复制视频文件。复制可能比较耗时，并且会额外占用硬盘空间。\n\n源目录: {}\n目标目录: {}\n\n是否继续？",
        source_root.display(),
        target_root.display()
    );

    dialog::choice2_default(&message, "继续复制", "取消", "") == Some(0)
}

fn is_cross_device_link_error(error: &std::io::Error) -> bool {
    #[cfg(target_os = "windows")]
    {
        return error.raw_os_error() == Some(17);
    }

    #[cfg(not(target_os = "windows"))]
    {
        error.raw_os_error() == Some(18)
    }
}

#[cfg(target_os = "windows")]
fn paths_on_different_volumes(source: &std::path::Path, target: &std::path::Path) -> bool {
    let source_prefix = windows_volume_prefix(source);
    let target_prefix = windows_volume_prefix(target);

    match (source_prefix, target_prefix) {
        (Some(source_prefix), Some(target_prefix)) => source_prefix != target_prefix,
        _ => false,
    }
}

#[cfg(not(target_os = "windows"))]
fn paths_on_different_volumes(_source: &std::path::Path, _target: &std::path::Path) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn windows_volume_prefix(path: &std::path::Path) -> Option<String> {
    use std::path::Component;

    match path.components().next() {
        Some(Component::Prefix(prefix)) => {
            Some(prefix.as_os_str().to_string_lossy().to_ascii_lowercase())
        }
        _ => None,
    }
}

/// 查找与视频文件同名的字幕文件
pub fn find_matching_subtitle_files(video_file_name: &str, base_path: &str) -> Vec<(String, String)> {
    let mut subtitle_files = Vec::new();
    
    // 获取视频文件的基本名称（不含扩展名）
    let video_base_name = if let Some(dot_pos) = video_file_name.rfind('.') {
        &video_file_name[..dot_pos]
    } else {
        video_file_name
    };
    
    // 读取目录
    let dir = match std::fs::read_dir(base_path) {
        Ok(entries) => entries,
        Err(_) => return subtitle_files,
    };
    
    // 查找匹配的字幕文件
    for entry in dir.filter_map(Result::ok) {
        let path = entry.path();
        
        // 跳过非文件项
        if !path.is_file() {
            continue;
        }
        
        // 获取文件名和扩展名
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        
        // 检查是否是字幕文件
        if !ext_in_list_ignore_ascii_case(ext, SUBTITLE_EXTENSIONS) {
            continue;
        }
        
        // 获取字幕文件的基本名称（不含扩展名）
        let subtitle_base_name = if let Some(dot_pos) = file_name.rfind('.') {
            &file_name[..dot_pos]
        } else {
            file_name
        };
        
        // 检查是否匹配视频文件名
        // 支持以下匹配模式：
        // 1. 完全匹配：video.mkv -> video.srt
        // 2. 语言标识匹配：video.mkv -> video.sc.srt, video.tc.srt, video.en.srt 等
        if subtitle_base_name == video_base_name {
            // 完全匹配
            subtitle_files.push((file_name.to_string(), ext.to_string()));
        } else if subtitle_base_name.starts_with(&format!("{}.", video_base_name)) {
            // 带语言标识的匹配
            let language_part = &subtitle_base_name[video_base_name.len() + 1..];
            // 检查语言标识是否合理（不包含特殊字符，长度合理）
            if language_part.len() <= 10 && language_part.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                subtitle_files.push((file_name.to_string(), ext.to_string()));
            }
        }
    }
    
    subtitle_files
}

/// 使用正则表达式从路径中提取番剧名
pub fn extract_anime_name_regex(file_name: &str) -> Option<String> {
    if file_name.starts_with('[') {
        // 支持：
        // 1) [组名][番剧名][其他]
        // 2) [Rev][组名][番剧名][其他]
        // 3) [组名]番剧名[其他]
        let mut rest = file_name;
        let mut tags: Vec<&str> = Vec::new();

        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else {
                break;
            };
            let tag = &stripped[..end];
            tags.push(tag);
            rest = &stripped[end + 1..];
        }

        if tags.len() >= 3 && tags[0].eq_ignore_ascii_case("rev") {
            let name = tags[2].trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }

        if tags.len() >= 2 {
            let name = tags[1].trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }

        // [组名]番剧名[其他]
        let after_first = rest.trim_start();
        if !after_first.is_empty() {
            let name_part = after_first.split('[').next().unwrap_or("").trim();
            if !name_part.is_empty() {
                return Some(name_part.to_string());
            }
        }
    }

    // 下划线格式：番剧名_其他
    file_name
        .split_once('_')
        .map(|(before, _)| before.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 注册右键菜单
#[cfg(target_os = "windows")]
pub fn register_context_menu(anime_path: &str) {
    // 获取当前可执行文件路径
    match env::current_exe() {
        Ok(exe_path_buf) => {
            let exe_path = exe_path_buf.to_string_lossy().to_string();
            let mut errors = Vec::new();
            let key_name = "Add To Cuby";
            let shell_path = "Directory\\shell";

            // 创建右键菜单项：-b 使用右键点击的目录；-a 使用用户配置的目标路径
            let command_val = format!("\"{}\" -b \"%1\" -a \"{}\"", exe_path, anime_path);

            let menu_key_path = format!("{}\\{}", shell_path, key_name);
            let menu_key_path_w = wide_null(&menu_key_path);

            unsafe {
                let mut menu_key: HKEY = std::ptr::null_mut();
                let status = RegCreateKeyW(HKEY_CLASSES_ROOT, menu_key_path_w.as_ptr(), &mut menu_key);
                if status != 0 {
                    errors.push(format!(
                        "创建菜单主键失败: {}",
                        std::io::Error::from_raw_os_error(status as i32)
                    ));
                } else {
                    let default_value_w = wide_null(key_name);
                    let icon_value = format!("\"{}\",0", exe_path);
                    let icon_value_w = wide_null(&icon_value);
                    let icon_name_w = wide_null("Icon");

                    let s1 = RegSetValueExW(
                        menu_key,
                        std::ptr::null(),
                        0,
                        REG_SZ,
                        default_value_w.as_ptr() as *const u8,
                        (default_value_w.len() * 2) as u32,
                    );
                    if s1 != 0 {
                        errors.push(format!(
                            "设置菜单默认值失败: {}",
                            std::io::Error::from_raw_os_error(s1 as i32)
                        ));
                    }

                    let s2 = RegSetValueExW(
                        menu_key,
                        icon_name_w.as_ptr(),
                        0,
                        REG_SZ,
                        icon_value_w.as_ptr() as *const u8,
                        (icon_value_w.len() * 2) as u32,
                    );
                    if s2 != 0 {
                        errors.push(format!(
                            "设置菜单图标失败: {}",
                            std::io::Error::from_raw_os_error(s2 as i32)
                        ));
                    }

                    let command_key_path = format!("{}\\command", menu_key_path);
                    let command_key_path_w = wide_null(&command_key_path);
                    let mut command_key: HKEY = std::ptr::null_mut();
                    let s3 = RegCreateKeyW(
                        HKEY_CLASSES_ROOT,
                        command_key_path_w.as_ptr(),
                        &mut command_key,
                    );
                    if s3 != 0 {
                        errors.push(format!(
                            "创建命令子键失败: {}",
                            std::io::Error::from_raw_os_error(s3 as i32)
                        ));
                    } else {
                        let cmd_value_w = wide_null(&command_val);
                        let s4 = RegSetValueExW(
                            command_key,
                            std::ptr::null(),
                            0,
                            REG_SZ,
                            cmd_value_w.as_ptr() as *const u8,
                            (cmd_value_w.len() * 2) as u32,
                        );
                        if s4 != 0 {
                            errors.push(format!(
                                "设置命令失败: {}",
                                std::io::Error::from_raw_os_error(s4 as i32)
                            ));
                        }
                        let _ = RegCloseKey(command_key);
                    }

                    let _ = RegCloseKey(menu_key);
                }
            }

            // 显示结果
            if errors.is_empty() {
                dialog::message_default("注册表项已成功添加");
            } else {
                dialog::message_default(&format!(
                    "注册表操作时发生错误:\n{}\n\n请确保以管理员身份运行本程序",
                    errors.join("\n")
                ));
            }
        }
        Err(e) => dialog::message_default(&format!("获取程序路径失败: {}", e)),
    }
}

/// 注册右键菜单（非 Windows 平台）
#[cfg(not(target_os = "windows"))]
pub fn register_context_menu(_anime_path: &str) {
    dialog::message_default("当前平台不支持注册右键菜单");
}

/// 注销文件夹右键菜单
#[cfg(target_os = "windows")]
pub fn unregister_context_menu() {
    let key_name = "Add To Cuby";
    let key_path = format!("Directory\\shell\\{}", key_name);

    unsafe {
        let key_path_w = wide_null(&key_path);
        let status = RegDeleteTreeW(HKEY_CLASSES_ROOT, key_path_w.as_ptr());
        if status == 0 {
            dialog::message_default("相关注册表项已成功删除");
        } else if status as u32 == 2 {
            // ERROR_FILE_NOT_FOUND
            dialog::message_default("未找到相关的注册表项，无需注销");
        } else {
            dialog::message_default(&format!(
                "注销操作时发生错误: {}\n\n请确保以管理员身份运行本程序",
                std::io::Error::from_raw_os_error(status as i32)
            ));
        }
    }
}

/// 注销文件夹右键菜单（非 Windows 平台）
#[cfg(not(target_os = "windows"))]
pub fn unregister_context_menu() {
    dialog::message_default("当前平台不支持注销右键菜单");
}
/// 打开文件夹选择器
pub fn open_folder_selector(title: &str) -> Option<String> {
    use fltk::dialog::FileDialogType;

    let mut chooser = fltk::dialog::NativeFileChooser::new(FileDialogType::BrowseDir);
    chooser.set_title(title);
    chooser.show();

    let filename = chooser.filename();
    if filename.exists() {
        filename.to_str().map(|s| s.to_string())
    } else {
        None
    }
}
/// 通用的路径更新函数
pub fn update_path(path: &mut String, title: &str) -> bool {
    let Some(selected_path) = open_folder_selector(title) else {
        return false;
    };
    *path = selected_path;
    true
}

/// 加载文件到文件浏览器
pub fn load_files_to_file_browser(path_str: &str, browser: &mut HoldBrowser) {
    browser.clear();
    
    // 读取目录
    let dir = match std::fs::read_dir(path_str) {
        Ok(entries) => entries,
        Err(_) => {
            dialog::message_default(&format!("无法读取目录: {}", path_str));
            return;
        }
    };
    
    // 处理视频文件
    for entry in dir.filter_map(Result::ok) {
        let path = entry.path();
        
        // 跳过非文件项
        if !path.is_file() {
            continue;
        }
        
        // 获取文件名和扩展名
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        
        // 只添加视频文件
        if ext_in_list_ignore_ascii_case(ext, VIDEO_EXTENSIONS) {
            browser.add(file_name);
        }
    }
}


/// 从浏览器收集源文件
pub fn collect_source_files_from_browser(file_browser: &HoldBrowser) -> Vec<String> {
    let mut file_names = Vec::new();
    
    // 直接收集所有文件
    for i in 1..=file_browser.size() {
        if let Some(text) = file_browser.text(i) {
            file_names.push(text.to_string());
        }
    }
    
    file_names
}

/// 处理文件浏览器中的 Drag 事件
fn handle_browser_drag_event(browser: &mut HoldBrowser, drag_item: &mut i32) -> bool {
    let _y = app::event_y(); // y 坐标可能用于更精确的行计算，但当前未使用
    let current_item_under_mouse = browser.value(); // 获取鼠标当前悬停或选中的行

    let initial_drag_item_val = *drag_item; // 读取当前拖拽项的值

    if initial_drag_item_val < 0 { // 如果 drag_item 小于0，表示这是拖拽的开始
        *drag_item = current_item_under_mouse; // 记录开始拖拽的项
        return true;
    }
    
    // 如果鼠标下的项有效，并且不是当前正在拖拽的项
    if current_item_under_mouse > 0 && current_item_under_mouse != initial_drag_item_val {
        let text1 = browser.text(initial_drag_item_val).unwrap_or_default().to_string();
        let text2 = browser.text(current_item_under_mouse).unwrap_or_default().to_string();
          
        browser.set_text(initial_drag_item_val, &text2); // 将原拖拽项的内容设置为新位置项的内容
        browser.set_text(current_item_under_mouse, &text1); // 将新位置项的内容设置为原拖拽项的内容
          
        *drag_item = current_item_under_mouse; // 更新拖拽的项为当前鼠标下的项
        browser.select(current_item_under_mouse); // 保持选中新位置的项
        browser.redraw();
        return true;
    }
    true // 即使没有发生交换，也处理了拖拽事件
}

/// 处理文件浏览器中的 Released 事件
fn handle_browser_released_event(drag_item: &mut i32) -> bool {
    *drag_item = -1; // 重置拖拽项
    true
}

/// 处理文件浏览器事件
pub fn handle_browser_events(browser: &mut HoldBrowser, event: Event, drag_item: &mut i32) -> bool {
    match event {
        Event::Drag => handle_browser_drag_event(browser, drag_item),
        Event::Released => handle_browser_released_event(drag_item),
        _ => false,
    }
}

/// 处理关于菜单
fn handle_about_menu() {
    let repo_url = "https://github.com/uuzp/bgm_rename_cuby";
    let _ = open_url(repo_url);
}

fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::UI::Shell::{
            ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_FLAG_NO_UI,
        };
        const SW_SHOWNORMAL: i32 = 1;

        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let file: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();

        let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        sei.fMask = SEE_MASK_FLAG_NO_UI;
        sei.hwnd = std::ptr::null_mut();
        sei.lpVerb = op.as_ptr();
        sei.lpFile = file.as_ptr();
        sei.lpParameters = std::ptr::null();
        sei.lpDirectory = std::ptr::null();
        sei.nShow = SW_SHOWNORMAL;

        let ok = unsafe { ShellExecuteExW(&mut sei as *mut _) };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("ShellExecuteExW failed (GetLastError={err})"),
            ));
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).status()?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(url).status()?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        let _ = url;
        Ok(())
    }
}

/// 清理文件名中的特殊字符，使用全角字符替换
pub fn clean_filename(s: &str) -> String {
    s.replace("/", "／")
     .replace("\\", "＼") // Note: in a regular string, this would be a single backslash.
     .replace("<", "＜")
     .replace(">", "＞")
     // Consider adding other common problematic characters like : * ? " |
     .replace(":", "：")
     .replace("*", "＊")
     .replace("?", "？")
     .replace("\"", "＂")
     .replace("|", "｜")
}

