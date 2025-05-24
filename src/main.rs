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
use std::sync::{Mutex, OnceLock};
use std::cell::RefCell;
use std::rc::Rc;
use clap::Parser;

// 引入模块
mod bangumi_api;

static BASE_PATH: OnceLock<Mutex<String>> = OnceLock::new();
static ANIME_PATH: OnceLock<Mutex<String>> = OnceLock::new();

// --- 命令行参数定义 ---
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// 源文件路径（包含视频文件的文件夹）
    #[arg(short = 'b', long)]
    pub base_path: Option<String>,

    /// 目标文件路径（重命名后文件存放的文件夹）
    #[arg(short = 'a', long)]
    pub anime_path: Option<String>,
}

#[derive(Copy, Clone)]
enum Message {
    Register,
    Logout,
    About,
    Exit,
    ButtonA,
    ButtonB,
    Search,
    Start,
}

struct Cuby {
    app: app::App,
    wind: Window,
    file_browser: HoldBrowser,
    search_browser: HoldBrowser,
    search_input: Input,
    info_frame: Frame,
    receiver: app::Receiver<Message>,
    // 搜索相关的数据存储
    search_results: Rc<RefCell<Option<Vec<bangumi_api::BangumiSubject>>>>,
    episode_list: Rc<RefCell<Option<bangumi_api::EpisodeCollection>>>,
    selected_anime_id: Rc<RefCell<Option<String>>>,
}

