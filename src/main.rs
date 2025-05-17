#![windows_subsystem = "windows"] // 禁止在 Windows 上显示控制台窗口

use fltk::{
    app,
    browser::{FileBrowser, MultiBrowser},
    button::Button,
    dialog, 
    enums::{Color, Event, Key}, 
    frame::Frame,
    group::Flex,
    input::Input,
    prelude::*,
    window::Window,
};
use std::{
    cell::RefCell,
    env, 
    path::{Path, Component},
    rc::Rc,
};
use winreg::enums::*; 
use winreg::RegKey;   

use clap::Parser;
use miniserde::{Deserialize, Serialize, json};
use minreq;
use urlencoding;
use webbrowser; 

// --- 通用网络请求辅助函数 ---
fn fetch_json_from_api<T: Deserialize>(url: &str, user_agent: Option<&str>) -> Result<T, String> {
    let mut request = minreq::get(url);
    if let Some(ua) = user_agent {
        request = request.with_header("User-Agent", ua);
    }

    let response = request.send().map_err(|e| format!("网络请求失败: {}", e))?;
    let response_text = response.as_str().map_err(|e| format!("响应文本编码错误: {}", e))?;

    match json::from_str(&response_text) {
        Ok(res) => Ok(res),
        Err(e) => {
            #[derive(Deserialize)]
            struct ApiErrorResponse {
                title: Option<String>,
                #[serde(rename = "type")]
                error_type: Option<String>,
                detail: Option<String>,
            }
            if let Ok(err_resp) = json::from_str::<ApiErrorResponse>(&response_text) {
                Err(format!(
                    "BGM API 错误: {}. 类型: {}. 详情: {}",
                    err_resp.title.unwrap_or_default(),
                    err_resp.error_type.unwrap_or_default(),
                    err_resp.detail.unwrap_or_default()
                ))
            } else {
                Err(format!("JSON 解析失败: {}. 原始响应: {}", e, response_text))
            }
        }
    }
}

