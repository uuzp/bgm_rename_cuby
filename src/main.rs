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
struct CliArgs {
    /// 源文件路径（包含视频文件的文件夹）
    #[arg(short = 'b', long)]
    base_path: Option<String>,

    /// 目标文件路径（重命名后文件存放的文件夹）
    #[arg(short = 'a', long)]
    anime_path: Option<String>,
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
    search_results: Rc<RefCell<Option<Vec<bangumi_api::Subject>>>>,
    episode_list: Rc<RefCell<Option<bangumi_api::Episodes>>>,

    selected_anime_id: Rc<RefCell<Option<String>>>,
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
        
        let mut btn_b = Button::default().with_label("|🌀|");
        btn_b.emit(sender, Message::ButtonB);
          let mut btn_a = Button::default().with_label("|🎬|");
        btn_a.emit(sender, Message::ButtonA);
        
        let mut search_input = Input::default(); 
        let mut search_btn = Button::default().with_label("🔍");
        search_btn.emit(sender, Message::Search);

        top_flex.fixed(&search_input, 345); 
        top_flex.fixed(&search_btn, 50);
        top_flex.fixed(&menu_btn, 60);
        top_flex.end();
        main_flex.fixed(&top_flex, 30);
        
        // 中区域：文件列表和搜索列表
        let mut mid_flex = Flex::default().row();
        
        let mut file_browser = HoldBrowser::default();
        file_browser.set_selection_color(enums::Color::from_hex_str("#9999FF").unwrap()); 
        
        let mut search_browser = HoldBrowser::default();
        search_browser.set_selection_color(enums::Color::from_hex_str("#39C5BB").unwrap()); 
        mid_flex.fixed(&search_browser, 400);
        mid_flex.end();          
        // 下区域：路径信息Frame和开始按钮
        let mut bottom_flex = Flex::default().row();        
        let mut info_frame = Frame::default().with_label("@");
        info_frame.set_align(enums::Align::Left | enums::Align::Inside);
       
        // 创建一个垂直布局来包含按钮和底部边距
        let mut btn_container = Flex::default().column();
        let mut start_btn = Button::default().with_label("开始");
        start_btn.emit(sender, Message::Start);
        
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
           
        // 初始化静态路径变量
        BASE_PATH.get_or_init(|| Mutex::new(String::new()));
        ANIME_PATH.get_or_init(|| Mutex::new(String::new()));        
        
        // 从命令行参数初始化路径
        if let Some(ref base_path_str) = cli_args.base_path {
            set_static_path(&BASE_PATH, base_path_str);
            // 加载文件到文件浏览器
            load_files_to_file_browser(base_path_str, &mut file_browser);
            // 从路径中提取番剧名并填充到搜索框
            if let Some(folder_name) = std::path::Path::new(base_path_str).file_name().and_then(|n| n.to_str()) {
                if let Some(extracted_name) = extract_anime_name_regex(folder_name) {
                    search_input.set_value(&extracted_name);
                }
            }
        }
        
        if let Some(ref anime_path_str) = cli_args.anime_path {
            set_static_path(&ANIME_PATH, anime_path_str);
        }
        