impl Cuby {
    pub fn new(cli_args: &CliArgs) -> Self {
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
        
        let mut menu_btn = MenuButton::default().with_label("菜单");
        menu_btn.add_choice("注册");
        menu_btn.add_choice("注销");
        menu_btn.add_choice("关于");
        menu_btn.add_choice("退出");
        menu_btn.set_callback(move |m| {
            match m.choice().unwrap().as_str() {
                "注册" => sender.send(Message::Register),
                "注销" => sender.send(Message::Logout),
                "关于" => sender.send(Message::About),
                "退出" => sender.send(Message::Exit),
                _ => (),
            }
        });
        
        let mut btn_b = Button::default().with_label("按钮B");
        btn_b.emit(sender, Message::ButtonB);
          let mut btn_a = Button::default().with_label("按钮A");
        btn_a.emit(sender, Message::ButtonA);
        
        let mut search_input = Input::default(); 
        let mut search_btn = Button::default().with_label("搜索");
        search_btn.emit(sender, Message::Search);

        top_flex.fixed(&search_input, 345); 
        top_flex.fixed(&search_btn, 50);
        top_flex.fixed(&menu_btn, 60);
        top_flex.end();
        main_flex.fixed(&top_flex, 30);
        
        // 中区域：文件列表和搜索列表，比例3:2
        let mut mid_flex = Flex::default().row();
        
        let mut file_browser = HoldBrowser::default();
        file_browser.set_selection_color(enums::Color::from_rgb(255, 255, 180)); // 设置为醒目的浅黄色
        
        let mut search_browser = HoldBrowser::default();
        search_browser.set_selection_color(enums::Color::from_rgb(255, 255, 180)); // 设置为醒目的浅黄色
          mid_flex.fixed(&search_browser, 400);
        mid_flex.end();          // 下区域：路径信息Frame和开始按钮
        let mut bottom_flex = Flex::default().row();        let mut info_frame = Frame::default().with_label("就绪 - 请选择文件夹");
        info_frame.set_align(enums::Align::Left | enums::Align::Inside);
        
        // 创建一个垂直布局来包含按钮和底部边距
        let mut btn_container = Flex::default().column();
        let mut start_btn = Button::default().with_label("开始");
        start_btn.emit(sender, Message::Start);
        
        let bottom_margin = Frame::default();
        btn_container.fixed(&bottom_margin, 2);
        btn_container.end();        // 添加一个空白 Frame 作为右侧边距
        let right_margin = Frame::default();
        
        // 固定宽度
        bottom_flex.fixed(&btn_container, 60);
        bottom_flex.fixed(&right_margin, 5);
        
        bottom_flex.end();
        main_flex.fixed(&bottom_flex, 30);
        
        main_flex.end();
        
        wind.resizable(&main_flex);
        wind.end();
        wind.show();
          // 设置浏览器的回调函数来处理点击事件
        file_browser.set_callback(move |_| {
            // 这里可以发送一个消息来更新信息显示
        });
          search_browser.set_callback(move |_| {
            // 这里可以发送一个消息来更新信息显示
        });
        
        // 初始化静态路径变量
        BASE_PATH.get_or_init(|| Mutex::new(String::new()));
        ANIME_PATH.get_or_init(|| Mutex::new(String::new()));        // 从命令行参数初始化路径
        if let Some(ref base_path_str) = cli_args.base_path {
            if let Some(mutex) = BASE_PATH.get() {
                if let Ok(mut base_path) = mutex.lock() {
                    *base_path = base_path_str.clone();
                    // 加载文件到文件浏览器
                    load_files_to_file_browser(base_path_str, &mut file_browser);
                      // 从路径中提取番剧名并填充到搜索框
                    if let Some(folder_name) = std::path::Path::new(base_path_str).file_name().and_then(|n| n.to_str()) {
                        if let Some(extracted_name) = extract_anime_name_regex(folder_name) {
                            search_input.set_value(&extracted_name);
                        }
                    }
                }
            }
        }
        
        if let Some(ref anime_path_str) = cli_args.anime_path {
            if let Some(mutex) = ANIME_PATH.get() {
                if let Ok(mut anime_path) = mutex.lock() {
                    *anime_path = anime_path_str.clone();
                }
            }
        }
        
        // 根据命令行参数更新初始信息显示
        let initial_message = match (&cli_args.base_path, &cli_args.anime_path) {
            (Some(base), Some(anime)) => format!("已从命令行加载路径 - Base: {} | Anime: {}", base, anime),
            (Some(base), None) => format!("已从命令行加载 Base Path: {} - 请选择 Anime Path", base),
            (None, Some(anime)) => format!("已从命令行加载 Anime Path: {} - 请选择 Base Path", anime),
            (None, None) => "就绪 - 请选择文件夹".to_string(),
        };
        info_frame.set_label(&initial_message);        Self {
            app,
            wind,
            file_browser,
            search_browser,
            search_input,
            info_frame,
            receiver,
            // 初始化搜索相关数据
            search_results: Rc::new(RefCell::new(None)),
            episode_list: Rc::new(RefCell::new(None)),
            selected_anime_id: Rc::new(RefCell::new(None)),
        }
    }

    /// 内部鼠标事件处理方法
    fn handle_mouse_events_internal(&mut self) {
        let event = app::event();        // 只在相关事件时处理，减少频繁触发
        if matches!(event, Event::Enter | Event::Push) {
            if let Some(widget) = app::belowmouse::<HoldBrowser>() {
                if widget.as_widget_ptr() == self.file_browser.as_widget_ptr() {                    // 鼠标在文件列表，显示base_path
                    if let Some(mutex) = BASE_PATH.get() {
                        if let Ok(base_path) = mutex.lock() {
                            if !base_path.is_empty() {
                                let display_text = format!("Base Path: {}", base_path);
                                self.info_frame.set_label(&display_text);
                            } else {
                                self.info_frame.set_label("Base Path: 未设置 - 请点击按钮B选择路径");
                            }
                        } else {
                            self.info_frame.set_label("Base Path: 锁定失败");
                        }
                    } else {
                        self.info_frame.set_label("Base Path: 未初始化 - 请点击按钮B选择路径");
                    }
                    self.info_frame.redraw();
                    self.wind.redraw();
                } else if widget.as_widget_ptr() == self.search_browser.as_widget_ptr() {                    // 鼠标在搜索列表，显示anime_path
                    if let Some(mutex) = ANIME_PATH.get() {
                        if let Ok(anime_path) = mutex.lock() {
                            if !anime_path.is_empty() {
                                let display_text = format!("Anime Path: {}", anime_path);
                                self.info_frame.set_label(&display_text);
                            } else {
                                self.info_frame.set_label("Anime Path: 未设置 - 请点击按钮A选择路径");
                            }
                        } else {
                            self.info_frame.set_label("Anime Path: 锁定失败");
                        }
                    } else {
                        self.info_frame.set_label("Anime Path: 未初始化 - 请点击按钮A选择路径");
                    }
                    self.info_frame.redraw();
                    self.wind.redraw();}
            }
        }
    }

