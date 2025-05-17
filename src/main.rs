#![windows_subsystem = "windows"] // 新增：禁止在 Windows 上显示控制台窗口
// filepath: a:\Dev\PJ\bgm_rename_cuby\src\main.rs
use fltk::{
    app,
    browser::{FileBrowser, MultiBrowser},
    button::Button,
    dialog::FileDialog, // Removed message_default as it's directly used via fltk::dialog::message_default
    enums::{Color, Event}, // Removed Key as it's not used in the provided snippet
    frame::Frame,
    group::Flex,
    input::Input,
    prelude::*,
    window::Window,
};
use std::{
    cell::RefCell,
    path::Path, // 移除未使用的 PathBuf
    rc::Rc,
    env,
    // fs::File, // 移除，如果不再需要生成临时文件
    // io::Write, // 移除，如果不再需要生成临时文件
    // process::{Command, ExitStatus}, // 移除，不再调用外部命令
};
use winreg::enums::*; // 新增：导入 winreg enums
use winreg::RegKey;  // 新增：导入 winreg RegKey

use clap::Parser;
use miniserde::{Deserialize, Serialize};
use miniserde::json;
use minreq;
use urlencoding;
use webbrowser; // 新增：导入 webbrowser crate

// 从lib.rs复制过来，直接在main.rs中定义
#[derive(Deserialize, Serialize)]
struct SearchResult {
    list: Vec<Subject>,
}

#[derive(Deserialize, Serialize)]
struct Subject {
    id: u64,
    name: String,
    name_cn: String,
}

#[derive(Deserialize, Serialize)]
struct EpisodesResult {
    data: Vec<Episode>,
}

#[derive(Deserialize, Serialize)]
struct Episode {
    airdate: String,
    sort: u64,
    name: String,
    name_cn: String,
}

#[derive(Debug, Clone)]
pub struct Bgm {
    pub name: Vec<String>,
    pub id: Vec<String>,
}

impl Bgm {
    pub fn new() -> Self {
        Self {
            name: Vec::new(),
            id: Vec::new(),
        }
    }      pub fn get(mut self, keywords: &str) -> Self {
        // 对搜索关键词进行URL编码
        let encoded_keywords = urlencoding::encode(keywords);
        let url = format!(
            "https://api.bgm.tv/search/subject/{}?type=2&responseGroup=small",
            encoded_keywords
        );
        // 使用 minreq 发送 GET 请求并获取文本
        let response = minreq::get(url).send().unwrap();
        let response_text = response.as_str().unwrap();
        let search_result: SearchResult = json::from_str(&response_text).unwrap();

        // 存储番剧数据到Bgm结构体
        for subject in search_result.list {
            self.id.push(subject.id.to_string());
            self.name.push(
                match subject.name_cn.is_empty() {
                    true => subject.name,
                    false => subject.name_cn,
                }
            );
        }
        self
    }
}

#[derive(Debug, Clone)] // 添加 Clone trait
pub struct Ep {
    pub name: Vec<String>,
    pub year: i32,
}

impl Ep {
    pub fn new() -> Self{
        Self { name: Vec::new(), year: 1970 }
    } 
    
    pub fn get(id: &str) -> Self {
        let url = format!(
            "https://api.bgm.tv/v0/episodes?subject_id={}&type=0&limit=100&offset=0",
            id
        );
        // 使用 minreq 发送 GET 请求
        let response = minreq::get(url)
            .with_header("User-Agent", "uuzp/bgm_rename_cuby")
            .send()
            .unwrap();
        // 获取响应文本并使用 miniserde 解析
        let response_text = response.as_str().unwrap();
        let episodes_result: EpisodesResult = json::from_str(&response_text).unwrap();

        let mut ep_list = Vec::new();
        let mut epn = Vec::new();
        let mut year = 1970; // 默认年份

        // 从第一个剧集中提取年份
        if let Some(first_episode) = episodes_result.data.first() {
             // 安全地解析年份，如果失败则使用默认值
             year = first_episode.airdate.get(0..4).unwrap_or("").parse().unwrap_or(1970);
        }        // 提取剧集编号和名称（优先使用中文名称，如果为空则使用原名）
        for episode in episodes_result.data {
            epn.push(episode.sort);
            // 如果 name_cn 不为空，则使用 name_cn，否则使用 name
            let s = if !episode.name_cn.is_empty() {
                episode.name_cn
            } else {
                episode.name
            };
            // 替换 HTML 实体
            let s = s.replace("<", "＜");
            let s = s.replace(">", "＞");
            ep_list.push(s);
        }

        let mut name = vec![];
        // 计算最大剧集编号所需的位数，用于格式化前导零
        let max_ep_num = epn.iter().max().cloned().unwrap_or(0);
        // 避免 log10(0) 导致 panic
        let num_digits = if max_ep_num == 0 { 1 } else { (max_ep_num as f64).log10() as usize + 1 };

        // 格式化剧集名称
        for i in 0..ep_list.len() {
            // 使用计算出的位数格式化剧集编号，添加前导零
            let ep_num_str = format!("{:0width$}", epn[i], width = num_digits);
            let ep = format!("ep{} - {}", ep_num_str, ep_list[i]);
            name.push(ep);
        }

        Ep { name, year }
    }
}

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const HALF_WIDTH: i32 = WINDOW_WIDTH / 2;

// --- 新增：定义命令行参数 ---
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliArgs {
    /// 设置源文件路径 (B)
    #[clap(short = 'b', long, value_parser)]
    base_path: Option<String>,

    /// 设置目标路径 (A)
    #[clap(short = 'a', long, value_parser)]
    anime_path: Option<String>,
}

