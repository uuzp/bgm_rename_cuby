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
    output::Output,
    prelude::*,
    window::Window,
};
use std::{
    cell::RefCell,
    path::Path, // Removed PathBuf as it's not used
    rc::Rc,
    // fs, // Removed fs as it's not directly used in the provided snippet, assuming it's used elsewhere or implicitly
    // io, // Removed io for the same reason as fs
};

use miniserde::{Deserialize, Serialize};
use miniserde::json;
use minreq;
use urlencoding;

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

fn main() {
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    let mut wind = Window::new(
        100,
        100,
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        "File Explorer and Search",
    );
    
    // 添加两个路径变量
    let base_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let anime_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    // --- 提前定义按钮，以便在不同 Flex 容器中使用 ---
    let mut btn_choose_base = Button::new(0, 0, 0, 0, "选择源路径 (B)");
    btn_choose_base.set_tooltip("选择源文件夹 (B)");
    let mut btn_choose_anime = Button::new(0, 0, 0, 0, "选择目标路径 (A)");
    btn_choose_anime.set_tooltip("选择目标文件夹 (A)");
    let mut btn_done = Button::new(0, 0, 0, 0, "完成");

    // --- 新增：提前定义搜索控件 ---
    let mut search_input = Input::new(0, 0, 0, 0, "");
    search_input.set_tooltip("输入番剧名称关键字");
    let mut search_button = Button::new(0, 0, 40, 0, "🔎");
    
    let mut main_vertical_flex = Flex::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, "");
    main_vertical_flex.set_type(fltk::group::FlexType::Column); // 垂直排列

    // --- 修改：菜单栏 & 路径栏 UI Elements ---
    const MENU_TRIGGER_HEIGHT: i32 = 30;
    const MENU_ITEMS_PANEL_EXPANDED_HEIGHT: i32 = 60; // 2 buttons * 30 height each
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
    
    top_triggers_flex.add(&btn_done); // 将“完成”按钮添加到顶部触发器行
    top_triggers_flex.fixed(&btn_done, 80); // 给“完成”按钮一个宽度

    let top_spacer = Frame::new(0,0,0,0,""); // 添加一个间隔，用于将搜索控件推到右侧
    top_triggers_flex.add(&top_spacer);

    // 将搜索框和搜索按钮添加到 top_triggers_flex
    top_triggers_flex.add(&search_input); // search_input 会自动填充剩余空间
    top_triggers_flex.add(&search_button);
    top_triggers_flex.fixed(&search_button, 40); // 搜索按钮固定宽度
    
    top_triggers_flex.end();
    
    main_vertical_flex.add(&top_triggers_flex);
    main_vertical_flex.fixed(&top_triggers_flex, MENU_TRIGGER_HEIGHT);

    // 可展开的菜单项面板 ("设置", "关于")
    let mut menu_items_panel_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, ""); // 初始高度为0
    menu_items_panel_flex.set_type(fltk::group::FlexType::Column);
    menu_items_panel_flex.set_margin(2); 

    let mut settings_button = Button::new(0, 0, 0, 30, "设置"); 
    let mut about_button = Button::new(0, 0, 0, 30, "关于");    

    menu_items_panel_flex.add(&settings_button);
    menu_items_panel_flex.add(&about_button);
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
    // let mut search_row_flex = Flex::new(0, 0, 0, 30, ""); 
    // search_row_flex.set_type(fltk::group::FlexType::Row);
    // let mut search_input = Input::new(0, 0, 0, 0, ""); // 定义已上移
    // search_input.set_tooltip("输入番剧名称关键字"); // 已上移
    // let mut search_button = Button::new(0, 0, 40, 0, "🔎"); // 定义已上移
    // search_row_flex.fixed(&search_button, 40); 
    // search_row_flex.end();
    // right_flex.fixed(&search_row_flex, 30); 

    // 搜索结果浏览器将填充 right_flex 的剩余空间
    let mut search_results_browser = MultiBrowser::new(0, 0, 0, 0, ""); 
    search_results_browser.set_selection_color(Color::Yellow);
    search_results_browser.set_type(fltk::browser::BrowserType::Hold); // 单选模式

    right_flex.end();
    
    content_flex.add(&left_flex);
    content_flex.add(&right_flex);
    content_flex.end();
      
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
        fltk::dialog::message_default("设置");
    });

    about_button.set_callback(|_| {
        fltk::dialog::message_default("关于");
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
        
        btn_choose_base.set_callback(move |_| {
            let mut dialog = FileDialog::new(fltk::dialog::FileDialogType::BrowseDir);
            dialog.show();
            let chosen_path = dialog.filename();
            if !chosen_path.as_os_str().is_empty() {
                let path = Path::new(&chosen_path);
                if path.is_dir() {
                    // 保存源路径（B按钮）
                    if let Some(path_str) = path.to_str() {
                        *base_path_copy.borrow_mut() = Some(path_str.to_string());
                        // base_path_display_copy.set_value(path_str);
                        btn_choose_base_clone.set_label(path_str); // 更新按钮B的标签
                        
                        // 直接从当前选择的路径加载文件到浏览器
                        file_browser_clone.clear(); // 清空浏览器
                        
                        if let Ok(entries) = std::fs::read_dir(&path) {
                            for entry in entries {
                                if let Ok(entry) = entry {
                                    let file_path = entry.path();
                                    if file_path.is_file() {
                                        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                                            match ext.to_lowercase().as_str() {
                                                "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => {
                                                    if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                                                        // 直接添加文件名
                                                        file_browser_clone.add(file_name);
                                                    }
                                                }
                                                _ => (),
                                            }
                                        }
                                    }
                                }
                            }
                        }
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
    {        let mut file_browser_copy = file_browser.clone();
        let episode_list_copy = episode_list.clone();
        let base_path_copy = base_path.clone(); // 目标路径
        let anime_path_copy = anime_path.clone(); // 源路径
        let search_results_copy = search_results.clone();
        let selected_anime_id_copy = selected_anime_id.clone();
        let mut search_results_browser_copy = search_results_browser.clone();
        
        btn_done.set_callback(move |_| {
            // 提示用户新增的字幕文件处理功能
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
            
            let base_path_str = base_path_copy.borrow().as_ref().unwrap().clone(); // 源路径（B按钮选择）
            let anime_path_str = anime_path_copy.borrow().as_ref().unwrap().clone(); // 目标路径（A按钮选择）
            
            // 获取文件浏览器中的文件列表（从第1行开始，不再有头部行）
            let mut file_names = Vec::new();
            for i in 1..=file_browser_copy.size() {
                if let Some(text) = file_browser_copy.text(i) {
                    // 直接使用文本，不再需要去除前缀
                    file_names.push(text.to_string());
                }
            }
            
            // 获取剧集列表
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
                        let src_path = Path::new(&base_path_str).join(src_file); // B按钮设置的源路径
                        
                        // 从源文件路径获取文件扩展名
                        let extension = src_path.extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("");
                        
                        // 构建包含扩展名的目标文件名
                        let dst_name_with_ext = if extension.is_empty() {
                            dst_name.clone()
                        } else {
                            format!("{}.{}", dst_name, extension)
                        };
                        
                        let dst_path = Path::new(&anime_path_str).join(&dst_name_with_ext); // A按钮设置的目标路径
                        
                        // 检查源文件是否存在
                        if !src_path.exists() {
                            println!("源文件不存在: {:?}", src_path);
                            continue;
                        }
                        
                        // 如果目标文件已存在，先删除
                        if dst_path.exists() {
                            if let Err(e) = std::fs::remove_file(&dst_path) {
                                println!("删除已存在的目标文件失败: {}", e);
                                continue;
                            }
                        }
                        
                        // 创建硬链接                        println!("创建硬链接: {:?} -> {:?}", src_path, dst_path);
                        if let Err(e) = std::fs::hard_link(&src_path, &dst_path) {
                            println!("创建硬链接失败: {}", e);
                        } else {
                            println!("硬链接创建成功");
                            success_count += 1;
                              // 处理对应的字幕文件
                            if let Some(src_stem) = src_path.file_stem().and_then(|s| s.to_str()) {
                                // 查找所有与该视频文件匹配的字幕文件
                                for subtitle_file in &subtitle_files {
                                    // 检查字幕文件是否属于当前视频文件
                                    // 格式可能是: 视频名.语言标识.ass 或 视频名.ass
                                    // Normalize by removing spaces and converting to lowercase for robust matching
                                    let normalized_src_stem = src_stem.replace(" ", "").to_lowercase();
                                    let normalized_subtitle_file = subtitle_file.replace(" ", "").to_lowercase();

                                    let mut is_match = false;
                                    if normalized_subtitle_file.starts_with(&normalized_src_stem) {
                                        // Ensure that what follows the normalized_src_stem in normalized_subtitle_file
                                        // starts with a dot, indicating an extension or language tag.
                                        if normalized_src_stem.len() < normalized_subtitle_file.len() {
                                            let remainder = &normalized_subtitle_file[normalized_src_stem.len()..];
                                            if remainder.starts_with('.') {
                                                // The original `is_subtitle` check (when populating `subtitle_files`)
                                                // already ensures it ends with a valid subtitle extension.
                                                is_match = true;
                                            }
                                        } else if normalized_src_stem.len() == normalized_subtitle_file.len() {
                                            // This case should not happen if subtitle_file always has an extension
                                            // and was filtered by is_subtitle. But as a safe guard,
                                            // if they are identical after normalization, it implies video stem was
                                            // somehow identical to a subtitle file name without its final extension.
                                            // This is unlikely to be a valid subtitle match unless src_stem itself
                                            // ended with something like ".ass_stem" and subtitle was ".ass_stem.real_ext".
                                            // Given `is_subtitle` filters for actual subtitle extensions, this path is less critical.
                                        }
                                    }
                                    
                                    println!("检查字幕文件 '{}' 与视频 '{}' (stem: '{}') 是否匹配: {}", 
                                              subtitle_file, src_file, src_stem, if is_match { "是" } else { "否" });
                                    
                                    if is_match {
                                        let subtitle_path = Path::new(&base_path_str).join(subtitle_file);
                                          let subtitle_ext = subtitle_path.extension()
                                            .and_then(|ext| ext.to_str())
                                            .unwrap_or("ass"); // Default to "ass" if no extension found

                                        let original_subtitle_stem = subtitle_path.file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("");

                                        // src_stem is from the video file (e.g., "Video.Name.Tag")
                                        // dst_name is the new episode base name (e.g., "EP01 - Title")

                                        let mut lang_and_middle_parts = "";
                                        // Compare original_subtitle_stem with src_stem (video stem)
                                        // to find parts like ".eng", ".chi.sim"
                                        if original_subtitle_stem.starts_with(src_stem) {
                                            let remainder = original_subtitle_stem.strip_prefix(src_stem).unwrap_or("");
                                            if remainder.starts_with('.') && !remainder.is_empty() {
                                                lang_and_middle_parts = remainder; // e.g., ".eng", ".chi.sim"
                                            }
                                        }
                                        
                                        // Construct the new subtitle name: dst_name + lang_parts + subtitle_extension
                                        let new_subtitle_name = format!("{}{}.{}",
                                            dst_name,              // New episode base name like "EP01 - Title"
                                            lang_and_middle_parts, // Language identifier like ".eng", or empty string
                                            subtitle_ext);         // Subtitle extension like "srt"
                                        
                                        println!("  源字幕stem: {}, 视频stem: {}, 语言部分: '{}', 新字幕名: {}", original_subtitle_stem, src_stem, lang_and_middle_parts, new_subtitle_name);
                                        
                                        let new_subtitle_path = Path::new(&anime_path_str).join(&new_subtitle_name);
                                        
                                        // 如果目标字幕文件已存在，先删除
                                        if new_subtitle_path.exists() {
                                            if let Err(e) = std::fs::remove_file(&new_subtitle_path) {
                                                println!("删除已存在的目标字幕文件失败: {}", e);
                                                continue;
                                            }
                                        }
                                        
                                        // 创建字幕文件的硬链接
                                        println!("创建字幕硬链接: {:?} -> {:?}", subtitle_path, new_subtitle_path);
                                        if let Err(e) = std::fs::hard_link(&subtitle_path, &new_subtitle_path) {
                                            println!("创建字幕硬链接失败: {}", e);
                                        } else {
                                            println!("字幕硬链接创建成功");
                                            subtitle_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                      let message = format!("成功创建 {}/{} 个视频硬链接，{} 个字幕硬链接", 
                        success_count, matched_pairs.len(), subtitle_count);
                    println!("{}", message);
                    fltk::dialog::message_default(&message);
                    
                    // 操作完成后清空文件浏览器和搜索结果列表
                    file_browser_copy.clear();
                    search_results_browser_copy.clear();
                    // 清空关联的内存数据
                    *episode_list_copy.borrow_mut() = None;
                    *search_results_copy.borrow_mut() = None;
                    *selected_anime_id_copy.borrow_mut() = None;
                    
                    println!("已清空文件浏览器和搜索结果列表，可以开始下一次操作");
                }
            } else {
                println!("错误: 未选择任何番剧或未获取到剧集信息");
                fltk::dialog::message_default("错误: 未选择任何番剧或未获取到剧集信息");
            }
        });
    }

    app.run().unwrap();
}