    pub fn run(mut self) {
        while self.app.wait() {
            if let Some(msg) = self.receiver.recv() {
                match msg {
                    Message::Register => {
                        register_context_menu();
                    },
                    Message::Logout => {
                        unregister_context_menu();
                    },
                    Message::About => {
                        handle_about_menu();
                    },
                    Message::Exit => {
                        self.wind.hide();
                        break;
                    },                    Message::ButtonA => {
                        update_path(&ANIME_PATH,"select anime_path");
                        if let Some(mutex) = ANIME_PATH.get() {
                            if let Ok(anime_path) = mutex.lock() {
                                self.info_frame.set_label(&format!("Anime Path: {}", anime_path));
                                self.info_frame.redraw();
                                self.wind.redraw();
                            }
                        }
                    },                    Message::ButtonB => {
                        update_path(&BASE_PATH,"select base_path");
                        
                        if let Some(mutex) = BASE_PATH.get() {
                            if let Ok(base_path) = mutex.lock() {
                                self.info_frame.set_label(&format!("Base Path: {}", base_path));
                                self.info_frame.redraw();
                                self.wind.redraw();
                                load_files_to_file_browser(&base_path, &mut self.file_browser);                                // 从路径中提取番剧名并填充到搜索框
                                if let Some(folder_name) = std::path::Path::new(&*base_path).file_name().and_then(|n| n.to_str()) {
                                    if let Some(extracted_name) = extract_anime_name_regex(folder_name) {
                                        self.search_input.set_value(&extracted_name);
                                    }
                                }
                            }
                        }
                    },Message::Search => {
                    let query = self.search_input.value();
                    if query.is_empty() {
                        self.search_browser.clear();
                        *self.search_results.borrow_mut() = None;
                        self.info_frame.set_label("搜索框为空");
                    } else {
                        self.info_frame.set_label(&format!("搜索中: {}", query));
                        self.handle_search(&query);
                    }
                    self.info_frame.redraw();
                    self.wind.redraw();
                },                    Message::Start => {
                        self.handle_start_button();
                        self.info_frame.redraw();
                        self.wind.redraw();
                    },
                }            } else {
                // 如果没有消息，检查是否有浏览器事件
                if let Some(widget) = app::belowmouse::<HoldBrowser>() {
                    let event = app::event();
                    
                    if widget.as_widget_ptr() == self.file_browser.as_widget_ptr() {
                        if handle_browser_events(&mut self.file_browser, event) {
                            // 事件已处理，可以根据需要重绘
                        }
                    } else if widget.as_widget_ptr() == self.search_browser.as_widget_ptr() {
                        // 处理搜索浏览器的双击事件
                        if event == Event::Push && app::event_clicks() {
                            self.handle_search_results_double_click();
                            self.info_frame.redraw();
                            self.wind.redraw();
                        }
                    }
                }
                
                // 检查鼠标事件并更新路径信息显示
                self.handle_mouse_events_internal();
            }
        }
    }    /// 处理搜索逻辑并更新UI
    fn handle_search(&mut self, query: &str) {
        match <Vec<bangumi_api::BangumiSubject> as bangumi_api::ResourceFetcher<&str>>::fetch(query) {
            Ok(subjects) => {
                if subjects.is_empty() {
                    self.search_browser.clear();
                    *self.search_results.borrow_mut() = None;
                    self.info_frame.set_label(&format!("未找到与\"{}\"相关的番剧", query));
                } else {
                    self.search_browser.clear();
                    for subject in &subjects {
                        self.search_browser.add(&format!("{} ({})", subject.name_cn, subject.name));
                    }
                    *self.search_results.borrow_mut() = Some(subjects);
                    self.info_frame.set_label(&format!("找到 {} 个搜索结果", self.search_browser.size()));
                }
            }
            Err(err_msg) => {
                self.search_browser.clear();
                *self.search_results.borrow_mut() = None;
                self.info_frame.set_label(&format!("搜索失败: {}", err_msg));
            }
        }
    }

