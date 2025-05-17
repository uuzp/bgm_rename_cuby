#![windows_subsystem = "windows"] // 禁止在 Windows 上显示控制台窗口

// --- 模块导入 ---
mod bangumi_api;
mod io;
mod ui;
mod ui_core;

// --- 使用声明 ---
use fltk::{app, prelude::*, window::Window, group::Flex, button::Button, browser::FileBrowser, input::Input, browser::MultiBrowser, enums::Event, dialog::{self, FileDialog, FileDialogType}, frame::Frame, enums::Color, enums::Key}; // 添加 fltk 组件并修正 dialog
use clap::Parser; // 导入 clap::Parser
use std::rc::Rc; // 添加 Rc
use std::cell::RefCell; // 添加 RefCell

// --- 常量 ---
pub const WINDOW_WIDTH: i32 = 800;
pub const WINDOW_HEIGHT: i32 = 600;
pub const HALF_WIDTH: i32 = WINDOW_WIDTH / 2;
pub const MENU_TRIGGER_HEIGHT: i32 = 30; // 菜单触发器行高度
pub const MENU_ITEMS_PANEL_EXPANDED_HEIGHT: i32 = 35; // 菜单项面板展开高度
pub const PATH_DISPLAY_PANEL_EXPANDED_HEIGHT: i32 = 30; // 路径显示面板展开高度
pub const MAX_BUTTON_LABEL_LEN: usize = 20; // 按钮标签最大显示字符数（粗略）

// --- 命令行参数定义 ---
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// 源文件路径（包含视频文件的文件夹）
    #[arg(short, long)]
    pub base_path: Option<String>,

    /// 目标文件路径（重命名后文件存放的文件夹）
    #[arg(short, long)]
    pub anime_path: Option<String>,
}

// --- 主函数 ---
fn main() {
    // 解析命令行参数
    let cli_args = CliArgs::parse();
    
    // 初始化 FLTK 应用
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    
    // 创建并初始化UI
    let mut main_window = ui::create_ui(&cli_args);
    
    // 显示窗口并运行应用
    main_window.show();
    app.run().unwrap();
}

// --- UI 创建与布局辅助函数 ---

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
    file_browser.set_type(fltk::browser::BrowserType::Hold);
    file_browser.set_damage(true);

    let mut search_results_browser = MultiBrowser::new(0, 0, 0, 0, "");
    search_results_browser.set_tooltip("Bangumi API 搜索结果");
    search_results_browser.set_selection_color(Color::Yellow);
    search_results_browser.set_type(fltk::browser::BrowserType::Hold);
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
    Rc<RefCell<Option<Vec<bangumi_api::BangumiSubject>>>>,
    Rc<RefCell<Option<bangumi_api::EpisodeCollection>>>,  // 修改类型
    Rc<RefCell<Option<String>>> // selected_anime_id_rc (保持 String, 因为API ID可能很大)
) {
    let is_menu_expanded = Rc::new(RefCell::new(false));
    let is_path_panel_expanded = Rc::new(RefCell::new(false));
    // 修改类型
    let search_results_rc: Rc<RefCell<Option<Vec<bangumi_api::BangumiSubject>>>> = Rc::new(RefCell::new(None));
    let episode_list_rc: Rc<RefCell<Option<bangumi_api::EpisodeCollection>>> = Rc::new(RefCell::new(None));
    let selected_anime_id_rc: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    if let Some(cli_base_path_str) = base_path_rc.borrow().as_deref() {
        btn_choose_base.set_label(&io::shorten_path_for_display(cli_base_path_str, MAX_BUTTON_LABEL_LEN));
        io::load_files_to_file_browser(cli_base_path_str, file_browser);
        if let Some(extracted_anime_name) = io::extract_anime_name_from_path(cli_base_path_str) {
            search_input.set_value(&extracted_anime_name);
            println!("从命令行路径 {} 提取到番剧名: {}", cli_base_path_str, extracted_anime_name);
        }
    }
    if let Some(cli_anime_path_str) = anime_path_rc.borrow().as_deref() {
        btn_choose_anime.set_label(&io::shorten_path_for_display(cli_anime_path_str, MAX_BUTTON_LABEL_LEN));
    }

    (is_menu_expanded, is_path_panel_expanded, search_results_rc, episode_list_rc, selected_anime_id_rc)
}

// --- UI 元素布局函数 ---
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