        Self {
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

    fn run(mut self) {
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
                    },                    
                    Message::ButtonA => {
                        self.handle_button_a();
                    }
                    Message::ButtonB => {
                        self.handle_button_b();
                    },
                    Message::Search => {
                        self.handle_search_button();
                    },                    
                    Message::Start => {
                        self.handle_start_button();
                        self.file_browser.clear();
                        self.search_browser.clear();
                        self.search_results.borrow_mut().take();
                    },
                }
            } else {
                // 如果没有消息，检查是否有浏览器事件
                if let Some(widget) = app::belowmouse::<HoldBrowser>() {
                    let event = app::event();
                    
                    match (widget.as_widget_ptr() == self.file_browser.as_widget_ptr(), event) {
                        // 文件浏览器事件
                        (true, Event::Push) => {
                            self.handle_browser_click(true); // true 表示文件浏览器
                        }
                        (true, drag_event) if handle_browser_events(&mut self.file_browser, drag_event) => {
                            // 拖拽事件已处理
                        }
                        // 搜索浏览器事件
                        (false, Event::Push) if app::event_clicks() => {
                            // 双击事件
                            self.handle_search_results_double_click();
                            self.info_frame.redraw();
                            self.wind.redraw();
                        }
                        (false, Event::Push) => {
                            // 单击事件
                            self.handle_browser_click(false); // false 表示搜索浏览器
                        }
                        _ => {
                            // 其他事件不处理
                        }
                    }
                }
                
            }
        }
    }

    /// 处理按钮A点击事件
    fn handle_button_a(&mut self) {
        update_path(&ANIME_PATH, "select ANIME_PATH");
        if let Some(mutex) = ANIME_PATH.get() {
            if let Ok(anime_path) = mutex.lock() {
                self.info_frame.set_label(&format!("@ {}", anime_path));
            }
        }
    }

    /// 处理按钮B点击事件
    fn handle_button_b(&mut self) {
        update_path(&BASE_PATH, "select BASE_PATH");
        
        if let Some(mutex) = BASE_PATH.get() {
            if let Ok(base_path) = mutex.lock() {
                self.info_frame.set_label(&format!("@ {}", base_path));
                load_files_to_file_browser(&base_path, &mut self.file_browser);
                // 从路径中提取番剧名并填充到搜索框
                if let Some(folder_name) = std::path::Path::new(&*base_path).file_name().and_then(|n| n.to_str()) {
                    if let Some(extracted_name) = extract_anime_name_regex(folder_name) {
                        self.search_input.set_value(&extracted_name);
                    }
                }
            }
        }
    }

    /// 处理搜索按钮点击事件
    fn handle_search_button(&mut self) {
        let query = self.search_input.value();
        if query.is_empty() {
            self.info_frame.set_label("无关键词");
        } else {
            self.handle_search(&query);
        }
    }

    /// 处理浏览器单击事件
    fn handle_browser_click(&mut self, is_file_browser: bool) {
        if is_file_browser {
            // 文件浏览器被点击，显示 BASE_PATH
            if let Some(mutex) = BASE_PATH.get() {
                if let Ok(base_path) = mutex.lock() {
                    self.info_frame.set_label(&format!("@ {}", base_path));
                }
            }
        } else {
            // 搜索浏览器被点击，显示 ANIME_PATH
            if let Some(mutex) = ANIME_PATH.get() {
                if let Ok(anime_path) = mutex.lock() {
                    self.info_frame.set_label(&format!("@ {}", anime_path));
                }
            }
        }
    }

    /// 处理搜索逻辑并更新UI
    fn handle_search(&mut self, query: &str) {
        match bangumi_api::get::<Vec<bangumi_api::Subject>, &str>(query) {
            Ok(subjects) => {
                if subjects.is_empty() {
                    self.search_browser.clear();
                    *self.search_results.borrow_mut() = None;
                    self.info_frame.set_label(&format!("未找到与\"{}\"相关的番剧", query));
                } else {
                    self.search_browser.clear();
                    self.search_results.borrow_mut().take(); 
                    self.episode_list.borrow_mut().take();
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

        // 检查当前状态：如果已经在显示剧集信息，则不处理双击事件
        if self.episode_list.borrow().is_some() {
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
        });        
        match subject_id_opt {
            Some(subject_id) => {
                match bangumi_api::get::<bangumi_api::Episodes, u64>(subject_id) {
                    Ok(episodes) => {
                        *self.episode_list.borrow_mut() = Some(episodes.clone());
                        *self.selected_anime_id.borrow_mut() = Some(subject_id.to_string());
                        
                        self.search_browser.clear();
                        if !episodes.items.is_empty() {
                            for ep in &episodes.items {
                                let name = if !ep.name_cn.is_empty() {
                                    ep.name_cn.clone()
                                } else {
                                    ep.name.clone()
                                };
                                let display_text = format!("Ep.{:02} - {}", ep.sort, name);
                                self.search_browser.add(&display_text);
                            }
                            self.info_frame.set_label(&format!("已加载 {} 集剧集信息", episodes.items.len()));
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
        // 统一验证
        let (base_path_str, anime_path_str) = match self.is_check() {
            Ok((base, anime)) => (base, anime),
            Err(err) => {
                self.info_frame.set_label(&err);
                return;
            }
        };

        // 收集源文件
        let source_files = collect_source_files_from_browser(&self.file_browser);

        // 获取剧集数据
        let ep_collection = self.episode_list.borrow().as_ref().unwrap().clone();

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
        };

        // 执行操作
        let (successful, failed, errors) = self.execute_file_operations(&source_files, &base_path_str, &ep_collection, &target_anime_dir);

        // 重新加载文件列表并显示结果
        self.post_operation_cleanup(&base_path_str, successful, failed, &errors);
    }

    /// 统一验证函数
    fn is_check(&self) -> Result<(String, String), String> {
        // 验证剧集信息
        if self.episode_list.borrow().is_none() {
            return Err("错误: 请先搜索并选择番剧".to_string());
        }

        // 验证BASE_PATH
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

        // 验证ANIME_PATH
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

    /// 执行文件操作
    fn execute_file_operations(
        &self,
        source_files: &[String],
        base_path_str: &str,
        episodes: &bangumi_api::Episodes,
        target_anime_dir: &std::path::Path,
    ) -> (usize, usize, Vec<String>) {
        let formatted_episode_names = episodes.formatted_names();
        self.execute_file_renaming(source_files, base_path_str, &formatted_episode_names, target_anime_dir)
    }

    /// 操作后清理和结果显示
    fn post_operation_cleanup(&mut self, base_path_str: &str, successful: usize, failed: usize, errors: &[String]) {
        // 重新加载文件列表
        load_files_to_file_browser(base_path_str, &mut self.file_browser);

        // 统计字幕文件处理情况
        let subtitle_success = errors.iter().filter(|msg| msg.contains("字幕文件复制成功")).count();
        let subtitle_failed = errors.iter().filter(|msg| msg.contains("字幕文件复制失败")).count();

        // 简单显示结果统计
        if subtitle_success > 0 || subtitle_failed > 0 {
            self.info_frame.set_label(&format!("完成: 视频 {}成功 {}失败, 字幕 {}成功 {}失败", 
                                              successful, failed, subtitle_success, subtitle_failed));
        } else {
            self.info_frame.set_label(&format!("完成: 视频 {}成功 {}失败", successful, failed));
        }
        self.search_input.set_value(""); // 清空搜索框
    }

    /// 获取选定的番剧名称和年份
    fn get_selected_anime_details(&self, episodes: &bangumi_api::Episodes) -> Result<(String, String), String> {
        let year = episodes.year.to_string();

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
              let cleaned_episode_name_part = clean_filename(&formatted_episode_names[i]);
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
            }            match std::fs::hard_link(&source_file_path, &target_file_path) {
                Ok(_) => {
                    successful_links += 1;
                    
                    // 处理匹配的字幕文件
                    let subtitle_files = find_matching_subtitle_files(source_file_name_str, base_path_str);
                    for (subtitle_file_name, subtitle_ext) in subtitle_files {
                        let source_subtitle_path = std::path::Path::new(base_path_str).join(&subtitle_file_name);
                        
                        // 构造字幕文件的新名称
                        let subtitle_new_name = if subtitle_file_name.starts_with(&format!("{}.", source_file_name_str.rsplit_once('.').map(|(base, _)| base).unwrap_or(source_file_name_str))) {
                            // 带语言标识的字幕文件
                            let video_base = source_file_name_str.rsplit_once('.').map(|(base, _)| base).unwrap_or(source_file_name_str);
                            let subtitle_base = subtitle_file_name.rsplit_once('.').map(|(base, _)| base).unwrap_or(&subtitle_file_name);
                            let language_part = &subtitle_base[video_base.len()..];
                            format!("{}{}.{}", cleaned_episode_name_part, language_part, subtitle_ext)
                        } else {
                            // 完全匹配的字幕文件
                            format!("{}.{}", cleaned_episode_name_part, subtitle_ext)
                        };
                        
                        let target_subtitle_path = target_anime_dir.join(&subtitle_new_name);
                        
                        // 复制字幕文件（因为字幕文件通常较小，且可能会修改内容）
                        if !target_subtitle_path.exists() {
                            match std::fs::copy(&source_subtitle_path, &target_subtitle_path) {
                                Ok(_) => {
                                    // 字幕文件复制成功，记录到日志中
                                    errors_log.push(format!("字幕文件复制成功: '{}' -> '{}'", subtitle_file_name, subtitle_new_name));
                                }
                                Err(e) => {
                                    errors_log.push(format!("字幕文件复制失败: '{}', 错误: {}", subtitle_file_name, e));
                                }
                            }
                        } else {
                            errors_log.push(format!("跳过字幕文件 '{}': 目标已存在", subtitle_file_name));
                        }
                    }
                }
                Err(e) => {
                    let err_msg = format!("失败: '{}', 错误: {}", source_file_name_str, e);
                    errors_log.push(err_msg);
                    failed_links += 1;
                }
            }
        }
        (successful_links, failed_links, errors_log)
    }
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

// 支持的字幕文件扩展名常量
static SUBTITLE_EXTENSIONS: &[&str] = &["srt", "ass", "ssa", "vtt", "sub", "idx", "sup"];

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
        if !SUBTITLE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
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
/// 设置静态路径变量的通用函数
fn set_static_path(lock: &OnceLock<Mutex<String>>, path_str: &str) {
    if let Some(mutex) = lock.get() {
        if let Ok(mut path) = mutex.lock() {
            *path = path_str.to_string();
        }
    }
}