    /// 处理搜索结果双击事件
    fn handle_search_results_double_click(&mut self) {
        // 确保是双击事件
        if !app::event_clicks() {
            return;
        }

        let line = self.search_browser.value();
        if line <= 0 || line > self.search_browser.size() {
            return;
        }

        // 从搜索结果中获取选定番剧的ID
        let subject_id_opt = self.search_results.borrow().as_ref().and_then(|subjects| {
            let idx = (line as usize) - 1;
            subjects.get(idx).map(|subject| subject.id)
        });        match subject_id_opt {
            Some(subject_id) => {
                match <bangumi_api::EpisodeCollection as bangumi_api::ResourceFetcher<u64>>::fetch(subject_id) {
                    Ok(ep_collection) => {
                        *self.episode_list.borrow_mut() = Some(ep_collection.clone());
                        *self.selected_anime_id.borrow_mut() = Some(subject_id.to_string());
                        
                        self.search_browser.clear();
                        if !ep_collection.episodes.is_empty() {
                            for ep in &ep_collection.episodes {
                                let name = if !ep.name_cn.is_empty() {
                                    ep.name_cn.clone()
                                } else {
                                    ep.name.clone()
                                };
                                let display_text = format!("Ep.{:02} - {}", ep.sort, name);
                                self.search_browser.add(&display_text);
                            }
                            self.info_frame.set_label(&format!("已加载 {} 集剧集信息", ep_collection.episodes.len()));
                        } else {
                            self.search_browser.add("未能获取到剧集信息或剧集列表为空");
                            self.info_frame.set_label("剧集列表为空");
                        }
                    }
                    Err(err_msg) => {
                        self.search_browser.clear();
                        self.search_browser.add(&format!("获取剧集列表失败: {}", err_msg));
                        *self.episode_list.borrow_mut() = None;
                        *self.selected_anime_id.borrow_mut() = None;
                        self.info_frame.set_label("获取剧集信息失败");
                    }
                }
            }
            None => {
                self.search_browser.clear();
                self.search_browser.add("选择无效或数据不一致");
                *self.episode_list.borrow_mut() = None;
                *self.selected_anime_id.borrow_mut() = None;
                self.info_frame.set_label("选择无效");
            }
        }
    }    /// 处理完成按钮逻辑
    fn handle_start_button(&mut self) {
        // 验证前置条件
        if self.episode_list.borrow().is_none() {
            self.info_frame.set_label("错误: 请先搜索并选择番剧");
            return;
        }        // 验证路径
        let (base_path_str, anime_path_str) = match validate_operation_paths() {
            Ok((base, anime)) => (base, anime),
            Err(err) => {
                self.info_frame.set_label(&err);
                return;
            }
        };

        // 收集源文件
        let source_files = collect_source_files_from_browser(&self.file_browser);
        if source_files.is_empty() {
            self.info_frame.set_label("错误: 源文件夹中没有视频文件");
            return;
        }

        // 获取剧集数据
        let ep_collection = match self.episode_list.borrow().as_ref() {
            Some(ep_data) => ep_data.clone(),
            None => {
                self.info_frame.set_label("错误: 剧集数据为空");
                return;
            }
        };

        // 获取番剧名称和年份
        let (anime_display_name, year) = match self.get_selected_anime_details(&ep_collection) {
            Ok((name, year)) => (name, year),
            Err(err) => {
                self.info_frame.set_label(&err);
                return;
            }
        };

        // 创建目标目录
        let target_anime_dir = match self.prepare_target_directory(&anime_path_str, &anime_display_name, &year) {
            Ok(dir) => dir,
            Err(err) => {
                self.info_frame.set_label(&err);
                return;
            }
        };        // 执行硬链接
        let formatted_episode_names = ep_collection.get_formatted_names();
        let (successful, failed, errors) = self.execute_file_renaming(
            &source_files,
            &base_path_str,
            &formatted_episode_names,
            &target_anime_dir,
        );

        // 重新加载文件列表
        load_files_to_file_browser(&base_path_str, &mut self.file_browser);
        
        // 显示结果
        self.display_rename_summary(successful, failed, &errors);
          // 更新状态信息
        if failed == 0 {
            self.info_frame.set_label(&format!("硬链接完成: {} 个文件成功", successful));
        } else {
            self.info_frame.set_label(&format!("硬链接完成: {} 成功, {} 失败", successful, failed));
        }
    }    /// 获取选定的番剧名称和年份
    fn get_selected_anime_details(&self, ep_collection: &bangumi_api::EpisodeCollection) -> Result<(String, String), String> {
        let year = ep_collection.year.to_string();

        // 检查是否有选中的番剧ID
        let selected_anime_id_opt = self.selected_anime_id.borrow();
        if selected_anime_id_opt.is_none() {
            return Err("错误: 请先搜索并选择番剧".to_string());
        }

        let selected_id_str = selected_anime_id_opt.as_ref().unwrap();
        let selected_id: u64 = selected_id_str.parse()
            .map_err(|_| "错误: 番剧ID格式无效".to_string())?;

        // 从搜索结果中找到对应的番剧
        let search_results_opt = self.search_results.borrow();
        match search_results_opt.as_ref() {
            Some(subjects) => {
                if let Some(selected_subject) = subjects.iter().find(|subject| subject.id == selected_id) {
                    let anime_display_name = if !selected_subject.name_cn.is_empty() {
                        selected_subject.name_cn.clone()
                    } else {
                        selected_subject.name.clone()
                    };
                    Ok((anime_display_name, year))
                } else {
                    Err("错误: 在搜索结果中未找到选中的番剧".to_string())
                }
            }
            None => Err("错误: 搜索结果为空".to_string()),
        }
    }/// 准备目标目录
    fn prepare_target_directory(&self, anime_path_root_str: &str, anime_display_name: &str, year: &str) -> Result<std::path::PathBuf, String> {
        let cleaned_anime_name_for_folder = replace_invalid_chars(anime_display_name);
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
        target_anime_dir: &std::path::Path,    ) -> (usize, usize, Vec<String>) {
        let mut successful_links = 0;
        let mut failed_links = 0;
        let mut errors_log = Vec::new();

        if source_files.len() > formatted_episode_names.len() {
            let msg = format!(
                "警告: 选择的文件数量 ({}) 多于剧集数量 ({}). 是否继续?",
                source_files.len(),
                formatted_episode_names.len()
            );            if dialog::choice2_default(&msg, "继续", "取消", "") != Some(0) {
                errors_log.push("操作被用户取消：文件数量多于剧集数量".to_string());
                return (successful_links, failed_links, errors_log);
            }
        }

        for (i, source_file_name_str) in source_files.iter().enumerate() {
            if i >= formatted_episode_names.len() {
                let err_msg = format!("跳过文件 '{}': 超出剧集范围", source_file_name_str);
                errors_log.push(err_msg);
                continue;
            }

            let source_file_path = std::path::Path::new(base_path_str).join(source_file_name_str);
            let original_extension = source_file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
              let cleaned_episode_name_part = replace_invalid_chars(&formatted_episode_names[i]);
            let new_file_name_str = if original_extension.is_empty() {
                cleaned_episode_name_part.clone()
            } else {
                format!("{}.{}", cleaned_episode_name_part, original_extension)
            };
            
            let target_file_path = target_anime_dir.join(&new_file_name_str);

            if source_file_path == target_file_path {
                let msg = format!("跳过文件 '{}': 源与目标相同", source_file_name_str);
                errors_log.push(msg);
                continue;
            }
              if target_file_path.exists() {
                let msg = format!("跳过文件 '{}': 目标已存在", source_file_name_str);
                errors_log.push(msg);
                failed_links += 1;
                continue;
            }

            match std::fs::hard_link(&source_file_path, &target_file_path) {
                Ok(_) => {
                    successful_links += 1;
                }Err(e) => {
                    let err_msg = format!("失败: '{}', 错误: {}", source_file_name_str, e);
                    errors_log.push(err_msg);
                    failed_links += 1;
                }
            }
        }
        (successful_links, failed_links, errors_log)
    }    /// 显示硬链接操作总结
    fn display_rename_summary(&self, successful_links: usize, failed_links: usize, errors_log: &[String]) {
        let mut summary_message = format!("硬链接完成报告:\n成功: {}\n失败: {}", successful_links, failed_links);
        if !errors_log.is_empty() {
            summary_message.push_str("\n\n详细信息:\n");
            summary_message.push_str(&errors_log.join("\n"));
        }
        dialog::message_default(&summary_message);
    }