// --- 数据模型 (直接用于反序列化和内部使用) ---
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BangumiSubject { // 重命名自 Subject
    pub id: u64,
    pub name: String,
    pub name_cn: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BangumiEpisode { // 重命名自 Episode
    pub airdate: String,
    pub sort: u64,
    pub name: String,
    pub name_cn: String,
}

// --- API 响应的顶层结构 ---
#[derive(Deserialize, Serialize)]
struct SubjectListPayload { // 重命名自 SearchResult
    list: Vec<BangumiSubject>,
}

#[derive(Deserialize, Serialize)]
struct EpisodeListPayload { // 重命名自 EpisodesResult
    data: Vec<BangumiEpisode>,
}

// --- 资源获取器特征 ---
pub trait ResourceFetcher<Param> where Self: Sized {
    fn fetch(param: Param) -> Result<Self, String>;
}

// --- 实现特征: 搜索番剧主题 ---
// 返回 Vec<BangumiSubject>
impl ResourceFetcher<&str> for Vec<BangumiSubject> {
    fn fetch(keywords: &str) -> Result<Self, String> {
        let encoded_keywords = urlencoding::encode(keywords);
        let url = format!(
            "https://api.bgm.tv/search/subject/{}?type=2&responseGroup=small",
            encoded_keywords
        );
        let payload: SubjectListPayload = fetch_json_from_api(&url, None)?;
        Ok(payload.list)
    }
}

// --- 剧集集合结构体 ---
#[derive(Debug, Clone)]
pub struct EpisodeCollection { // 重命名自 Ep
    pub episodes: Vec<BangumiEpisode>,
    pub year: i32,
}

impl EpisodeCollection {
    pub fn new_empty() -> Self { // 用于创建空实例
        Self { episodes: Vec::new(), year: 1970 }
    }

    pub fn get_formatted_names(&self) -> Vec<String> {
        if self.episodes.is_empty() {
            return Vec::new();
        }
        let mut ep_display_names = Vec::new();
        let mut ep_sort_numbers = Vec::new();

        for episode in &self.episodes {
            ep_sort_numbers.push(episode.sort);
            let name_to_use = if !episode.name_cn.is_empty() {
                &episode.name_cn
            } else {
                &episode.name
            };
            ep_display_names.push(replace_invalid_chars(name_to_use));
        }

        let mut formatted_names = vec![];
        let max_ep_num = ep_sort_numbers.iter().max().cloned().unwrap_or(0);
        let num_digits = if max_ep_num == 0 { 1 } else { (max_ep_num as f64).log10() as usize + 1 };

        for i in 0..ep_display_names.len() {
            let ep_num_str = format!("{:0width$}", ep_sort_numbers[i], width = num_digits);
            let formatted_ep_name = format!("ep{} - {}", ep_num_str, ep_display_names[i]);
            formatted_names.push(formatted_ep_name);
        }
        formatted_names
    }
}

// --- 实现特征: 获取剧集 ---
// 参数为 u64 类型的 subject_id
impl ResourceFetcher<u64> for EpisodeCollection {
    fn fetch(subject_id: u64) -> Result<Self, String> {
        let url = format!(
            "https://api.bgm.tv/v0/episodes?subject_id={}&type=0&limit=100&offset=0",
            subject_id 
        );
        let payload: EpisodeListPayload = fetch_json_from_api(&url, Some("uuzp/bgm_rename_cuby"))?;
        
        let mut year = 1970;
        if let Some(first_episode) = payload.data.first() {
            year = first_episode.airdate.get(0..4).unwrap_or("").parse().unwrap_or(1970);
        }
        
        Ok(EpisodeCollection { episodes: payload.data, year })
    }
}


// --- Main Function (Moved to top) ---
fn main() {
    let cli_args = CliArgs::parse();
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    let mut wind = create_main_window();

    let (base_path_rc, anime_path_rc) = initialize_paths_from_cli(&cli_args);

    let (mut btn_choose_base, mut btn_choose_anime, mut btn_done, mut search_input, mut search_button) = create_core_controls();
    let (mut settings_button, mut unregister_button, mut about_button) = create_menu_buttons();
    let (mut file_browser, mut search_results_browser) = create_main_browsers();

    let (
        mut main_vertical_flex,
        mut menu_trigger_button,
        mut path_trigger_button,
        mut menu_items_panel_flex,
        mut path_display_panel_flex,
    ) = build_ui_layout(
        &mut wind,
        &btn_choose_base, &btn_choose_anime, &btn_done, &search_input, &search_button,
        &settings_button, &unregister_button, &about_button,
        &mut file_browser, &mut search_results_browser,
    );

    // 修改 Rc<RefCell<Option<...>>> 的类型
    let (is_menu_expanded, is_path_panel_expanded, search_results_rc, episode_list_rc, selected_anime_id_rc) =
        initialize_ui_state_and_apply_cli_args(
            &base_path_rc, &anime_path_rc,
            &mut btn_choose_base, &mut btn_choose_anime,
            &mut file_browser, &mut search_input,
        );

    let file_browser_clone_for_base_cb = file_browser.clone();
    let search_input_clone_for_base_cb = search_input.clone();
    let search_results_browser_clone_for_search_actions = search_results_browser.clone();
    let file_browser_clone_for_done_cb = file_browser.clone();
    let search_results_browser_clone_for_done_cb = search_results_browser.clone();

    register_all_callbacks(
        &mut wind,
        &mut main_vertical_flex,
        &mut menu_trigger_button, is_menu_expanded.clone(), &mut menu_items_panel_flex,
        &mut path_trigger_button, is_path_panel_expanded.clone(), &mut path_display_panel_flex,
        &mut settings_button, &mut unregister_button, &mut about_button,
        &mut btn_choose_base, base_path_rc.clone(), file_browser_clone_for_base_cb, search_input_clone_for_base_cb,
        &mut btn_choose_anime, anime_path_rc.clone(),
        &mut search_input, &mut search_button, search_results_rc.clone(), search_results_browser_clone_for_search_actions,
        &mut search_results_browser, search_results_rc.clone(), episode_list_rc.clone(), selected_anime_id_rc.clone(),
        &mut file_browser,
        &mut btn_done, file_browser_clone_for_done_cb, episode_list_rc.clone(), base_path_rc.clone(), anime_path_rc.clone(),
        search_results_rc.clone(), selected_anime_id_rc.clone(), search_results_browser_clone_for_done_cb,
    );

    wind.show();
    app.run().unwrap();
}

// --- Helper Functions for main() ---

fn create_main_window() -> Window {
    Window::new(
        100,
        100,
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        "BGM Rename Cuby - 番剧文件批量重命名工具",
    )
}

fn initialize_paths_from_cli(cli_args: &CliArgs) -> (Rc<RefCell<Option<String>>>, Rc<RefCell<Option<String>>>) {
    (
        Rc::new(RefCell::new(cli_args.base_path.clone())),
        Rc::new(RefCell::new(cli_args.anime_path.clone())),
    )
}

fn create_core_controls() -> (Button, Button, Button, Input, Button) {
    let mut btn_choose_base = Button::new(0, 0, 0, 0, "选择源路径 (B)");
    btn_choose_base.set_tooltip("选择包含视频文件的源文件夹 (B)");
    let mut btn_choose_anime = Button::new(0, 0, 0, 0, "选择目标路径 (A)");
    btn_choose_anime.set_tooltip("选择重命名后文件存放的目标文件夹 (A)");
    let mut btn_done = Button::new(0, 0, 0, 0, "✔️ 完成");
    btn_done.set_tooltip("开始重命名操作");
    let mut search_input = Input::new(0, 0, 0, 0, "");
    search_input.set_tooltip("输入番剧名称关键字进行搜索");
    let mut search_button = Button::new(0, 0, 40, 0, "🔎");
    search_button.set_tooltip("点击搜索");
    (btn_choose_base, btn_choose_anime, btn_done, search_input, search_button)
}

fn create_menu_buttons() -> (Button, Button, Button) {
    let mut settings_button = Button::new(0, 0, 0, 30, "📝 注册");
    settings_button.set_tooltip("注册右键菜单到系统");
    let mut unregister_button = Button::new(0, 0, 0, 30, "🗑️ 注销");
    unregister_button.set_tooltip("从系统注销右键菜单");
    let mut about_button = Button::new(0, 0, 0, 30, "📦 关于");
    about_button.set_tooltip("查看项目信息");
    (settings_button, unregister_button, about_button)
}

fn create_main_browsers() -> (FileBrowser, MultiBrowser) {
    let mut file_browser = FileBrowser::new(0, 0, 0, 0, "");
    file_browser.set_tooltip("源文件夹中的文件列表");
    file_browser.set_selection_color(Color::Yellow);
    file_browser.set_type(fltk::browser::BrowserType::Hold); // Allow multi-selection if needed, or Single
    file_browser.set_damage(true);

    let mut search_results_browser = MultiBrowser::new(0, 0, 0, 0, "");
    search_results_browser.set_tooltip("Bangumi API 搜索结果");
    search_results_browser.set_selection_color(Color::Yellow);
    search_results_browser.set_type(fltk::browser::BrowserType::Hold); // Should be Single for selection
    (file_browser, search_results_browser)
}

fn build_ui_layout(
    wind: &mut Window,
    btn_choose_base_ref: &Button, btn_choose_anime_ref: &Button, btn_done_ref: &Button,
    search_input_ref: &Input, search_button_ref: &Button,
    settings_button_ref: &Button, unregister_button_ref: &Button, about_button_ref: &Button,
    file_browser: &mut FileBrowser, search_results_browser: &mut MultiBrowser,
) -> (Flex, Button, Button, Flex, Flex) {
    let mut main_vertical_flex = Flex::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, "");
    main_vertical_flex.set_type(fltk::group::FlexType::Column);

    let (menu_trigger_button, path_trigger_button, top_triggers_flex) =
        setup_top_triggers_flex(btn_done_ref, search_input_ref, search_button_ref);
    main_vertical_flex.add(&top_triggers_flex);
    main_vertical_flex.fixed(&top_triggers_flex, MENU_TRIGGER_HEIGHT);

    let menu_items_panel_flex =
        setup_menu_items_panel(settings_button_ref, unregister_button_ref, about_button_ref);
    main_vertical_flex.add(&menu_items_panel_flex);

    let path_display_panel_flex = setup_path_display_panel(btn_choose_base_ref, btn_choose_anime_ref);
    main_vertical_flex.add(&path_display_panel_flex);

    let (content_flex, mut left_flex, mut right_flex) = setup_content_area();
    left_flex.add(file_browser);
    right_flex.add(search_results_browser);
    main_vertical_flex.add(&content_flex);

    main_vertical_flex.end();
    wind.add(&main_vertical_flex);
    wind.resizable(&main_vertical_flex);

    (main_vertical_flex, menu_trigger_button, path_trigger_button, menu_items_panel_flex, path_display_panel_flex)
}