// --- 新增：从路径中提取番剧名的函数 ---
fn extract_anime_name_from_path(path_str: &str) -> Option<String> {
    let path = Path::new(path_str);
    let dir_name = path.file_name()?.to_str()?;

    if dir_name.starts_with('[') {
        // 规则 1: 检查首位字符是不是 '['
        let cont = if dir_name.len() >= 5 && // 确保有足够长度访问 dir_name[1..4]
                       (dir_name[1..4].eq_ignore_ascii_case("rev") || 
                        dir_name[1..4].eq_ignore_ascii_case("raw")) && // 添加对 [raw] 的检查
                       dir_name.chars().nth(4) == Some(']') { // 确保是 [rev] 或 [raw] 结束
            3
        } else {
            2
        };

        let parts: Vec<String> = dir_name
            .replace(']', "[") // 将 ']' 替换为 '[' 以便使用单个分隔符
            .split('[')       // 按 '[' 分割
            .filter(|s| !s.trim().is_empty()) // 过滤掉空字符串或仅包含空白的字符串
            .map(|s| s.trim().to_string())    // 去除首尾空白并转换为 String
            .collect();

        if parts.len() >= cont {
            Some(parts[cont - 1].clone()) // cont 是 1-based 索引
        } else {
            None // 没有足够的有效部分
        }
    } else {
        // 规则 2: 取 '_' 前面的所有字符串
        dir_name.split('_').next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }
}