    // ...existing code...
}
// main函数
fn main() {
    // 解析命令行参数
    let cli_args = CliArgs::parse();
    
    let app = Cuby::new(&cli_args);
    app.run();
}
// 
use std::env;
use winreg::enums::*; 
use winreg::RegKey;

// 支持的视频文件扩展名常量
static VIDEO_EXTENSIONS: &[&str] = &["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm"];

/// 替换文件名中的特殊字符
// 直接编码成URL格式
// URL.encode() 是一个示例函数，实际实现需要根据具体需求来
use urlencoding;

/// 替换文件名中的特殊字符
pub fn replace_special_characters(file_name: &str) -> String {
    urlencoding::encode(file_name).to_string()
}

// 需要添加 regex 依赖: 
use regex::Regex;

/// 使用正则表达式从路径中提取番剧名
pub fn extract_anime_name_regex(file_name: &str) -> Option<String> {
    if file_name.starts_with('[') {
        // 处理带标签格式
        // 提取两种情况:
        // 1. [组名][番剧名][其他]
        // 2. [Rev][组名][番剧名][其他]
        let re = Regex::new(r"^\[(?:Rev|rev)\]\[[^\]]+\]\[([^\]]+)\]|^\[[^\]]+\]\[([^\]]+)\]").unwrap();
        if let Some(caps) = re.captures(file_name) {
            // 第一个捕获组是针对有[Rev]的情况，第二个是没有的情况
            return caps.get(1).or_else(|| caps.get(2))
                .map(|m| m.as_str().trim().to_string());
        }
        
        // 处理 [组名]番剧名[其他] 格式
        let re2 = Regex::new(r"^\[[^\]]+\]\s*([^[]+)").unwrap();
        if let Some(caps) = re2.captures(file_name) {
            return caps.get(1).map(|m| m.as_str().trim().to_string());
        }
    } else {
        // 处理下划线格式
        return file_name.split_once('_')
            .map(|(before, _)| before.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    None
}

/// 注册右键菜单
pub fn register_context_menu() {
    // 获取当前可执行文件路径
    match env::current_exe() {
        Ok(exe_path_buf) => {
            let exe_path = exe_path_buf.to_string_lossy().to_string();
            let mut errors = Vec::new();
            let hkey_classes_root = RegKey::predef(HKEY_CLASSES_ROOT);
            let key_name = "Add To Cuby";
            let shell_path = "Directory\\shell";
            
            // 创建右键菜单项
            let command_val = format!("\"{}\" -b \"%1\" -a \"%1\\anime\"", exe_path);
            match hkey_classes_root.create_subkey(format!("{}\\{}", shell_path, key_name)) {
                Ok((key, _)) => {
                    // 设置菜单属性
                    match (
                        key.set_value("", &key_name),
                        key.set_value("Icon", &format!("\"{}\",0", exe_path))
                    ) {
                        (Ok(_), Ok(_)) => {},
                        (Err(e), _) => errors.push(format!("设置菜单默认值失败: {}", e)),
                        (_, Err(e)) => errors.push(format!("设置菜单图标失败: {}", e)),
                    }
                    
                    // 创建命令子键
                    match key.create_subkey("command") {
                        Ok((cmd_key, _)) => {
                            if let Err(e) = cmd_key.set_value("", &command_val) {
                                errors.push(format!("设置命令失败: {}", e));
                            }
                        },
                        Err(e) => errors.push(format!("创建命令子键失败: {}", e)),
                    }
                },
                Err(e) => errors.push(format!("创建菜单主键失败: {}", e)),
            }
            
            // 显示结果
            if errors.is_empty() {
                dialog::message_default("注册表项已成功添加");
            } else {
                dialog::message_default(&format!("注册表操作时发生错误:\n{}\n\n请确保以管理员身份运行本程序", errors.join("\n")));
            }
        },
        Err(e) => dialog::message_default(&format!("获取程序路径失败: {}", e)),
    }
}

/// 注销文件夹右键菜单
pub fn unregister_context_menu() {
    let hkey_classes_root = RegKey::predef(HKEY_CLASSES_ROOT);
    let key_name = "Add To Cuby";
    let key_path = format!("Directory\\shell\\{}", key_name);
    
    // 尝试删除右键菜单项
    match hkey_classes_root.delete_subkey_all(key_path) {
        Ok(_) => dialog::message_default("相关注册表项已成功删除"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            dialog::message_default("未找到相关的注册表项，无需注销");
        },
        Err(e) => {
            dialog::message_default(&format!("注销操作时发生错误: {}\n\n请确保以管理员身份运行本程序", e));
        },
    }
}
use fltk::dialog::FileDialogType;
/// 打开文件夹选择器
pub fn open_folder_selector(title: &str) -> Option<String> {
    let mut path = String::new();
    let mut chooser = fltk::dialog::NativeFileChooser::new(FileDialogType::BrowseDir);
    chooser.set_title(title);
    chooser.show();
    if chooser.filename().exists() {
        if let Some(selected_path) = chooser.filename().to_str() {
            path = selected_path.to_string();
        }
    }
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}
/// 通用的路径更新函数
pub fn update_path(lock: &OnceLock<Mutex<String>>, title: &str) -> Option<String> {
    // 打开文件夹选择对话框
    let selected_path = open_folder_selector(title)?;
    
    // 获取已存在的Mutex或初始化一个新的
    let mutex = lock.get_or_init(|| Mutex::new(String::new()));
    
    // 尝试获取锁并更新值
    match mutex.lock() {
        Ok(mut path) => {
            *path = selected_path.clone();
            Some(selected_path)
        },
        Err(_) => None // 获取锁失败
    }
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
        if VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
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
fn handle_browser_drag_event(browser: &mut HoldBrowser, drag_item_ptr: *mut i32) -> bool {
    let _y = app::event_y(); // y 坐标可能用于更精确的行计算，但当前未使用
    let current_item_under_mouse = browser.value(); // 获取鼠标当前悬停或选中的行

    let initial_drag_item_val = unsafe { *drag_item_ptr }; // 读取当前拖拽项的值

    if initial_drag_item_val < 0 { // 如果 drag_item 小于0，表示这是拖拽的开始
        unsafe { *drag_item_ptr = current_item_under_mouse; } // 记录开始拖拽的项
        return true;
    }
    
    // 如果鼠标下的项有效，并且不是当前正在拖拽的项
    if current_item_under_mouse > 0 && current_item_under_mouse != initial_drag_item_val {
        let text1 = browser.text(initial_drag_item_val).unwrap_or_default().to_string();
        let text2 = browser.text(current_item_under_mouse).unwrap_or_default().to_string();
          
        browser.set_text(initial_drag_item_val, &text2); // 将原拖拽项的内容设置为新位置项的内容
        browser.set_text(current_item_under_mouse, &text1); // 将新位置项的内容设置为原拖拽项的内容
          
        unsafe { *drag_item_ptr = current_item_under_mouse; } // 更新拖拽的项为当前鼠标下的项
        browser.select(current_item_under_mouse); // 保持选中新位置的项
        browser.redraw();
        return true;
    }
    true // 即使没有发生交换，也处理了拖拽事件
}

/// 处理文件浏览器中的 Released 事件
fn handle_browser_released_event(drag_item_ptr: *mut i32) -> bool {
    unsafe { *drag_item_ptr = -1; } // 重置拖拽项
    true
}

/// 处理文件浏览器事件
pub fn handle_browser_events(browser: &mut HoldBrowser, event: Event) -> bool {
    // 使用静态变量来跟踪拖放项
    static mut DRAG_ITEM: i32 = -1;
    
    match event {
        Event::Drag => handle_browser_drag_event(browser, std::ptr::addr_of_mut!(DRAG_ITEM)),
        Event::Released => handle_browser_released_event(std::ptr::addr_of_mut!(DRAG_ITEM)),
        _ => false,
    }
}

/// 处理关于菜单
fn handle_about_menu() {
    let repo_url = "https://github.com/uuzp/bgm_rename_cuby";
    let _ = webbrowser::open(repo_url);
}

/// 验证操作路径
pub fn validate_operation_paths() -> Result<(String, String), String> {
    let base_path_str = if let Some(mutex) = BASE_PATH.get() {
        if let Ok(path) = mutex.lock() {
            if path.is_empty() {
                return Err("错误: 未设置源文件路径（B按钮）".to_string());
            }
            path.clone()
        } else {
            return Err("错误: 无法访问源文件路径".to_string());
        }
    } else {
        return Err("错误: 未初始化源文件路径".to_string());
    };

    let anime_path_str = if let Some(mutex) = ANIME_PATH.get() {
        if let Ok(path) = mutex.lock() {
            if path.is_empty() {
                return Err("错误: 未设置目标位置路径（A按钮）".to_string());
            }
            path.clone()
        } else {
            return Err("错误: 无法访问目标位置路径".to_string());
        }
    } else {
        return Err("错误: 未初始化目标位置路径".to_string());
    };

    Ok((base_path_str, anime_path_str))
}

/// 替换文件名中的特殊字符
pub fn replace_invalid_chars(s: &str) -> String {
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

/// 从路径中提取番剧名的函数
pub fn extract_anime_name_from_path(path_str: &str) -> Option<String> {
    let path = std::path::Path::new(path_str);
    let dir_name = path.file_name()?.to_str()?;

    // 优先尝试从带标签的格式提取
    if let Some(name) = extract_from_tagged_format(dir_name) {
        return Some(name);
    }

    // 回退到简单格式
    extract_from_simple_format(dir_name)
}

/// 从带标签的格式提取番剧名，例如 [组名][状态]番剧名
fn extract_from_tagged_format(dir_name: &str) -> Option<String> {
    if !dir_name.starts_with('[') {
        return None;
    }
    
    // 确定要提取的部分索引
    let cont = if dir_name.len() >= 5 && 
               (dir_name[1..4].eq_ignore_ascii_case("rev") || 
                dir_name[1..4].eq_ignore_ascii_case("raw")) && 
               dir_name.chars().nth(4) == Some(']') {
        3 // 对应 [rev][组名]番剧名 或 [raw][组名]番剧名 格式
    } else {
        2 // 对应 [组名][状态]番剧名 格式
    };

    // 分割字符串并提取相应部分
    let parts: Vec<String> = dir_name
        .replace(']', "[") // 统一分隔符
        .split('[')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    // 确保有足够的部分且索引有效
    if parts.len() >= cont {
        Some(parts[cont - 1].clone())
    } else {
        None
    }
}

/// 从简单格式提取番剧名，例如 番剧名_其他信息
fn extract_from_simple_format(dir_name: &str) -> Option<String> {
    dir_name
        .split('_')
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