fn initialize_ui_state_and_apply_cli_args(
    base_path_rc: &Rc<RefCell<Option<String>>>,
    anime_path_rc: &Rc<RefCell<Option<String>>>,
    btn_choose_base: &mut Button,
    btn_choose_anime: &mut Button,
    file_browser: &mut FileBrowser,
    search_input: &mut Input,
) -> (
    Rc<RefCell<bool>>,
    Rc<RefCell<bool>>,
    Rc<RefCell<Option<Vec<BangumiSubject>>>>, // 修改类型
    Rc<RefCell<Option<EpisodeCollection>>>,  // 修改类型
    Rc<RefCell<Option<String>>>, // selected_anime_id_rc (保持 String, 因为API ID可能很大)
) {
    let is_menu_expanded = Rc::new(RefCell::new(false));
    let is_path_panel_expanded = Rc::new(RefCell::new(false));
    // 修改类型
    let search_results_rc: Rc<RefCell<Option<Vec<BangumiSubject>>>> = Rc::new(RefCell::new(None));
    let episode_list_rc: Rc<RefCell<Option<EpisodeCollection>>> = Rc::new(RefCell::new(None));
    let selected_anime_id_rc: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    if let Some(cli_base_path_str) = base_path_rc.borrow().as_deref() {
        btn_choose_base.set_label(&shorten_path_for_display(cli_base_path_str, MAX_BUTTON_LABEL_LEN));
        load_files_to_file_browser(cli_base_path_str, file_browser);
        if let Some(extracted_anime_name) = extract_anime_name_from_path(cli_base_path_str) {
            search_input.set_value(&extracted_anime_name);
            println!("从命令行路径 {} 提取到番剧名: {}", cli_base_path_str, extracted_anime_name);
        }
    }
    if let Some(cli_anime_path_str) = anime_path_rc.borrow().as_deref() {
        btn_choose_anime.set_label(&shorten_path_for_display(cli_anime_path_str, MAX_BUTTON_LABEL_LEN));
    }

    (is_menu_expanded, is_path_panel_expanded, search_results_rc, episode_list_rc, selected_anime_id_rc)
}