// --- 新增：辅助函数，用于加载文件到 FileBrowser ---
fn load_files_to_file_browser(path_str: &str, browser: &mut FileBrowser) {
    browser.clear();
    if let Ok(entries) = std::fs::read_dir(path_str) {
        for entry in entries.filter_map(Result::ok) {
            let file_path = entry.path();
            if file_path.is_file() {
                if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                    match ext.to_lowercase().as_str() {
                        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => {
                            if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                                browser.add(file_name);
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}

// --- 移除不再使用的 run_reg_command_elevated 函数 ---
// fn run_reg_command_elevated(args: &[String]) -> Result<ExitStatus, std::io::Error> {
//     let mut arg_list_str = String::new();
//     for (i, arg) in args.iter().enumerate() {
//         if i > 0 {
//             arg_list_str.push_str(", ");
//         }
//         // 在 PowerShell 中正确引用参数，特别是包含空格的路径或值
//         // PowerShell 中字符串用单引号，内部单引号用两个单引号转义
//         arg_list_str.push_str(&format!("\'\'\'{}\'\'\'", arg.replace("\'", "\'\'\'\'")));
//     }
//
//     let ps_command_str = format!(
//         "Start-Process reg.exe -ArgumentList ({}) -Verb RunAs -Wait -WindowStyle Hidden",
//         arg_list_str
//     );
//
//     Command::new("powershell.exe")
//         .arg("-NoProfile")
//         .arg("-NonInteractive")
//         .arg("-WindowStyle")
//         .arg("Hidden")
//         .arg("-Command")
//         .arg(&ps_command_str)
//         .status()
//     }
// }

fn main() {
    let cli_args = CliArgs::parse(); // --- 新增：解析命令行参数 ---

    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    let mut wind = Window::new(
        100,
        100,
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        "File Explorer and Search",
    );
    
    // 修改：使用命令行参数初始化路径
    let base_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(cli_args.base_path.clone()));
    let anime_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(cli_args.anime_path.clone()));

    // --- 提前定义按钮，以便在不同 Flex 容器中使用 ---
    let mut btn_choose_base = Button::new(0, 0, 0, 0, "选择源路径 (B)");
    btn_choose_base.set_tooltip("选择源文件夹 (B)");
    let mut btn_choose_anime = Button::new(0, 0, 0, 0, "选择目标路径 (A)");
    btn_choose_anime.set_tooltip("选择目标文件夹 (A)");
    let mut btn_done = Button::new(0, 0, 0, 0, "✔️ 完成"); // 修改：添加图标

    // --- 新增：提前定义搜索控件 ---
    let mut search_input = Input::new(0, 0, 0, 0, "");
    search_input.set_tooltip("输入番剧名称关键字");
    let mut search_button = Button::new(0, 0, 40, 0, "🔎");
    
    let mut main_vertical_flex = Flex::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, "");
    main_vertical_flex.set_type(fltk::group::FlexType::Column); // 垂直排列

    // --- 修改：菜单栏 & 路径栏 UI Elements ---
    const MENU_TRIGGER_HEIGHT: i32 = 30;
    const MENU_ITEMS_PANEL_EXPANDED_HEIGHT: i32 = 35; // Adjusted for a single row of buttons with margin
    const PATH_DISPLAY_PANEL_EXPANDED_HEIGHT: i32 = 30; // Original height of bottom_flex

    let is_menu_expanded = Rc::new(RefCell::new(false));
    let is_path_panel_expanded = Rc::new(RefCell::new(false)); // 新增状态

    // 顶部触发器行 (包含菜单按钮和路径按钮)
    let mut top_triggers_flex = Flex::new(0, 0, WINDOW_WIDTH, MENU_TRIGGER_HEIGHT, "");
    top_triggers_flex.set_type(fltk::group::FlexType::Row);

    let mut menu_trigger_button = Button::new(0, 0, 80, 0, "菜单 ☰"); // 定义菜单按钮
    top_triggers_flex.add(&menu_trigger_button);
    top_triggers_flex.fixed(&menu_trigger_button, 80);

    let mut path_trigger_button = Button::new(0, 0, 80, 0, "路径 🗀"); // 新增路径按钮
    top_triggers_flex.add(&path_trigger_button);
    top_triggers_flex.fixed(&path_trigger_button, 80);
    
    // Spacer to push the search group to the right
    let top_spacer = Frame::new(0,0,0,0,""); 
    top_triggers_flex.add(&top_spacer); // This spacer is flexible, pushing subsequent items to the right

    // Search group starts here, aligned to the right of top_spacer
    top_triggers_flex.add(&btn_done); 
    top_triggers_flex.fixed(&btn_done, 80); // Fixed width for Done button

    let search_gap_spacer = Frame::new(0,0,10,0,""); // 10px gap
    top_triggers_flex.add(&search_gap_spacer);
    top_triggers_flex.fixed(&search_gap_spacer, 10);

    // 将搜索框和搜索按钮添加到 top_triggers_flex
    top_triggers_flex.add(&search_input); // search_input is flexible and will take available space
    top_triggers_flex.add(&search_button);
    top_triggers_flex.fixed(&search_button, 40); // 搜索按钮固定宽度
    
    top_triggers_flex.end();
    
    main_vertical_flex.add(&top_triggers_flex);
    main_vertical_flex.fixed(&top_triggers_flex, MENU_TRIGGER_HEIGHT);

    // 可展开的菜单项面板 ("设置", "关于")
    let mut menu_items_panel_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, ""); // 初始高度为0
    menu_items_panel_flex.set_type(fltk::group::FlexType::Row); // 修改为 Row
    menu_items_panel_flex.set_margin(2); 

    let mut settings_button = Button::new(0, 0, 0, 30, "📝 注册"); 
    let mut unregister_button = Button::new(0, 0, 0, 30, "🗑️ 注销"); 
    let mut about_button = Button::new(0, 0, 0, 30, "📦 关于"); 

    menu_items_panel_flex.add(&settings_button); // 注册按钮
    menu_items_panel_flex.fixed(&settings_button, 80); 
    menu_items_panel_flex.add(&unregister_button); // 注销按钮
    menu_items_panel_flex.fixed(&unregister_button, 80); 
    menu_items_panel_flex.add(&about_button); // 关于按钮
    menu_items_panel_flex.fixed(&about_button, 100); 

    menu_items_panel_flex.end();
    menu_items_panel_flex.hide(); 
    main_vertical_flex.add(&menu_items_panel_flex);

    // --- 新增：定义路径显示相关的控件 ---
    // 这些控件之前在 bottom_flex 中定义，现在移到这里，以便添加到新的可折叠面板中
    // 移除 base_path_display, path_arrow, anime_path_display
    // let mut base_path_display = Output::new(0, 0, 0, 0, "");
    // base_path_display.set_tooltip("源路径(B按钮)");
    // let path_arrow = Frame::new(0, 0, 30, 0, "=>");
    // let mut anime_path_display = Output::new(0, 0, 0, 0, "");
    // anime_path_display.set_tooltip("目标路径(A按钮)");

    // --- 新增：可展开的路径显示面板 (现在包含路径选择按钮) ---
    let mut path_display_panel_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, ""); // 初始高度为0
    path_display_panel_flex.set_type(fltk::group::FlexType::Row);
    path_display_panel_flex.set_margin(2); // 添加一些边距
    path_display_panel_flex.add(&btn_choose_base); // 添加B按钮
    path_display_panel_flex.add(&btn_choose_anime); // 添加A按钮
    // 按钮将自动拉伸以填充空间
    path_display_panel_flex.end();
    path_display_panel_flex.hide(); // 初始隐藏
    main_vertical_flex.add(&path_display_panel_flex);
    
    // --- 菜单栏相关定义结束 ---
    
    // 创建内容区域的水平布局
    // 高度将由 main_vertical_flex 在菜单栏、路径栏和底部栏之间自动分配
    let mut content_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, ""); 
    content_flex.set_type(fltk::group::FlexType::Row);
    
    // 左半部分
    // 高度将由 content_flex 自动分配
    let mut left_flex = Flex::new(0, 0, HALF_WIDTH, 0, "");    
    left_flex.set_type(fltk::group::FlexType::Column); 
    left_flex.set_margin(5); 
    
    // 移除旧的 button_row_flex
    // let mut button_row_flex = Flex::new(0, 0, 0, 30, ""); 
    // button_row_flex.set_type(fltk::group::FlexType::Row);
    // let mut btn_choose_base = Button::new(0, 0, 0, 0, "B"); 
    // let mut btn_choose_anime = Button::new(0, 0, 0, 0, "A"); 
    // let mut btn_done = Button::new(0, 0, 0, 0, "完成"); 
    // button_row_flex.end();
    // left_flex.fixed(&button_row_flex, 30); 

    // 文件浏览器将填充 left_flex 的剩余空间
    let mut file_browser = FileBrowser::new(0, 0, 0, 0, ""); 
    file_browser.set_selection_color(Color::Yellow);
    file_browser.set_type(fltk::browser::BrowserType::Hold); // 单选模式
    file_browser.set_damage(true); // Ensure redraws    // For drag-and-drop reordering
    let dragged_line_index: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
    // 为空格键标记添加一个新的变量
    let marked_line_index: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
    
    let d_idx_for_handle = dragged_line_index.clone();
    let marked_idx_for_handle = marked_line_index.clone();
    
    file_browser.handle(move |b, ev| {
        let mut d_idx = d_idx_for_handle.borrow_mut();
        let mut marked_idx = marked_idx_for_handle.borrow_mut();
        
        match ev {
            Event::Push => {
                // 获取FLTK FileBrowser自己检测到的行号（单击即可开始拖拽）
                let line_num = b.value();
                if line_num > 0 { // 修改：不再忽略头部行，因为不再有文件夹名的头部行
                    *d_idx = Some(line_num);
                    
                    // 打印选中行的信息用于调试
                    if let Some(text) = b.text(line_num) {
                        println!("开始拖拽第 {} 行: \"{}\"", line_num, text);
                    }
                    
                    return true;
                }
                *d_idx = None;
                false
            },            // 添加对键盘事件的处理
            Event::KeyDown => {                // 检查是否按下空格键
                if app::event_key() == fltk::enums::Key::from_char(' ') {
                    let current_line = b.value();
                    if current_line > 0 { // 修改：不再忽略头部行
                        if let Some(first_marked_line) = *marked_idx {
                            // 已有标记的行，执行交换操作
                            println!("交换第 {} 行和第 {} 行", first_marked_line, current_line);
                            
                            if first_marked_line != current_line {
                                // 获取两行的文本内容
                                if let (Some(first_text), Some(second_text)) = (b.text(first_marked_line), b.text(current_line)) {
                                    println!("交换内容: \"{}\" <-> \"{}\"", first_text, second_text);
                                    
                                    // 收集浏览器中所有行的文本
                                    let mut all_lines = Vec::new();
                                    for i in 1..=b.size() {
                                        if let Some(text) = b.text(i) {
                                            all_lines.push(text.to_string());
                                        }
                                    }
                                    
                                    // 交换指定行的内容
                                    let first_idx = first_marked_line as usize - 1; // 转为0-based索引
                                    let second_idx = current_line as usize - 1; // 转为0-based索引
                                    if first_idx < all_lines.len() && second_idx < all_lines.len() {
                                        all_lines.swap(first_idx, second_idx);
                                    }
                                    
                                    // 清空浏览器
                                    b.clear();
                                    
                                    // 重新添加所有行
                                    for line in all_lines {
                                        b.add(&line);
                                    }
                                }
                                
                                // 高亮当前行
                                b.select(current_line);
                            }
                            

                            // 清除标记
                            *marked_idx = None;
                        } else {
                            // 标记当前行
                            *marked_idx = Some(current_line);
                            println!("标记第 {} 行", current_line);
                            

                            // 高亮当前行以提供视觉反馈
                            b.select(current_line);
                        }
                        return true;
                    }
                }
                false
            },
            Event::Drag => {
                if d_idx.is_some() {
                    // 拖拽期间，让行选中效果跟随鼠标位置
                    // 让FLTK自行处理鼠标位置与行号的对应关系
                    let mouse_y = app::event_y();
                    let widget_y = b.y();
                    let widget_h = b.h();
                    
                    if mouse_y >= widget_y && mouse_y < widget_y + widget_h {
                        // 传递事件让FileBrowser内部自行检测行
                        b.handle_event(Event::Move);
                        
                        // 获取当前鼠标悬停在哪一行
                        let drag_to_line = b.value();
                        
                        // 打印当前拖拽位置的行号
                        if drag_to_line > 0 {
                            println!("拖拽到第 {} 行", drag_to_line);
                        }
                    }
                    
                    return true;
                }
                false
            },
            Event::Released => {
                if let Some(from_line) = d_idx.take() {
                    // 获取释放位置的行号
                    let to_line = b.value();
                    
                    println!("从第 {} 行移动到第 {} 行", from_line, to_line);
                    
                    // 执行移动操作
                    if to_line > 0 && to_line != from_line { // 修改：不再检查是否大于1
                        // 在移动前打印源和目标行的内容
                        if let Some(from_text) = b.text(from_line) {
                            println!("源行 {} 内容: \"{}\"", from_line, from_text);
                        }
                        
                        if let Some(to_text) = b.text(to_line) {
                            println!("目标行 {} 内容: \"{}\"", to_line, to_text);
                        }
                        
                        // 根据您的测试，参数顺序是 (to, from)
                        b.move_item(to_line, from_line);
                        
                        // 高亮目标行
                        b.select(to_line);
                        
                        // 打印移动后的内容
                        if let Some(new_text) = b.text(to_line) {
                            println!("移动后位置 {} 内容: \"{}\"", to_line, new_text);
                        }
                    }
                    
                    return true;
                }
                false
            },
            _ => false,
        }
    });

    left_flex.end();
    
    // 右半部分
    // 高度将由 content_flex 自动分配
    let mut right_flex = Flex::new(HALF_WIDTH, 0, HALF_WIDTH, 0, "");
    right_flex.set_type(fltk::group::FlexType::Column);
    right_flex.set_margin(5);

    // --- 移除旧的 search_row_flex ---