// --- 回调注册 ---
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
    search_results_rc_for_search: Rc<RefCell<Option<Vec<bangumi_api::BangumiSubject>>>>,
    search_results_browser_for_search_actions: MultiBrowser,
    search_results_browser: &mut MultiBrowser,
    search_results_rc_for_dblclick: Rc<RefCell<Option<Vec<bangumi_api::BangumiSubject>>>>,
    episode_list_rc_for_dblclick: Rc<RefCell<Option<bangumi_api::EpisodeCollection>>>, // 修改类型
    selected_anime_id_rc_for_dblclick: Rc<RefCell<Option<String>>>,
    file_browser: &mut FileBrowser, // 修正这里的逗号缺失
    btn_done: &mut Button,
    file_browser_for_done_cb: FileBrowser,
    episode_list_rc_for_done: Rc<RefCell<Option<bangumi_api::EpisodeCollection>>>, // 修改类型
    base_path_rc_for_done: Rc<RefCell<Option<String>>>,
    anime_path_rc_for_done: Rc<RefCell<Option<String>>>,
    search_results_rc_for_done: Rc<RefCell<Option<Vec<bangumi_api::BangumiSubject>>>>, // 修改类型
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


// --- UI 事件处理函数 ---

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
    let mut dialog = FileDialog::new(FileDialogType::BrowseDir);
    dialog.set_title("选择源文件夹 (B)");
    dialog.show();
    let chosen_path_pb = dialog.filename();
    if !chosen_path_pb.as_os_str().is_empty() {
        let path = std::path::Path::new(&chosen_path_pb); // 使用 std::path::Path 明确指定
        if path.is_dir() {
            if let Some(path_str) = path.to_str() {
                *base_path_rc.borrow_mut() = Some(path_str.to_string());
                btn_choose_base.set_label(&io::shorten_path_for_display(path_str, MAX_BUTTON_LABEL_LEN));
                io::load_files_to_file_browser(path_str, &mut file_browser);
                if let Some(extracted_anime_name) = io::extract_anime_name_from_path(path_str) {
                    search_input.set_value(&extracted_anime_name);
                    println!("从路径 {} 提取到番剧名: {}", path_str, extracted_anime_name);
                }
            }
        }
    }
}

fn handle_choose_anime_path_callback(anime_path_rc: Rc<RefCell<Option<String>>>, mut btn_choose_anime: Button) {
    let mut dialog = FileDialog::new(FileDialogType::BrowseDir);
    dialog.set_title("选择目标文件夹 (A)");
    dialog.show();
    let chosen_path = dialog.filename();
    if !chosen_path.as_os_str().is_empty() {
        let path = std::path::Path::new(&chosen_path); // 使用 std::path::Path 明确指定
        if path.is_dir() {
            if let Some(path_str) = path.to_str() {
                *anime_path_rc.borrow_mut() = Some(path_str.to_string());
                btn_choose_anime.set_label(&io::shorten_path_for_display(path_str, MAX_BUTTON_LABEL_LEN));
            }
        }
    }
}