#[allow(clippy::too_many_arguments)] 
fn register_all_callbacks(
    wind: &mut Window,
    main_vertical_flex: &mut Flex,
    menu_trigger_button: &mut Button,
    is_menu_expanded: Rc<RefCell<bool>>,
    menu_items_panel_flex: &mut Flex,
    path_trigger_button: &mut Button,
    is_path_panel_expanded: Rc<RefCell<bool>>,
    path_display_panel_flex: &mut Flex,
    settings_button: &mut Button,
    unregister_button: &mut Button,
    about_button: &mut Button,
    btn_choose_base: &mut Button,
    base_path_rc: Rc<RefCell<Option<String>>>,
    file_browser_for_b_cb: FileBrowser,
    search_input_for_b_cb: Input,
    btn_choose_anime: &mut Button,
    anime_path_rc: Rc<RefCell<Option<String>>>,
    search_input_for_search_cb: &mut Input,
    search_button: &mut Button,
    search_results_rc_for_search: Rc<RefCell<Option<Vec<BangumiSubject>>>>, // 修改类型
    search_results_browser_for_search_actions: MultiBrowser, 
    search_results_browser: &mut MultiBrowser,
    search_results_rc_for_dblclick: Rc<RefCell<Option<Vec<BangumiSubject>>>>, // 修改类型
    episode_list_rc_for_dblclick: Rc<RefCell<Option<EpisodeCollection>>>, // 修改类型
    selected_anime_id_rc_for_dblclick: Rc<RefCell<Option<String>>>,
    file_browser: &mut FileBrowser,
    btn_done: &mut Button,
    file_browser_for_done_cb: FileBrowser,
    episode_list_rc_for_done: Rc<RefCell<Option<EpisodeCollection>>>, // 修改类型
    base_path_rc_for_done: Rc<RefCell<Option<String>>>,
    anime_path_rc_for_done: Rc<RefCell<Option<String>>>,
    search_results_rc_for_done: Rc<RefCell<Option<Vec<BangumiSubject>>>>, // 修改类型
    selected_anime_id_rc_for_done: Rc<RefCell<Option<String>>>,
    search_results_browser_for_done_cb: MultiBrowser,
) {
    // Menu Toggle Callback
    let is_menu_expanded_cb = is_menu_expanded.clone();
    let mut main_flex_cb_menu = main_vertical_flex.clone();
    let mut menu_panel_cb_menu = menu_items_panel_flex.clone();
    let mut wind_cb_menu = wind.clone();
    menu_trigger_button.set_callback(move |_| {
        handle_menu_toggle(
            is_menu_expanded_cb.clone(),
            &mut main_flex_cb_menu,
            &mut menu_panel_cb_menu,
            &mut wind_cb_menu,
        );
    });

    // Path Panel Toggle Callback
    let is_path_panel_expanded_cb = is_path_panel_expanded.clone();
    let mut main_flex_cb_path = main_vertical_flex.clone();
    let mut path_panel_cb_path = path_display_panel_flex.clone();
    let mut wind_cb_path = wind.clone();
    path_trigger_button.set_callback(move |_| {
        handle_path_panel_toggle(
            is_path_panel_expanded_cb.clone(),
            &mut main_flex_cb_path,
            &mut path_panel_cb_path,
            &mut wind_cb_path,
        );
    });

    // Menu Item Callbacks
    settings_button.set_callback(|_| handle_register_context_menu());
    unregister_button.set_callback(|_| handle_unregister_context_menu());
    about_button.set_callback(|_| handle_about_button());

    // Path Choose Callbacks
    let base_path_cb_b = base_path_rc.clone();
    let btn_choose_base_cb_b = btn_choose_base.clone();
    // file_browser_for_b_cb and search_input_for_b_cb are cloned for the closure
    btn_choose_base.set_callback(move |_| {
        handle_choose_base_path_callback(
            base_path_cb_b.clone(),
            btn_choose_base_cb_b.clone(),
            file_browser_for_b_cb.clone(), 
            search_input_for_b_cb.clone(),  
        );
    });

    let anime_path_cb_a = anime_path_rc.clone();
    let btn_choose_anime_cb_a = btn_choose_anime.clone();
    btn_choose_anime.set_callback(move |_| {
        handle_choose_anime_path_callback(
            anime_path_cb_a.clone(),
            btn_choose_anime_cb_a.clone(),
        );
    });
    
    // Search Callbacks
    let search_input_cb_search_btn = search_input_for_search_cb.clone();
    let search_results_cb_search_btn = search_results_rc_for_search.clone();
    let srb_for_search_button_closure = search_results_browser_for_search_actions.clone();
    search_button.set_callback(move |_| {
        handle_search_button_callback(
            search_input_cb_search_btn.clone(),
            search_results_cb_search_btn.clone(),
            srb_for_search_button_closure.clone(), 
        );
    });

    let search_input_cb_enter = search_input_for_search_cb.clone();
    let search_results_cb_enter = search_results_rc_for_search.clone();
    search_input_for_search_cb.handle(move |_, ev| {
        if ev == Event::KeyDown && app::event_key() == Key::Enter {
            return handle_search_input_enter_key(
                search_input_cb_enter.clone(),
                search_results_cb_enter.clone(),
                search_results_browser_for_search_actions.clone(), 
            );
        }
        false
    });

    // Search Results Browser Callback
    search_results_browser.set_callback(move |b| {
        handle_search_results_double_click(
            b,
            search_results_rc_for_dblclick.clone(),
            episode_list_rc_for_dblclick.clone(),
            selected_anime_id_rc_for_dblclick.clone(),
        );
    });

    // File Browser Callback
    file_browser.handle(move |b, ev| {
        handle_file_browser_events(b, ev)
    });

    // Done Button Callback
    btn_done.set_callback(move |_| {
        handle_done_button_callback(
            file_browser_for_done_cb.clone(),
            episode_list_rc_for_done.clone(),
            base_path_rc_for_done.clone(),
            anime_path_rc_for_done.clone(),
            search_results_rc_for_done.clone(),
            selected_anime_id_rc_for_done.clone(),
            search_results_browser_for_done_cb.clone(),
        );
    });
}


// --- Data Structures and API Logic (Existing Code) ---

/// 替换文件名中的特殊字符
pub fn replace_invalid_chars(s: &str) -> String {
    s.replace("/", "／")
     .replace("\\\\", "＼") // Note: in a regular string, this would be a single backslash.
     .replace("<", "＜")
     .replace(">", "＞")
     // Consider adding other common problematic characters like : * ? " |
     .replace(":", "：")
     .replace("*", "＊")
     .replace("?", "？")
     .replace("\"", "＂")
     .replace("|", "｜")
}


#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// 源文件路径（包含视频文件的文件夹）
    #[arg(short, long)]
    base_path: Option<String>,

    /// 目标文件路径（重命名后文件存放的文件夹）
    #[arg(short, long)]
    anime_path: Option<String>,
}

// BgmApi and Ep structs are now replaced by ResourceFetcher implementations
// and EpisodeCollection struct.

// ... (Constants and Utility Functions like extract_anime_name_from_path, shorten_path_for_display remain) ...
// ... (UI setup functions like setup_top_triggers_flex, etc. remain) ...
// ... (Event handlers like handle_register_context_menu, etc. remain) ...
// ... (load_files_to_file_browser remains) ...

// --- 回调处理函数 ---
// ... (handle_menu_toggle, handle_path_panel_toggle, handle_choose_base_path_callback, handle_choose_anime_path_callback remain) ...

/// 处理搜索按钮点击的回调
fn handle_search_button_callback(
    search_input: Input, 
    search_results_rc: Rc<RefCell<Option<Vec<BangumiSubject>>>>, // 修改类型
    mut search_results_browser: MultiBrowser, 
) {
    let query = search_input.value();
    if !query.is_empty() {
        println!("正在搜索: {}", query);
        match <Vec<BangumiSubject> as ResourceFetcher<&str>>::fetch(&query) {
            Ok(subjects) => { 
                println!("搜索成功，找到 {} 个结果", subjects.len());
                search_results_browser.clear();
                for subject in &subjects { 
                    let display_name = if subject.name_cn.is_empty() { 
                        &subject.name 
                    } else { 
                        &subject.name_cn 
                    };
                    search_results_browser.add(&display_name.replace("&", "&&"));
                }
                *search_results_rc.borrow_mut() = Some(subjects); 
            }
            Err(err_msg) => {
                println!("搜索失败: {}", err_msg);
                dialog::message_default(&format!("搜索失败: {}", err_msg));
                *search_results_rc.borrow_mut() = None; // 清空旧结果
            }
        }
    } else {
        println!("搜索查询为空，不执行搜索。");
        search_results_browser.clear(); // 清空浏览器
        *search_results_rc.borrow_mut() = None; // 清空数据
    }
}

