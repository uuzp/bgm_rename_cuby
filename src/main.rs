use fltk::{
    app,
    browser::{FileBrowser, MultiBrowser},
    button::Button,
    dialog::FileDialog,
    enums::{Event, Color},
    group::Flex,
    input::Input,
    prelude::*,
    window::Window,
};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
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
        }

        // 提取剧集编号和中文名称
        for episode in episodes_result.data {
            epn.push(episode.sort);
            let s = episode.name_cn;
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
    );    let mut main_flex = Flex::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, "");
    main_flex.set_type(fltk::group::FlexType::Row); // 水平排列

    // 左半部分
    let mut left_flex = Flex::new(0, 0, HALF_WIDTH, WINDOW_HEIGHT, "");
    left_flex.set_type(fltk::group::FlexType::Column); // 垂直排列
    left_flex.set_margin(5); // 添加一些边距

    let mut button_row_flex = Flex::new(0, 0, 0, 30, ""); // 高度固定，宽度由 left_flex 控制
    button_row_flex.set_type(fltk::group::FlexType::Row);
    let mut btn_choose_folder = Button::new(0, 0, 0, 0, "选择文件夹"); // 大小由 Flex 控制
    let mut btn_done = Button::new(0, 0, 0, 0, "完成"); // 大小由 Flex 控制
    button_row_flex.end();
    left_flex.fixed(&button_row_flex, 30); // 固定按钮行的高度

    let mut file_browser = FileBrowser::new(0, 0, 0, 0, ""); // 大小由 Flex 控制
    file_browser.set_selection_color(Color::Yellow);
    file_browser.set_type(fltk::browser::BrowserType::Hold); // 单选模式
    file_browser.set_damage(true); // Ensure redraws

    // For drag-and-drop reordering
    let dragged_line_index: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
    
    let d_idx_for_handle = dragged_line_index.clone();

    file_browser.handle(move |b, ev| {
        let mut d_idx = d_idx_for_handle.borrow_mut();
        
        match ev {
            Event::Push => {
                if app::event_clicks() {
                    // 获取FLTK FileBrowser自己检测到的行号
                    let line_num = b.value();
                    if line_num > 1 { // 忽略头部行
                        *d_idx = Some(line_num);
                        
                        // 打印选中行的信息用于调试
                        if let Some(text) = b.text(line_num) {
                            println!("开始拖拽第 {} 行: \"{}\"", line_num, text);
                        }
                        
                        return true;
                    }
                    *d_idx = None;
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
                    if to_line > 1 && to_line != from_line {
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

    let mut file_browser_clone = file_browser.clone();
    btn_choose_folder.set_callback(move |_| {
        let mut dialog = FileDialog::new(fltk::dialog::FileDialogType::BrowseDir);
        dialog.show();
        let chosen_path = dialog.filename();
        if !chosen_path.as_os_str().is_empty() {
            let path = Path::new(&chosen_path);
            if path.is_dir() {
                file_browser_clone.clear(); // 清空浏览器
                if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                    file_browser_clone.add(&format!("├─ {}", folder_name)); // 修改文件夹名称格式
                }

                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let file_path = entry.path();
                            if file_path.is_file() {
                                if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                                    match ext.to_lowercase().as_str() {
                                        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => {
                                            if let Some(file_name) =
                                                file_path.file_name().and_then(|n| n.to_str())
                                            {
                                                file_browser_clone.add(&format!("├─── {}", file_name)); // 修改文件名称格式，移除空格，延长横线
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
    });

    left_flex.end();
    main_flex.add(&left_flex); // 将左侧 Flex 添加到主 Flex    // 右半部分
    let mut right_flex = Flex::new(HALF_WIDTH, 0, HALF_WIDTH, WINDOW_HEIGHT, "");
    right_flex.set_type(fltk::group::FlexType::Column);
    right_flex.set_margin(5);

    let mut search_row_flex = Flex::new(0, 0, 0, 30, ""); // 高度固定，宽度由 right_flex 控制
    search_row_flex.set_type(fltk::group::FlexType::Row);
    let mut search_input = Input::new(0, 0, 0, 0, ""); // 大小由 Flex 控制
    search_input.set_tooltip("输入番剧名称关键字");
    let mut search_button = Button::new(0, 0, 40, 0, "🔎"); // 宽度固定，高度由 Flex 控制
    search_row_flex.fixed(&search_button, 40); // 固定搜索按钮宽度
    search_row_flex.end();
    right_flex.fixed(&search_row_flex, 30); // 固定搜索行高度

    let mut search_results_browser = MultiBrowser::new(0, 0, 0, 0, ""); // 大小由 Flex 控制
    search_results_browser.set_selection_color(Color::Yellow);
    search_results_browser.set_type(fltk::browser::BrowserType::Hold); // 单选模式

    right_flex.end();
    main_flex.add(&right_flex); // 将右侧 Flex 添加到主 Flex

    main_flex.end();
    wind.add(&main_flex); // 将主 Flex 添加到窗口

    wind.resizable(&main_flex); // 使主 Flex 可调整大小    wind.end();
    wind.show();

    // 创建共享数据结构
    let search_results: Rc<RefCell<Option<Bgm>>> = Rc::new(RefCell::new(None));
    let episode_list: Rc<RefCell<Option<Ep>>> = Rc::new(RefCell::new(None));
    let selected_anime_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
      // 设置搜索按钮回调
    let search_input_copy = search_input.clone();
    let search_results_copy = search_results.clone();
    let mut search_results_browser_copy = search_results_browser.clone();
    
    search_button.set_callback(move |_| {
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
    });    // 设置搜索结果双击事件处理
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

    // 设置"完成"按钮回调
    let file_browser_copy = file_browser.clone();
    let episode_list_copy = episode_list.clone();
      btn_done.set_callback(move |_| {
        // 获取文件浏览器中的文件列表（从第2行开始）
        let mut file_names = Vec::new();
        for i in 2..=file_browser_copy.size() {
            if let Some(text) = file_browser_copy.text(i) {
                if text.starts_with("├───") {
                    // 使用字符级别的操作而不是字节级别的索引
                    // 跳过前5个字符而不是字节
                    file_names.push(text.chars().skip(5).collect::<String>()); // 去除前缀 "├─── "
                }
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
                    println!("{} -> {}", file_name, ep.name[i]);
                    matched_pairs.push((file_name.clone(), ep.name[i].clone()));
                    matched_count += 1;
                }
            }
            
            println!("成功匹配: {}/{} 个文件", matched_count, file_names.len());
            
            if matched_count < file_names.len() {
                println!("警告: 有 {} 个文件没有对应的剧集信息", 
                    file_names.len() - matched_count);
            }
            
            // 这里可以将matched_pairs传递给其他函数进行进一步处理
            println!("总共匹配了 {} 个文件与剧集对", matched_pairs.len());
        } else {
            println!("错误: 未选择任何番剧或未获取到剧集信息");
        }
    });

    app.run().unwrap();
}