fn handle_file_browser_events(browser: &mut FileBrowser, event: Event) -> bool {
    static mut DRAG_ITEM: i32 = -1;
    static mut HIGHLIGHTED_LINE: i32 = -1;
    
    match event {
        Event::Push => {
            if app::event_clicks() { // 双击事件
                unsafe { 
                    HIGHLIGHTED_LINE = browser.value();
                    println!("高亮行: {}", HIGHLIGHTED_LINE);
                }
                return true;
            }
            false
        },        Event::KeyDown => {
            let key = app::event_key();
            if key == fltk::enums::Key::from_char(' ') {
                let current_line = browser.value();
                unsafe {
                    if HIGHLIGHTED_LINE == -1 {
                        // 第一次按空格，设置高亮行
                        HIGHLIGHTED_LINE = current_line;
                        println!("高亮行: {}", HIGHLIGHTED_LINE);
                        browser.select(current_line); // 高亮选中该行
                        return true;
                    } else if current_line != HIGHLIGHTED_LINE && current_line > 0 {
                        // 第二次按空格，交换行
                        println!("交换行: {} 与 {}", HIGHLIGHTED_LINE, current_line);
                        
                        // 获取两行的文本
                        let text1 = browser.text(HIGHLIGHTED_LINE).unwrap_or_default().to_string();
                        let text2 = browser.text(current_line).unwrap_or_default().to_string();
                        
                        // 交换内容
                        browser.set_text(HIGHLIGHTED_LINE, &text2);
                        browser.set_text(current_line, &text1);
                          // 重置高亮行
                        HIGHLIGHTED_LINE = -1;
                        browser.deselect(current_line);
                        browser.redraw();
                        return true;
                    } else {
                        // 取消高亮
                        HIGHLIGHTED_LINE = -1;
                        browser.deselect(current_line);
                        return true;
                    }
                }
            }
            false
        },
        Event::Drag => {
            unsafe {
                let y = app::event_y();
                let item = browser.value();
                
                if DRAG_ITEM < 0 {
                    DRAG_ITEM = item; // 记录开始拖拽的项
                    return true;
                }
                  // 计算当前移动到哪一行
                // 由于没有直接的方法，使用当前选中行
                let new_item = browser.value();
                
                if new_item > 0 && new_item != DRAG_ITEM {
                    // 获取两行的文本
                    let text1 = browser.text(DRAG_ITEM).unwrap_or_default().to_string();
                    let text2 = browser.text(new_item).unwrap_or_default().to_string();
                    
                    // 交换内容
                    browser.set_text(DRAG_ITEM, &text2);
                    browser.set_text(new_item, &text1);
                      DRAG_ITEM = new_item; // 更新拖拽的项
                    browser.select(new_item); // 更新选中状态
                    browser.redraw();
                    return true;
                }
            }
            true
        },
        Event::Released => {
            unsafe {
                DRAG_ITEM = -1; // 重置拖拽项
            }
            true
        },
        _ => false,
    }
}

// --- 菜单项事件处理函数 ---
fn handle_register_context_menu() {
    io::register_context_menu();
}

fn handle_unregister_context_menu() {
    io::unregister_context_menu();
}

fn handle_about_button() {
    let repo_url = "https://github.com/uuzp/bgm_rename_cuby";
    if webbrowser::open(repo_url).is_err() {
        dialog::message_default(&format!("无法打开链接: {}", repo_url));
        println!("Error opening URL: {}", repo_url);
    }
}

// --- 核心逻辑回调处理函数 ---