/// 处理搜索输入框回车键事件
fn handle_search_input_enter_key(
    search_input: Input, 
    search_results_rc: Rc<RefCell<Option<Vec<BangumiSubject>>>>, // 修改类型
    mut search_results_browser: MultiBrowser, 
) -> bool { 
    let query = search_input.value();
    if !query.is_empty() {
        println!("通过回车搜索: {}", query);
        match <Vec<BangumiSubject> as ResourceFetcher<&str>>::fetch(&query) {
            Ok(subjects) => { 
                println!("搜索成功，找到 {} 个结果", subjects.len());
                search_results_browser.clear();
                for subject in &subjects { 
                    let display_name = if subject.name_cn.is_empty() { 
                        &subject.name 
                    } else { 
                        &subject.name_cn 
                    };
                    search_results_browser.add(&display_name.replace("&", "&&"));
                }
                *search_results_rc.borrow_mut() = Some(subjects); 
            }
            Err(err_msg) => {
                println!("搜索失败: {}", err_msg);
                dialog::message_default(&format!("搜索失败: {}", err_msg));
                *search_results_rc.borrow_mut() = None; // 清空旧结果
            }
        }
    } else {
        println!("搜索查询为空，不执行搜索。");
        search_results_browser.clear(); // 清空浏览器
        *search_results_rc.borrow_mut() = None; // 清空数据
    }
    true 
}

/// 处理搜索结果列表项双击事件
fn handle_search_results_double_click(
    browser: &mut MultiBrowser, 
    search_results_rc: Rc<RefCell<Option<Vec<BangumiSubject>>>>, // 修改类型
    episode_list_rc: Rc<RefCell<Option<EpisodeCollection>>>,    // 修改类型
    selected_anime_id_rc: Rc<RefCell<Option<String>>>,
) {
    if app::event_clicks() { 
        let line = browser.value(); 
        if line > 0 && line <= browser.size() {
            if let Some(subjects) = &*search_results_rc.borrow() { // 使用 subjects
                let idx = (line as usize) - 1; 
                if idx < subjects.len() {
                    let subject_id = subjects[idx].id; // 直接获取 u64 ID
                    println!("双击选中番剧ID: {}", subject_id); 

                    match <EpisodeCollection as ResourceFetcher<u64>>::fetch(subject_id) {
                        Ok(ep_collection) => {
                            println!("获取到剧集信息: {} 个", ep_collection.episodes.len());
                            *episode_list_rc.borrow_mut() = Some(ep_collection);
                            *selected_anime_id_rc.borrow_mut() = Some(subject_id.to_string());
                        }
                        Err(err_msg) => {
                            println!("获取剧集列表失败: {}", err_msg); 
                            dialog::message_default(&format!("获取剧集列表失败: {}", err_msg));
                            *episode_list_rc.borrow_mut() = None;
                            *selected_anime_id_rc.borrow_mut() = None;
                        }
                    }
                }
            }
        }
    }
}

/// 从 RefCell 中安全地获取剧集数据以供处理
fn get_episode_data_for_processing(
    episode_list_rc: &Rc<RefCell<Option<EpisodeCollection>>>
) -> Result<EpisodeCollection, String> {
    match episode_list_rc.borrow().as_ref() {
        Some(ep_data) => Ok(ep_data.clone()),
        None => {
            let err_msg = "剧集列表为空，无法执行操作。请先在右侧搜索并双击选定一部番剧。";
            println!("{}", err_msg);
            Err(err_msg.to_string())
        }
    }
}

