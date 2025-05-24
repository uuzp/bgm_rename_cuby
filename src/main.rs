use fltk::dialog;
use fltk::enums::{Event, Key};
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

static BASE_PATH: OnceLock<Mutex<String>> = OnceLock::new();
static ANIME_PATH: OnceLock<Mutex<String>> = OnceLock::new();

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
}

impl Cuby {
    pub fn new() -> Self {
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
        
        let search_input = Input::default(); 
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
        
        Self {
            app,
            wind,
            file_browser,
            search_browser,
            search_input,
            info_frame,
            receiver,
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
                                load_files_to_file_browser(&base_path, &mut self.file_browser);
                            }
                        }
                    },Message::Search => {
                        let query = self.search_input.value();
                        self.info_frame.set_label(&format!("搜索: {}", query));
                        self.info_frame.redraw();
                        self.wind.redraw();
                    },
                    Message::Start => {
                        self.info_frame.set_label("开始处理");
                        self.info_frame.redraw();
                        self.wind.redraw();
                    },
                }            } else {
                // 如果没有消息，检查是否有 file_browser 的事件
                if let Some(widget) = app::belowmouse::<HoldBrowser>() {
                    if widget.as_widget_ptr() == self.file_browser.as_widget_ptr() {
                        let event = app::event();
                        if handle_browser_events(&mut self.file_browser, event) {
                            // 事件已处理，可以根据需要重绘
                            // self.file_browser.redraw();
                            // self.wind.redraw();
                        }
                    }
                }
                
                // 检查鼠标事件并更新路径信息显示
                self.handle_mouse_events_internal();
            }
        }
    }
}
// main函数
fn main() {
    let app = Cuby::new();
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


/// 验证操作路径
pub fn validate_operation_paths() -> bool {
 check(&BASE_PATH) && check(&ANIME_PATH)
}

fn check(lcok: &OnceLock<Mutex<String>>) -> bool {
    // 检查路径是否有效
    match lcok.get() {
        Some(mutex) => match mutex.lock() {
            Ok(path) => !path.is_empty(),
            _ => false
        },
        None => false
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