/// 处理搜索按钮点击的回调
fn handle_search_button_callback(
    search_input: Input,
    search_results_rc: Rc<RefCell<Option<Vec<bangumi_api::BangumiSubject>>>>,
    mut search_results_browser: MultiBrowser,
) {
    let query = search_input.value();
    if !query.is_empty() {
        println!("正在搜索: {}", query);
        match <Vec<bangumi_api::BangumiSubject> as bangumi_api::ResourceFetcher<&str>>::fetch(&query) {
            Ok(subjects) => {
                if subjects.is_empty() {
                    println!("未找到番剧: {}", query);
                    search_results_browser.clear();
                    *search_results_rc.borrow_mut() = None;
                    dialog::message_default(&format!("未找到与“{}”相关的番剧。", query));
                } else {
                    println!("找到 {} 个番剧。", subjects.len());
                    search_results_browser.clear();
                    for subject in &subjects {
                        search_results_browser.add(&format!("{} ({})", subject.name_cn, subject.name));
                    }
                    *search_results_rc.borrow_mut() = Some(subjects);
                }
            }
            Err(err_msg) => {
                println!("搜索 '{}' 失败: {}", query, err_msg);
                search_results_browser.clear();
                *search_results_rc.borrow_mut() = None;
                dialog::message_default(&format!("搜索“{}”失败：请检查网络连接或稍后再试。\n详细错误: {}", query, err_msg));
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
    search_results_rc: Rc<RefCell<Option<Vec<bangumi_api::BangumiSubject>>>>,
    mut search_results_browser: MultiBrowser,
) -> bool {
    let query = search_input.value();
    if !query.is_empty() {
        println!("通过回车搜索: {}", query);
        match <Vec<bangumi_api::BangumiSubject> as bangumi_api::ResourceFetcher<&str>>::fetch(&query) {
            Ok(subjects) => {
                if subjects.is_empty() {
                    println!("未找到番剧: {}", query);
                    search_results_browser.clear();
                    *search_results_rc.borrow_mut() = None;
                    dialog::message_default(&format!("未找到与“{}”相关的番剧。", query));
                } else {
                    println!("找到 {} 个番剧。", subjects.len());
                    search_results_browser.clear();
                    for subject in &subjects {
                        search_results_browser.add(&format!("{} ({})", subject.name_cn, subject.name));
                    }
                    *search_results_rc.borrow_mut() = Some(subjects);
                }
            }
            Err(err_msg) => {
                println!("搜索 '{}' 失败: {}", query, err_msg);
                search_results_browser.clear();
                *search_results_rc.borrow_mut() = None;
                dialog::message_default(&format!("搜索“{}”失败：请检查网络连接或稍后再试。\n详细错误: {}", query, err_msg));
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
    search_results_rc: Rc<RefCell<Option<Vec<bangumi_api::BangumiSubject>>>>,
    episode_list_rc: Rc<RefCell<Option<bangumi_api::EpisodeCollection>>>,    // 修改类型
    selected_anime_id_rc: Rc<RefCell<Option<String>>>,
) {
    if app::event_clicks() {
        let line = browser.value();
        if line > 0 && line <= browser.size() {
            if let Some(subjects) = &*search_results_rc.borrow() {
                let idx = (line as usize) - 1;
                if idx < subjects.len() {
                    let subject_id = subjects[idx].id;
                    println!("双击选中番剧ID: {}", subject_id);

                    match <bangumi_api::EpisodeCollection as bangumi_api::ResourceFetcher<u64>>::fetch(subject_id) {
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
    episode_list_rc: &Rc<RefCell<Option<bangumi_api::EpisodeCollection>>>
) -> Result<bangumi_api::EpisodeCollection, String> {
    match episode_list_rc.borrow().as_ref() {
        Some(ep_data) => Ok(ep_data.clone()),
        None => {
            let err_msg = "剧集列表为空，无法执行操作。请先在右侧搜索并双击选定一部番剧。";
            println!("{}", err_msg);
            Err(err_msg.to_string())
        }
    }
}

/// "完成" 按钮回调处理函数
#[allow(clippy::too_many_arguments)]
fn handle_done_button_callback(
    file_browser: FileBrowser,
    episode_list_rc: Rc<RefCell<Option<bangumi_api::EpisodeCollection>>>,
    base_path_rc: Rc<RefCell<Option<String>>>,
    anime_path_rc: Rc<RefCell<Option<String>>>,
    search_results_rc: Rc<RefCell<Option<Vec<bangumi_api::BangumiSubject>>>>,
    _selected_anime_id_rc: Rc<RefCell<Option<String>>>, // ID is used for fetching, name/year from elsewhere
    search_results_browser: MultiBrowser,
) {
    println!("Done button clicked!");

    if episode_list_rc.borrow().is_none() {
        dialog::message_default("错误：剧集列表为空。
请先在右侧搜索并双击选定一部番剧。");
        return;
    }
    if base_path_rc.borrow().is_none() {
        dialog::message_default("错误：源路径（B按钮）未设定。");
        return;
    }
    if anime_path_rc.borrow().is_none() {
        dialog::message_default("错误：目标路径（A按钮）未设定。");
        return;
    }

    match io::validate_operation_paths(&base_path_rc, &anime_path_rc) {
        Ok((base_path_str, anime_path_root_str)) => {
            println!("Base path: {}, Anime path root: {}", base_path_str, anime_path_root_str);
            let source_files = io::collect_source_files_from_browser(&file_browser);

            if source_files.is_empty() {
                dialog::message_default("错误：未在左侧文件列表中选择任何文件进行处理。");
                return;
            }
            println!("Source files selected: {:?}", source_files);

            if let Ok(ep_collection) = get_episode_data_for_processing(&episode_list_rc) {
                let year = ep_collection.year; // 读取 year 字段

                // 获取选定的番剧名称
                let anime_display_name = {
                    let search_results_opt = search_results_rc.borrow();
                    let selected_idx_in_browser = search_results_browser.value(); // 1-indexed

                    if selected_idx_in_browser > 0 {
                        if let Some(subjects) = search_results_opt.as_ref() {
                            let actual_idx = (selected_idx_in_browser as usize) - 1;
                            if actual_idx < subjects.len() {
                                let selected_subject = &subjects[actual_idx];
                                if !selected_subject.name_cn.is_empty() {
                                    selected_subject.name_cn.clone()
                                } else {
                                    selected_subject.name.clone()
                                }
                            } else {
                                dialog::message_default("错误：无法获取选定的番剧名称（列表索引越界）。
请重新搜索并选择番剧。");
                                return;
                            }
                        } else {
                            dialog::message_default("错误：无法获取选定的番剧名称（无搜索结果缓存）。
请重新搜索并选择番剧。");
                            return;
                        }
                    } else {
                        dialog::message_default("错误：未在右侧搜索结果中选定番剧。
请先搜索并双击选定一部番剧。");
                        return;
                    }
                };
                
                let cleaned_anime_name_for_folder = io::replace_invalid_chars(&anime_display_name);
                let target_anime_folder_name = format!("{}({})", cleaned_anime_name_for_folder, year); // 使用 year 字段
                let target_anime_dir = std::path::Path::new(&anime_path_root_str).join(target_anime_folder_name);

                println!("计划创建/使用的目标番剧文件夹: {:?}", target_anime_dir);

                if let Err(e) = std::fs::create_dir_all(&target_anime_dir) {
                    dialog::message_default(&format!("错误：创建目标文件夹失败：{}
路径：{:?}", e, target_anime_dir));
                    return;
                }

                let formatted_episode_names = ep_collection.get_formatted_names();
                
                if source_files.len() > formatted_episode_names.len() {
                     let msg = format!(
                        "警告：选中的文件数量 ({}) 大于获取到的剧集数量 ({}).
多余的文件将不会被重命名。
是否继续?",
                        source_files.len(),
                        formatted_episode_names.len()
                    );
                    if dialog::choice2_default(&msg, "继续", "取消", "") != Some(0) {
                        return;
                    }
                }


                let mut successful_renames = 0;
                let mut failed_renames = 0;
                let mut errors_log = Vec::new();

                for (i, source_file_name_str) in source_files.iter().enumerate() {
                    if i >= formatted_episode_names.len() {
                        let err_msg = format!("跳过文件 '{}': 没有对应的剧集名称 (选中 {} 个文件, 获取到 {} 个剧集名)。", source_file_name_str, source_files.len(), formatted_episode_names.len());
                        println!("{}", err_msg);
                        errors_log.push(err_msg);
                        // failed_renames += 1; // Not a failure of rename, but a skip.
                        continue;
                    }

                    let source_file_path = std::path::Path::new(&base_path_str).join(source_file_name_str);
                    let original_extension = source_file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    
                    let cleaned_episode_name_part = io::replace_invalid_chars(&formatted_episode_names[i]);
                    let new_file_name_str = if original_extension.is_empty() {
                        cleaned_episode_name_part.clone()
                    } else {
                        format!("{}.{}", cleaned_episode_name_part, original_extension)
                    };
                    
                    let target_file_path = target_anime_dir.join(&new_file_name_str);

                    println!("准备重命名: {:?} -> {:?}", source_file_path, target_file_path);

                    if source_file_path == target_file_path {
                        let msg = format!("跳过文件 '{}': 源路径和目标路径相同。", source_file_name_str);
                        println!("{}", msg);
                        errors_log.push(msg);
                        continue;
                    }
                    if target_file_path.exists() {
                        let msg = format!("跳过文件 '{}': 目标文件 '{}' 已存在。", source_file_name_str, target_file_path.display());
                         println!("{}", msg);
                        errors_log.push(msg);
                        failed_renames +=1;
                        continue;
                    }


                    if let Err(e) = std::fs::rename(&source_file_path, &target_file_path) {
                        let err_msg = format!("重命名 '{}' 失败: {}", source_file_name_str, e);
                        println!("{}", err_msg);
                        errors_log.push(err_msg);
                        failed_renames += 1;
                    } else {
                        successful_renames += 1;
                    }
                }
                
                // Reload files in base_path_str to reflect changes
                io::load_files_to_file_browser(&base_path_str, &mut file_browser.clone());


                let mut summary_message = format!("重命名操作完成。
成功: {}
失败: {}", successful_renames, failed_renames);
                if !errors_log.is_empty() {
                    summary_message.push_str("

日志详情:
");
                    summary_message.push_str(&errors_log.join("
"));
                }
                dialog::message_default(&summary_message);

            } else {
                 dialog::message_default("错误：无法获取剧集数据进行处理。
请重新搜索并双击选定一部番剧。");
            }
        }
        Err(e) => {
            dialog::message_default(&e);
        }
    }
}