/// 新增 "完成" 按钮回调处理函数
#[allow(clippy::too_many_arguments)]
fn handle_done_button_callback(
    file_browser: FileBrowser,
    episode_list_rc: Rc<RefCell<Option<EpisodeCollection>>>,
    base_path_rc: Rc<RefCell<Option<String>>>,
    anime_path_rc: Rc<RefCell<Option<String>>>,
    search_results_rc: Rc<RefCell<Option<Vec<BangumiSubject>>>>,
    selected_anime_id_rc: Rc<RefCell<Option<String>>>,
    search_results_browser: MultiBrowser,
) {
    println!("Done button clicked!");
    println!("File browser state: value = {}, size = {}", file_browser.value(), file_browser.size());
    if file_browser.value() > 0 {
        if let Some(text) = file_browser.text(file_browser.value()) {
            println!("Focused file in file_browser: {}", text);
        }
    }

    if episode_list_rc.borrow().is_none() {
        println!("No episode data available.");
        dialog::message_default("错误：剧集列表为空。
请先在右侧搜索并双击选定一部番剧。");
        return;
    }
     if base_path_rc.borrow().is_none() {
        println!("Base path is not set.");
        dialog::message_default("错误：源路径未设定。");
        return;
    }

    let _episodes = episode_list_rc.borrow();
    let _base_p = base_path_rc.borrow();
    let _anime_p = anime_path_rc.borrow();
    let _search_res = search_results_rc.borrow();
    let _selected_id = selected_anime_id_rc.borrow();
    println!("Selected anime ID: {:?}", _selected_id.as_deref().unwrap_or("None"));
    println!("Search results browser has {} items, selected: {}", search_results_browser.size(), search_results_browser.value());

    dialog::message_default("重命名功能尚未实现。
请在控制台查看选定项的调试信息。");
}

// --- Constants and Utility Functions (Existing Code) ---
const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const HALF_WIDTH: i32 = WINDOW_WIDTH / 2;
const MENU_TRIGGER_HEIGHT: i32 = 30; // 菜单触发器行高度
const MENU_ITEMS_PANEL_EXPANDED_HEIGHT: i32 = 35; // 菜单项面板展开高度
const PATH_DISPLAY_PANEL_EXPANDED_HEIGHT: i32 = 30; // 路径显示面板展开高度
const MAX_BUTTON_LABEL_LEN: usize = 20; // 按钮标签最大显示字符数（粗略）

/// 从路径中提取番剧名的函数
fn extract_anime_name_from_path(path_str: &str) -> Option<String> {
    let path = Path::new(path_str);
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

// --- 新增的事件处理函数 ---
fn handle_register_context_menu() {
    match env::current_exe() {
        Ok(exe_path_buf) => {
            let exe_path = exe_path_buf.to_string_lossy().to_string();
            let mut errors = Vec::new();

            let hkey_classes_root = RegKey::predef(HKEY_CLASSES_ROOT);

            // 注册文件夹右键菜单
            let dir_shell_path = "Directory\\shell";
            let dir_key_name = "BgmRenameCuby";
            let dir_command_val = format!("\"{}\" -b \"%1\" -a \"%1\\anime\"", exe_path); // Removed extra backslashes

            match hkey_classes_root.create_subkey(format!("{}\\{}", dir_shell_path, dir_key_name)) {
                Ok((key, _disp)) => {
                    if let Err(e) = key.set_value("", &"使用 BgmRenameCuby 处理文件夹") { errors.push(format!("设置文件夹菜单默认值失败: {}", e)); }
                    if let Err(e) = key.set_value("Icon", &format!("\"{}\",0", exe_path)) { errors.push(format!("设置文件夹菜单Icon失败: {}", e)); } // Icon format
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
            let dir_bg_command_val = format!("\"{}\" -b \"%V\" -a \"%V\\anime\"", exe_path); // Removed extra backslashes

            match hkey_classes_root.create_subkey(format!("{}\\{}", dir_bg_shell_path, dir_key_name)) {
                Ok((key, _disp)) => {
                    if let Err(e) = key.set_value("", &"BgmRenameCuby 在此处理") { errors.push(format!("设置背景菜单默认值失败: {}", e)); }
                    if let Err(e) = key.set_value("Icon", &format!("\"{}\",0", exe_path)) { errors.push(format!("设置背景菜单Icon失败: {}", e)); } // Icon format
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
                dialog::message_default("注册表项已成功添加/更新。\n部分更改可能需要重启资源管理器或重新登录才能生效。");
            } else {
                dialog::message_default(&format!("注册表操作时发生错误:\n{}\n\n请确保以管理员身份运行本程序。", errors.join("\n")));
            }
        }
        Err(e) => {
            dialog::message_default(&format!("获取程序路径失败: {}", e));
        }
    }
}

fn handle_unregister_context_menu() {
    let mut errors = Vec::new();
    let hkey_classes_root = RegKey::predef(HKEY_CLASSES_ROOT);
    let mut deleted_anything = false;
    let dir_key_path = "Directory\\shell\\BgmRenameCuby";
    let dir_bg_key_path = "Directory\\Background\\shell\\BgmRenameCuby";

    match hkey_classes_root.delete_subkey_all(dir_key_path) {
        Ok(_) => { deleted_anything = true; }
        Err(e) => { if e.kind() != std::io::ErrorKind::NotFound { errors.push(format!("删除文件夹菜单项失败 (HKCR\\{}): {}", dir_key_path, e)); } }
    }
    match hkey_classes_root.delete_subkey_all(dir_bg_key_path) {
        Ok(_) => { deleted_anything = true; }
        Err(e) => { if e.kind() != std::io::ErrorKind::NotFound { errors.push(format!("删除背景菜单项失败 (HKCR\\{}): {}", dir_bg_key_path, e)); } }
    }

    if errors.is_empty() {
        if deleted_anything { dialog::message_default("相关注册表项已成功删除。\n部分更改可能需要重启资源管理器或重新登录才能生效。"); } 
        else { dialog::message_default("未找到相关的注册表项，无需注销。"); }
    } else {
        dialog::message_default(&format!("注销操作时发生错误:\n{}\n\n请确保以管理员身份运行本程序。", errors.join("\n")));
    }
}

fn handle_about_button() {
    let repo_url = "https://github.com/uuzp/bgm_rename_cuby";
    if webbrowser::open(repo_url).is_err() {
        dialog::message_default(&format!("无法打开链接: {}", repo_url));
        println!("Error opening URL: {}", repo_url);
    }
}

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
fn shorten_path_for_display(path_str: &str, max_len: usize) -> String {
    if path_str.is_empty() { return "".to_string(); }
    if path_str.len() <= max_len { return path_str.to_string(); }

    let path = Path::new(path_str);
    let ellipsis = "...\\"; 

    let drive_prefix_str = path.components().next().and_then(|c| match c {
        Component::Prefix(prefix_component) => {
            let prefix_os_str = prefix_component.as_os_str();
            let prefix_cow = prefix_os_str.to_string_lossy();
            let s = prefix_cow.as_ref();
            if s.ends_with(':') { 
                Some(format!("{}:\\", s.trim_end_matches(':')))
            } else if s.starts_with("\\\\") { 
                Some(format!("{}\\", s.trim_end_matches('\\')))
            } else { 
                Some(format!("{}\\", s.trim_end_matches('\\')))
            }
        },
        _ => None, 
    }).unwrap_or_else(|| {
        if path_str.len() > 2 && path_str.chars().nth(1) == Some(':') && path_str.chars().nth(2) == Some('\\') {
            format!("{}:\\", path_str.chars().next().unwrap_or_default())
        } else if path_str.starts_with("\\\\") {
            let parts: Vec<&str> = path_str.splitn(4, '\\').filter(|s| !s.is_empty()).collect();
            if parts.len() >= 2 { format!("\\\\{}\\{}\\", parts[0], parts[1]) } else { "".to_string() }
        }
        else { "".to_string() } 
    });

    if !drive_prefix_str.is_empty() && 
       (path_str == drive_prefix_str.trim_end_matches('\\') || path_str == drive_prefix_str) {
        return if drive_prefix_str.len() <= max_len { drive_prefix_str } 
               else { drive_prefix_str.chars().take(max_len).collect() };
    }

    let final_component_name = path.file_name().and_then(|os_str| os_str.to_str()).unwrap_or("");
    let parent_folder_name = path.parent().and_then(|p| p.file_name()).and_then(|os_str| os_str.to_str()).unwrap_or("");

    if !drive_prefix_str.is_empty() && !parent_folder_name.is_empty() && !final_component_name.is_empty() {
        let len_with_parent_and_final = drive_prefix_str.len() + ellipsis.len() + parent_folder_name.len() + 1 + final_component_name.len();
        if len_with_parent_and_final <= max_len {
            return format!("{}{}{}\\{}", drive_prefix_str, ellipsis, parent_folder_name, final_component_name);
        }
        let space_for_final_after_parent = max_len.saturating_sub(drive_prefix_str.len() + ellipsis.len() + parent_folder_name.len() + 1);
        if space_for_final_after_parent > 0 { 
            let shortened_final: String = final_component_name.chars().take(space_for_final_after_parent).collect();
            if !shortened_final.is_empty() { 
                return format!("{}{}{}\\{}", drive_prefix_str, ellipsis, parent_folder_name, shortened_final);
            }
        }
    }

    if !drive_prefix_str.is_empty() && !final_component_name.is_empty() {
        let len_with_final_only = drive_prefix_str.len() + ellipsis.len() + final_component_name.len();
        if len_with_final_only <= max_len {
            return format!("{}{}{}", drive_prefix_str, ellipsis, final_component_name);
        }
        let remaining_space_for_filename = max_len.saturating_sub(drive_prefix_str.len()).saturating_sub(ellipsis.len());
        if remaining_space_for_filename > 0 { 
            let shortened_filename: String = final_component_name.chars().take(remaining_space_for_filename).collect();
            if !shortened_filename.is_empty() { 
                return format!("{}{}{}", drive_prefix_str, ellipsis, shortened_filename);
            }
        }
    }
    
    let fallback_ellipsis = "..."; 
    if max_len > fallback_ellipsis.len() {
        let chars_from_end_to_take = max_len - fallback_ellipsis.len();
        let skip_count = path_str.chars().count().saturating_sub(chars_from_end_to_take); // Use chars().count() for Unicode
        format!("{}{}", fallback_ellipsis, &path_str.chars().skip(skip_count).collect::<String>())
    } else {
        path_str.chars().take(max_len).collect()
    }
}

fn setup_top_triggers_flex(btn_done: &Button, search_input: &Input, search_button: &Button) -> (Button, Button, Flex) { 
    let mut top_triggers_flex = Flex::new(0, 0, WINDOW_WIDTH, MENU_TRIGGER_HEIGHT, "");
    top_triggers_flex.set_type(fltk::group::FlexType::Row);
    let mut menu_trigger_button = Button::new(0, 0, 80, 0, "菜单 ☰");
    menu_trigger_button.set_tooltip("显示/隐藏菜单项");
    top_triggers_flex.add(&menu_trigger_button);
    top_triggers_flex.fixed(&menu_trigger_button, 80);
    let mut path_trigger_button = Button::new(0, 0, 80, 0, "路径 🗀");
    path_trigger_button.set_tooltip("显示/隐藏路径选择按钮");
    top_triggers_flex.add(&path_trigger_button);
    top_triggers_flex.fixed(&path_trigger_button, 80);
    let top_spacer = Frame::new(0,0,0,0,""); 
    top_triggers_flex.add(&top_spacer);
    top_triggers_flex.add(btn_done); 
    top_triggers_flex.fixed(btn_done, 80);
    let search_gap_spacer = Frame::new(0,0,10,0,""); 
    top_triggers_flex.add(&search_gap_spacer);
    top_triggers_flex.fixed(&search_gap_spacer, 10);
    top_triggers_flex.add(search_input); 
    top_triggers_flex.add(search_button); 
    top_triggers_flex.fixed(search_button, 40);
    top_triggers_flex.end();
    (menu_trigger_button, path_trigger_button, top_triggers_flex)
}

fn setup_menu_items_panel(settings_button: &Button, unregister_button: &Button, about_button: &Button) -> Flex {
    let mut menu_items_panel_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, ""); 
    menu_items_panel_flex.set_type(fltk::group::FlexType::Row);
    menu_items_panel_flex.set_margin(2);
    menu_items_panel_flex.add(settings_button);
    menu_items_panel_flex.fixed(settings_button, 80);
    menu_items_panel_flex.add(unregister_button);
    menu_items_panel_flex.fixed(unregister_button, 80);
    menu_items_panel_flex.add(about_button);
    menu_items_panel_flex.fixed(about_button, 100);
    menu_items_panel_flex.end();
    menu_items_panel_flex.hide(); 
    menu_items_panel_flex
}

fn setup_path_display_panel(btn_choose_base: &Button, btn_choose_anime: &Button) -> Flex {
    let mut path_display_panel_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, ""); 
    path_display_panel_flex.set_type(fltk::group::FlexType::Row);
    path_display_panel_flex.set_margin(2);
    path_display_panel_flex.add(btn_choose_base);
    path_display_panel_flex.add(btn_choose_anime);
    path_display_panel_flex.end();
    path_display_panel_flex.hide(); 
    path_display_panel_flex
}

fn setup_content_area() -> (Flex, Flex, Flex) { 
    let mut content_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, ""); 
    content_flex.set_type(fltk::group::FlexType::Row);
    let mut left_flex = Flex::new(0, 0, HALF_WIDTH, 0, ""); 
    left_flex.set_type(fltk::group::FlexType::Column);
    left_flex.set_margin(5);
    left_flex.end();
    let mut right_flex = Flex::new(HALF_WIDTH, 0, HALF_WIDTH, 0, ""); 
    right_flex.set_type(fltk::group::FlexType::Column);
    right_flex.set_margin(5);
    right_flex.end();
    content_flex.add(&left_flex);
    content_flex.add(&right_flex);
    content_flex.end();
    (content_flex, left_flex, right_flex)
}

fn handle_menu_toggle(is_menu_expanded: Rc<RefCell<bool>>, main_flex: &mut Flex, menu_panel: &mut Flex, window: &mut Window) {
    let mut expanded = is_menu_expanded.borrow_mut();
    *expanded = !*expanded; 
    if *expanded {
        main_flex.fixed(menu_panel, MENU_ITEMS_PANEL_EXPANDED_HEIGHT);
        menu_panel.show();
    } else {
        menu_panel.hide();
        main_flex.fixed(menu_panel, 0); 
    }
    main_flex.layout(); 
    window.redraw(); 
}

fn handle_path_panel_toggle(is_path_panel_expanded: Rc<RefCell<bool>>, main_flex: &mut Flex, path_panel: &mut Flex, window: &mut Window) {
    let mut expanded = is_path_panel_expanded.borrow_mut();
    *expanded = !*expanded; 
    if *expanded {
        main_flex.fixed(path_panel, PATH_DISPLAY_PANEL_EXPANDED_HEIGHT);
        path_panel.show();
    } else {
        path_panel.hide();
        main_flex.fixed(path_panel, 0); 
    }
    main_flex.layout(); 
    window.redraw(); 
}

fn handle_choose_base_path_callback(base_path_rc: Rc<RefCell<Option<String>>>, mut btn_choose_base: Button, mut file_browser: FileBrowser, mut search_input: Input) {
    let mut dialog = dialog::FileDialog::new(dialog::FileDialogType::BrowseDir);
    dialog.set_title("选择源文件夹 (B)");
    dialog.show();
    let chosen_path_pb = dialog.filename();
    if !chosen_path_pb.as_os_str().is_empty() {
        let path = Path::new(&chosen_path_pb);
        if path.is_dir() {
            if let Some(path_str) = path.to_str() {
                *base_path_rc.borrow_mut() = Some(path_str.to_string());
                btn_choose_base.set_label(&shorten_path_for_display(path_str, MAX_BUTTON_LABEL_LEN));
                load_files_to_file_browser(path_str, &mut file_browser);
                if let Some(extracted_anime_name) = extract_anime_name_from_path(path_str) {
                    search_input.set_value(&extracted_anime_name);
                    println!("从路径 {} 提取到番剧名: {}", path_str, extracted_anime_name);
                }
            }
        }
    }
}

fn handle_choose_anime_path_callback(anime_path_rc: Rc<RefCell<Option<String>>>, mut btn_choose_anime: Button) {
    let mut dialog = dialog::FileDialog::new(dialog::FileDialogType::BrowseDir);
    dialog.set_title("选择目标文件夹 (A)");
    dialog.show();
    let chosen_path = dialog.filename();
    if !chosen_path.as_os_str().is_empty() {
        let path = Path::new(&chosen_path);
        if path.is_dir() {
            if let Some(path_str) = path.to_str() {
                *anime_path_rc.borrow_mut() = Some(path_str.to_string());
                btn_choose_anime.set_label(&shorten_path_for_display(path_str, MAX_BUTTON_LABEL_LEN));
            }
        }
    }
}

fn validate_operation_paths(base_path_rc: &Rc<RefCell<Option<String>>>, anime_path_rc: &Rc<RefCell<Option<String>>>) -> Result<(String, String), String> {
    let base_path_str = match base_path_rc.borrow().as_ref() {
        Some(path) => path.clone(),
        None => { return Err("错误: 未设置源文件路径（B按钮）".to_string()); }
    };
    let anime_path_str = match anime_path_rc.borrow().as_ref() {
        Some(path) => path.clone(),
        None => { return Err("错误: 未设置目标位置路径（A按钮）".to_string()); }
    };
    Ok((base_path_str, anime_path_str))
}

fn collect_source_files_from_browser(file_browser: &FileBrowser) -> Vec<String> {
    let mut file_names = Vec::new();
    for i in 1..=file_browser.size() { // FileBrowser is 1-indexed
        if file_browser.selected(i) { // Process only selected files if BrowserType::Multi
             if let Some(text) = file_browser.text(i) {
                file_names.push(text.to_string());
            }
        } else if file_browser.get_type::<fltk::browser::BrowserType>() == fltk::browser::BrowserType::Hold && file_browser.value() == i { 
            // For Hold type, value() gives the selected line
             if let Some(text) = file_browser.text(i) {
                file_names.push(text.to_string());
            }
        } else if file_browser.get_type::<fltk::browser::BrowserType>() != fltk::browser::BrowserType::Multi && file_browser.get_type::<fltk::browser::BrowserType>() != fltk::browser::BrowserType::Hold {
            // For Single or Normal, consider all files or the single selected one.
            // This example assumes we want all files if not Multi/Hold with specific selection.
            // For simplicity, if it's not Multi and not Hold, we'll take all.
            // Or, if it's Hold, and no specific line is selected via value(), take all.
            // This part might need refinement based on exact desired behavior for FileBrowser selection.
            // Current `load_files_to_file_browser` adds all video files.
            // Let's assume for "Done", we process all files listed in the browser.
            if let Some(text) = file_browser.text(i) {
                file_names.push(text.to_string());
            }
        }
    }
     if file_names.is_empty() && file_browser.size() > 0 && file_browser.get_type::<fltk::browser::BrowserType>() != fltk::browser::BrowserType::Multi {
        // If no specific selection logic matched for non-multi types, but files exist, take all.
        for i in 1..=file_browser.size() {
            if let Some(text) = file_browser.text(i) {
                file_names.push(text.to_string());
            }
        }
    }
    file_names
}


fn handle_file_browser_events(_browser: &mut FileBrowser, event: Event) -> bool { 
    match event {
        Event::Push => { false },
        Event::KeyDown => { false  },
        Event::Drag => { false },
        Event::Released => { false },
        _ => false, 
    }
}