//     let mut search_row_flex = Flex::new(0, 0, 0, 30, ""); 
//     search_row_flex.set_type(fltk::group::FlexType::Row);
//     let mut search_input = Input::new(0, 0, 0, 0, ""); // 定义已上移
//     search_input.set_tooltip("输入番剧名称关键字"); // 已上移
//     let mut search_button = Button::new(0, 0, 40, 0, "🔎"); // 定义已上移
//     search_row_flex.fixed(&search_button, 40); 
//     search_row_flex.end();
//     right_flex.fixed(&search_row_flex, 30); 

    // 搜索结果浏览器将填充 right_flex 的剩余空间
    let mut search_results_browser = MultiBrowser::new(0, 0, 0, 0, ""); 
    search_results_browser.set_selection_color(Color::Yellow);
    search_results_browser.set_type(fltk::browser::BrowserType::Hold); // 单选模式

    right_flex.end();
    
    content_flex.add(&left_flex);
    content_flex.add(&right_flex);
    content_flex.end();
      
    // --- 新增：如果通过命令行参数设置了路径，则更新UI ---
    if let Some(cli_base_path_str) = base_path.borrow().as_deref() {
        btn_choose_base.set_label(cli_base_path_str); // 更新按钮B的标签
        load_files_to_file_browser(cli_base_path_str, &mut file_browser); // 加载文件

        // --- 新增：如果 -b 参数存在，尝试提取番剧名并填充搜索框 ---
        if let Some(extracted_anime_name) = extract_anime_name_from_path(cli_base_path_str) {
            search_input.set_value(&extracted_anime_name);
            println!("从路径 {} 提取到番剧名: {}", cli_base_path_str, extracted_anime_name);
        }
        // --- 提取番剧名结束 ---
    }

    if let Some(cli_anime_path_str) = anime_path.borrow().as_deref() {
        btn_choose_anime.set_label(cli_anime_path_str); // 更新按钮A的标签
    }
    // --- UI 更新结束 ---

    // 移除原有的 bottom_flex 定义，其内容已移至 path_display_panel_flex
    // let mut bottom_flex = Flex::new(0, 0, WINDOW_WIDTH, 30, "");
    // ... (base_path_display, path_arrow, anime_path_display were here)
    // bottom_flex.end();
    
    // 将内容区域加入主布局
    main_vertical_flex.add(&content_flex); // content_flex 会占据菜单栏和底部栏之间的剩余空间
    // 移除 main_vertical_flex.add(&bottom_flex);
    // 移除 main_vertical_flex.fixed(&bottom_flex, 30);
    main_vertical_flex.end();
    
    wind.add(&main_vertical_flex); 
    wind.resizable(&main_vertical_flex); 
    wind.end();
    wind.show();

    // --- 修改：菜单栏按钮回调 ---
    let mut menu_items_panel_flex_clone_cb = menu_items_panel_flex.clone();
    let mut main_vertical_flex_cb_clone_menu = main_vertical_flex.clone(); 
    let is_menu_expanded_clone = is_menu_expanded.clone(); // 为回调克隆状态
    let mut wind_clone_for_menu = wind.clone(); // 克隆 wind 用于菜单回调

    menu_trigger_button.set_callback(move |_| {
        let mut expanded = is_menu_expanded_clone.borrow_mut(); 
        *expanded = !*expanded;

        if *expanded {
            main_vertical_flex_cb_clone_menu.fixed(&menu_items_panel_flex_clone_cb, MENU_ITEMS_PANEL_EXPANDED_HEIGHT);
            menu_items_panel_flex_clone_cb.show();
        } else {
            menu_items_panel_flex_clone_cb.hide(); 
            main_vertical_flex_cb_clone_menu.fixed(&menu_items_panel_flex_clone_cb, 0);
        }
        main_vertical_flex_cb_clone_menu.layout(); 
        wind_clone_for_menu.redraw(); // 使用克隆的 wind
    });

    settings_button.set_callback(|_| {
        match env::current_exe() {
            Ok(exe_path_buf) => {
                let exe_path = exe_path_buf.to_string_lossy().to_string();
                let mut errors = Vec::new();

                // 目标是 HKEY_CLASSES_ROOT，它通常需要管理员权限才能写入系统范围的关联
                // winreg crate 会尝试写入，如果权限不足，操作会失败。
                let hkey_classes_root = RegKey::predef(HKEY_CLASSES_ROOT);

                // 注册文件夹右键菜单
                let dir_shell_path = "Directory\\shell";
                let dir_key_name = "BgmRenameCuby";
                let dir_command_val = format!("\"{}\" -b \"%1\" -a \"%1\\anime\"", exe_path);

                match hkey_classes_root.create_subkey(format!("{}\\{}", dir_shell_path, dir_key_name)) {
                    Ok((key, _disp)) => {
                        if let Err(e) = key.set_value("", &"使用 BgmRenameCuby 处理文件夹") { errors.push(format!("设置文件夹菜单默认值失败: {}", e)); }
                        if let Err(e) = key.set_value("Icon", &exe_path) { errors.push(format!("设置文件夹菜单Icon失败: {}", e)); }
                        match key.create_subkey("command") {
                            Ok((cmd_key, _)) => {
                                if let Err(e) = cmd_key.set_value("", &dir_command_val) { errors.push(format!("设置文件夹命令失败: {}", e)); }
                            }
                            Err(e) => errors.push(format!("创建文件夹命令子键失败: {}", e)),
                        }
                    }
                    Err(e) => errors.push(format!("创建文件夹菜单主键 (HKCR\\{}\\{}) 失败: {}", dir_shell_path, dir_key_name, e)),
                }

                // 注册文件夹背景右键菜单
                let dir_bg_shell_path = "Directory\\Background\\shell";
                // dir_key_name is the same
                let dir_bg_command_val = format!("\"{}\" -b \"%V\" -a \"%V\\anime\"", exe_path);

                match hkey_classes_root.create_subkey(format!("{}\\{}", dir_bg_shell_path, dir_key_name)) {
                    Ok((key, _disp)) => {
                        if let Err(e) = key.set_value("", &"BgmRenameCuby 在此处理") { errors.push(format!("设置背景菜单默认值失败: {}", e)); }
                        if let Err(e) = key.set_value("Icon", &exe_path) { errors.push(format!("设置背景菜单Icon失败: {}", e)); }
                        match key.create_subkey("command") {
                            Ok((cmd_key, _)) => {
                                if let Err(e) = cmd_key.set_value("", &dir_bg_command_val) { errors.push(format!("设置背景命令失败: {}", e)); }
                            }
                            Err(e) => errors.push(format!("创建背景命令子键失败: {}", e)),
                        }
                    }
                    Err(e) => errors.push(format!("创建背景菜单主键 (HKCR\\{}\\{}) 失败: {}", dir_bg_shell_path, dir_key_name, e)),
                }

                if errors.is_empty() {
                    fltk::dialog::message_default("注册表项已成功添加/更新。\n部分更改可能需要重启资源管理器或重新登录才能生效。");
                } else {
                    fltk::dialog::message_default(&format!("注册表操作时发生错误:\n{}\n\n请确保以管理员身份运行本程序。", errors.join("\n")));
                }
            }
            Err(e) => {
                fltk::dialog::message_default(&format!("获取程序路径失败: {}", e));
            }
        }
    });

    unregister_button.set_callback(|_| {
        let mut errors = Vec::new();
        let hkey_classes_root = RegKey::predef(HKEY_CLASSES_ROOT);
        let mut deleted_anything = false; // 新增：跟踪是否有实际删除操作

        let dir_key_path = "Directory\\shell\\BgmRenameCuby";
        let dir_bg_key_path = "Directory\\Background\\shell\\BgmRenameCuby";

        match hkey_classes_root.delete_subkey_all(dir_key_path) {
            Ok(_) => {
                deleted_anything = true; // 标记已删除
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    errors.push(format!("删除文件夹菜单项失败 (HKCR\\{}): {}", dir_key_path, e));
                }
                // NotFound 不是错误，但表示没有删除任何东西
            }
        }

        match hkey_classes_root.delete_subkey_all(dir_bg_key_path) {
            Ok(_) => {
                deleted_anything = true; // 标记已删除
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    errors.push(format!("删除背景菜单项失败 (HKCR\\{}): {}", dir_bg_key_path, e));
                }
                // NotFound 不是错误，但表示没有删除任何东西
            }
        }

        if errors.is_empty() {
            if deleted_anything {
                fltk::dialog::message_default("相关注册表项已成功删除。\n部分更改可能需要重启资源管理器或重新登录才能生效。");
            } else {
                fltk::dialog::message_default("未找到相关的注册表项，无需注销。"); // 修改：如果未删除任何内容，则显示此消息
            }
        } else {
            // 修改：使用 \n 进行换行，并确保 errors.join 使用 \n
            fltk::dialog::message_default(&format!("注销操作时发生错误:\n{}\n\n请确保以管理员身份运行本程序。", errors.join("\n")));
        }
    });

    about_button.set_callback(|_| {
        // fltk::dialog::message_default("关于"); // 旧的回调
        let github_url = "https://github.com/uuzp/bgm_rename_cuby"; // 请替换为您的仓库 URL
        if webbrowser::open(github_url).is_err() {
            fltk::dialog::message_default(&format!("无法打开浏览器访问: {}", github_url));
        }
    });
    
    // --- 新增：路径触发按钮回调 ---
    let mut path_display_panel_flex_clone_cb = path_display_panel_flex.clone();
    let mut main_vertical_flex_cb_clone_path = main_vertical_flex.clone();
    let is_path_panel_expanded_clone = is_path_panel_expanded.clone(); // 为回调克隆状态
    let mut wind_clone_for_path = wind.clone(); // 克隆 wind 用于路径回调

    path_trigger_button.set_callback(move |_| {
        let mut expanded = is_path_panel_expanded_clone.borrow_mut(); 
        *expanded = !*expanded;

        if *expanded {
            main_vertical_flex_cb_clone_path.fixed(&path_display_panel_flex_clone_cb, PATH_DISPLAY_PANEL_EXPANDED_HEIGHT);
            path_display_panel_flex_clone_cb.show();
        } else {
            path_display_panel_flex_clone_cb.hide();
            main_vertical_flex_cb_clone_path.fixed(&path_display_panel_flex_clone_cb, 0);
        }
        main_vertical_flex_cb_clone_path.layout();
        wind_clone_for_path.redraw(); // 使用克隆的 wind
    });
    // --- 菜单栏按钮回调结束 ---

    // 创建共享数据结构
    let search_results: Rc<RefCell<Option<Bgm>>> = Rc::new(RefCell::new(None));
    let episode_list: Rc<RefCell<Option<Ep>>> = Rc::new(RefCell::new(None));
    let selected_anime_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));    // 按钮B - 设置源路径并加载文件列表
    {
        let base_path_copy = base_path.clone();
        // 移除 base_path_display_copy，改为更新按钮标签
        // let mut base_path_display_copy = base_path_display.clone();
        let mut btn_choose_base_clone = btn_choose_base.clone(); // 克隆按钮B
        let mut file_browser_clone = file_browser.clone();
        let mut search_input_clone_for_b = search_input.clone(); // 克隆 search_input
        
        btn_choose_base.set_callback(move |_| {
            let mut dialog = FileDialog::new(fltk::dialog::FileDialogType::BrowseDir);
            dialog.show();
            let chosen_path_pb = dialog.filename(); // Renamed to avoid conflict
            if !chosen_path_pb.as_os_str().is_empty() {
                let path = Path::new(&chosen_path_pb);
                if path.is_dir() {
                    if let Some(path_str) = path.to_str() {
                        *base_path_copy.borrow_mut() = Some(path_str.to_string());
                        btn_choose_base_clone.set_label(path_str); 
                        
                        load_files_to_file_browser(path_str, &mut file_browser_clone);

                        // --- 新增：通过按钮选择路径后，也尝试提取番剧名 ---
                        if let Some(extracted_anime_name) = extract_anime_name_from_path(path_str) {
                            search_input_clone_for_b.set_value(&extracted_anime_name);
                            println!("从路径 {} 提取到番剧名: {}", path_str, extracted_anime_name);
                        }
                        // --- 提取番剧名结束 ---
                    }
                }
            }
        });
    }
      // 按钮A - 设置目标路径
    {
        let anime_path_copy = anime_path.clone();
        // 移除 anime_path_display_copy，改为更新按钮标签
        // let mut anime_path_display_copy = anime_path_display.clone();
        let mut btn_choose_anime_clone = btn_choose_anime.clone(); // 克隆按钮A
        
        btn_choose_anime.set_callback(move |_| {
            let mut dialog = FileDialog::new(fltk::dialog::FileDialogType::BrowseDir);
            dialog.show();
            let chosen_path = dialog.filename();
            if !chosen_path.as_os_str().is_empty() {
                let path = Path::new(&chosen_path);
                if path.is_dir() {
                    // 保存目标路径（A按钮）
                    if let Some(path_str) = path.to_str() {
                        *anime_path_copy.borrow_mut() = Some(path_str.to_string());
                        // anime_path_display_copy.set_value(path_str);
                        btn_choose_anime_clone.set_label(path_str); // 更新按钮A的标签
                    }
                }
            }
        });
    }
    
    // 为搜索按钮设置回调
    {
        let search_input_copy = search_input.clone();
        let search_results_copy = search_results.clone();
        let mut search_results_browser_copy = search_results_browser.clone();
        
        search_button.set_callback(move |_| {
            // 执行搜索逻辑
            let query = search_input_copy.value();
            if !query.is_empty() {
                // 执行搜索
                let bgm = Bgm::new().get(&query);
                
                // 清空并更新搜索结果列表
                search_results_browser_copy.clear();
                for name in &bgm.name {
                    search_results_browser_copy.add(name);
                }
                
                // 保存搜索结果
                *search_results_copy.borrow_mut() = Some(bgm);
            }
        });
    }
    
    // 为搜索框设置回车键处理
    {
        let search_input_copy = search_input.clone();
        let search_results_copy = search_results.clone();
        let mut search_results_browser_copy = search_results_browser.clone();
        
        search_input.handle(move |_, ev| {
            if ev == Event::KeyDown && app::event_key() == fltk::enums::Key::Enter {
                // 当按下回车键时执行搜索
                let query = search_input_copy.value();
                if !query.is_empty() {
                    // 执行搜索
                    let bgm = Bgm::new().get(&query);
                    
                    // 清空并更新搜索结果列表
                    search_results_browser_copy.clear();
                    for name in &bgm.name {
                        search_results_browser_copy.add(name);
                    }
                    
                    // 保存搜索结果
                    *search_results_copy.borrow_mut() = Some(bgm);
                }
                return true; // 表示事件已处理
            }
            false // 让其他按键由默认处理程序处理
        });
    }
    
    // 设置搜索结果双击事件处理
    {
        let search_results_copy = search_results.clone();
        let episode_list_copy = episode_list.clone();
        let selected_anime_id_copy = selected_anime_id.clone();
        
        search_results_browser.set_callback(move |b| {
            if app::event_clicks() {
                let line = b.value();
                if line > 0 && line <= b.size() {
                    if let Some(bgm) = &*search_results_copy.borrow() {
                        let idx = (line as usize) - 1;
                        if idx < bgm.id.len() {
                            let anime_id = &bgm.id[idx];
                            
                            // 获取选中番剧的剧集信息
                            let ep = Ep::get(anime_id);
                            
                            // 清空并显示剧集列表
                            b.clear();
                            for episode_name in &ep.name {
                                b.add(episode_name);
                            }
                            
                            // 保存选中的番剧ID和剧集信息
                            *selected_anime_id_copy.borrow_mut() = Some(anime_id.clone());
                            *episode_list_copy.borrow_mut() = Some(ep);
                        }
                    }
                }
            }
        });
    }    // 设置"完成"按钮回调
    {        
        let mut file_browser_copy = file_browser.clone();
        let episode_list_copy = episode_list.clone();
        let base_path_copy = base_path.clone(); 
        let anime_path_copy = anime_path.clone(); 
        let search_results_copy_for_done = search_results.clone(); 
        let selected_anime_id_copy_for_done = selected_anime_id.clone(); 
        let mut search_results_browser_copy_for_done = search_results_browser.clone();
        
        btn_done.set_callback(move |_| {
            println!("注意: 该程序会自动查找并处理与视频文件对应的字幕文件，保留原有语言标识");
            println!("      支持的字幕格式: .ass, .srt, .ssa, .sub");
            
            // 首先检查源路径和目标路径是否都已设置
            if base_path_copy.borrow().is_none() {
                println!("错误: 未设置源文件路径（B按钮）");
                fltk::dialog::message_default("错误: 未设置源文件路径（B按钮）");
                return;
            }
            
            if anime_path_copy.borrow().is_none() {
                println!("错误: 未设置目标位置路径（A按钮）");
                fltk::dialog::message_default("错误: 未设置目标位置路径（A按钮）");
                return;
            }
            
            let base_path_str = base_path_copy.borrow().as_ref().unwrap().clone(); 
            let anime_path_str = anime_path_copy.borrow().as_ref().unwrap().clone(); 
            
            // 获取文件浏览器中的文件列表（从第1行开始，不再有头部行）
            let mut file_names = Vec::new();
            for i in 1..=file_browser_copy.size() {
                if let Some(text) = file_browser_copy.text(i) {
                    file_names.push(text.to_string());
                }
            }
            
            if let Some(ep) = &*episode_list_copy.borrow() {
                // 创建文件名与剧集对应的列表
                let mut matched_pairs = Vec::new();
                
                // 打印匹配结果
                println!("文件与剧集匹配结果:");
                
                let mut matched_count = 0;
                for (i, file_name) in file_names.iter().enumerate() {
                    if i < ep.name.len() {
                        // 获取源文件的扩展名
                        let src_path = Path::new(&base_path_str).join(file_name);
                        let extension = src_path.extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("");
                        
                        // 构建带扩展名的目标文件名用于显示
                        let dst_name_with_ext = if extension.is_empty() {
                            ep.name[i].clone()
                        } else {
                            format!("{}.{}", ep.name[i], extension)
                        };
                        
                        println!("{} -> {}", file_name, dst_name_with_ext);
                        matched_pairs.push((file_name.clone(), ep.name[i].clone()));
                        matched_count += 1;
                    }
                }
                
                println!("成功匹配: {}/{} 个文件", matched_count, file_names.len());
                
                if matched_count < file_names.len() {
                    println!("警告: 有 {} 个文件没有对应的剧集信息", 
                        file_names.len() - matched_count);
                }
                  // 执行硬链接操作
                if !matched_pairs.is_empty() {
                    // 确保目标目录存在
                    let dest_path = Path::new(&anime_path_str); // A按钮设置的目标路径
                    if !dest_path.exists() {
                        if let Err(e) = std::fs::create_dir_all(dest_path) {
                            println!("创建目标目录失败: {}", e);
                            fltk::dialog::message_default(&format!("创建目标目录失败: {}", e));
                            return;
                        }
                    }
                    
                    // 查找可能存在的字幕文件
                    let mut subtitle_files = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(&base_path_str) {
                        for entry in entries.filter_map(Result::ok) {
                            let path = entry.path();
                            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                                // 检查是否是字幕文件（通常以.ass、.srt等结尾）
                                let is_subtitle = path.extension()
                                    .and_then(|ext| ext.to_str())
                                    .map(|ext| ext.to_lowercase())
                                    .map(|ext| ext == "ass" || ext == "srt" || ext == "ssa" || ext == "sub")
                                    .unwrap_or(false);
                                
                                if is_subtitle {
                                    // 添加到字幕列表
                                    subtitle_files.push(file_name.to_string());
                                }
                            }
                        }
                    }
                    
                    println!("找到 {} 个字幕文件", subtitle_files.len());
                    if !subtitle_files.is_empty() {
                        println!("字幕文件列表:");
                        for sub_file in &subtitle_files {
                            println!("  - {}", sub_file);
                        }
                    }
                    
                    // 创建硬链接
                    let mut success_count = 0;
                    let mut subtitle_count = 0;
                    
                    for (src_file, dst_name) in &matched_pairs {
                        let src_path = Path::new(&base_path_str).join(src_file); 
                        
                        // 从源文件路径获取文件扩展名
                        let extension_osstr = src_path.extension();
                        let extension_str = extension_osstr.and_then(|s| s.to_str());
                        
                        // 构建包含扩展名的目标文件名
                        let dst_name_with_ext = if extension_str.is_some() && !extension_str.unwrap().is_empty() {
                            format!("{}.{}", dst_name, extension_str.unwrap())
                        } else {
                            dst_name.clone()
                        };

                        let dest_file_path = dest_path.join(&dst_name_with_ext);

                        // 创建硬链接
                        match std::fs::hard_link(&src_path, &dest_file_path) {
                            Ok(_) => {
                                println!("成功创建硬链接: {} => {}", src_path.display(), dest_file_path.display());
                                success_count += 1;
                            }
                            Err(e) => {
                                println!("创建硬链接失败 ({}): {} => {}", e, src_path.display(), dest_file_path.display());
                            }
                        }

                        // 为视频文件查找并处理对应的字幕文件
                        let video_file_stem = src_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        for sub_file_name in &subtitle_files {
                            if let Some(sub_file_stem) = Path::new(sub_file_name).file_stem().and_then(|s| s.to_str()) {
                                // 检查字幕文件名是否以视频文件名（不含扩展名）开头
                                if sub_file_stem.starts_with(video_file_stem) {
                                    let lang_suffix = sub_file_stem.trim_start_matches(video_file_stem);
                                    let sub_src_path = Path::new(&base_path_str).join(sub_file_name);
                                    let sub_extension = sub_src_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                    
                                    // 构建新的字幕文件名，格式为：剧集名 + 语言标识 + .字幕扩展名
                                    let new_sub_file_name = if sub_extension.is_empty() {
                                        format!("{}{}", dst_name, lang_suffix) 
                                    } else {
                                        format!("{}{}.{}", dst_name, lang_suffix, sub_extension)
                                    };
                                    let sub_dest_file_path = dest_path.join(&new_sub_file_name);

                                    match std::fs::hard_link(&sub_src_path, &sub_dest_file_path) {
                                        Ok(_) => {
                                            println!("成功创建字幕硬链接: {} => {}", sub_src_path.display(), sub_dest_file_path.display());
                                            subtitle_count += 1;
                                        }
                                        Err(e) => {
                                            println!("创建字幕硬链接失败 ({}): {} => {}", e, sub_src_path.display(), sub_dest_file_path.display());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    let message = format!("操作完成！\n成功创建 {} 个视频硬链接。\n成功创建 {} 个字幕硬链接。", success_count, subtitle_count);
                    println!("{}", message);
                    fltk::dialog::message_default(&message);

                    file_browser_copy.clear();
                    search_results_browser_copy_for_done.clear(); 
                    
                    *episode_list_copy.borrow_mut() = None;
                    *search_results_copy_for_done.borrow_mut() = None; 
                    *selected_anime_id_copy_for_done.borrow_mut() = None;

                } else {
                    println!("没有匹配到任何文件和剧集，或者剧集列表为空。");
                    fltk::dialog::message_default("没有匹配到任何文件和剧集，或者剧集列表为空。");
                }
            } else {
                println!("剧集列表为空，无法执行操作。");
                fltk::dialog::message_default("剧集列表为空，无法执行操作。");
            }
        });
    }
    app.run().unwrap(); // 新增：启动 FLTK 事件循环
}
