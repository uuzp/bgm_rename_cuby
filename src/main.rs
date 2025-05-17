#![windows_subsystem = "windows"] // 禁止在 Windows 上显示控制台窗口

// --- 模块导入 ---
mod bangumi_api;
mod io;

// --- 使用声明 ---
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
    rc::Rc,
};

use clap::Parser; // 导入 clap::Parser
use webbrowser;

// --- 别名 ---
use bangumi_api as api; // API 操作使用别名 api
// io 模块直接使用 io::

// --- 常量 ---
const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const HALF_WIDTH: i32 = WINDOW_WIDTH / 2;
const MENU_TRIGGER_HEIGHT: i32 = 30; // 菜单触发器行高度
const MENU_ITEMS_PANEL_EXPANDED_HEIGHT: i32 = 35; // 菜单项面板展开高度
const PATH_DISPLAY_PANEL_EXPANDED_HEIGHT: i32 = 30; // 路径显示面板展开高度
const MAX_BUTTON_LABEL_LEN: usize = 20; // 按钮标签最大显示字符数（粗略）


// --- 命令行参数定义 ---
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

// --- 主函数 ---
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
    Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    Rc<RefCell<Option<api::EpisodeCollection>>>,  // 修改类型
    Rc<RefCell<Option<String>>>, // selected_anime_id_rc (保持 String, 因为API ID可能很大)
) {
    let is_menu_expanded = Rc::new(RefCell::new(false));
    let is_path_panel_expanded = Rc::new(RefCell::new(false));
    // 修改类型
    let search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>> = Rc::new(RefCell::new(None));
    let episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>> = Rc::new(RefCell::new(None));
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
    search_results_rc_for_search: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    search_results_browser_for_search_actions: MultiBrowser,
    search_results_browser: &mut MultiBrowser,
    search_results_rc_for_dblclick: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    episode_list_rc_for_dblclick: Rc<RefCell<Option<api::EpisodeCollection>>>, // 修改类型
    selected_anime_id_rc_for_dblclick: Rc<RefCell<Option<String>>>,
    file_browser: &mut FileBrowser,
    btn_done: &mut Button,
    file_browser_for_done_cb: FileBrowser,
    episode_list_rc_for_done: Rc<RefCell<Option<api::EpisodeCollection>>>, // 修改类型
    base_path_rc_for_done: Rc<RefCell<Option<String>>>,
    anime_path_rc_for_done: Rc<RefCell<Option<String>>>,
    search_results_rc_for_done: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>, // 修改类型
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
    let mut dialog = dialog::FileDialog::new(dialog::FileDialogType::BrowseDir);
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
    let mut dialog = dialog::FileDialog::new(dialog::FileDialogType::BrowseDir);
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

fn handle_file_browser_events(_browser: &mut FileBrowser, event: Event) -> bool {
    match event {
        Event::Push => { false },
        Event::KeyDown => { false  },
        Event::Drag => { false },
        Event::Released => { false },
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
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    mut search_results_browser: MultiBrowser,
) {
    let query = search_input.value();
    if !query.is_empty() {
        println!("正在搜索: {}", query);
        match <Vec<api::BangumiSubject> as api::ResourceFetcher<&str>>::fetch(&query) {
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
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    mut search_results_browser: MultiBrowser,
) -> bool {
    let query = search_input.value();
    if !query.is_empty() {
        println!("通过回车搜索: {}", query);
        match <Vec<api::BangumiSubject> as api::ResourceFetcher<&str>>::fetch(&query) {
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
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>>,    // 修改类型
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

                    match <api::EpisodeCollection as api::ResourceFetcher<u64>>::fetch(subject_id) {
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
    episode_list_rc: &Rc<RefCell<Option<api::EpisodeCollection>>>
) -> Result<api::EpisodeCollection, String> {
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
    episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>>,
    base_path_rc: Rc<RefCell<Option<String>>>,
    anime_path_rc: Rc<RefCell<Option<String>>>,
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
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

    // 调用 io 模块的函数进行文件操作的示例（此处仅为示意，具体实现根据需求）
    match io::validate_operation_paths(&base_path_rc, &anime_path_rc) {
        Ok((base_path, anime_path)) => {
            println!("Base path: {}, Anime path: {}", base_path, anime_path);
            let source_files = io::collect_source_files_from_browser(&file_browser);
            println!("Source files: {:?}", source_files);

            if let Ok(ep_collection) = get_episode_data_for_processing(&episode_list_rc) {
                let formatted_names = ep_collection.get_formatted_names();
                let mut cleaned_formatted_names = Vec::new();
                for name in formatted_names {
                    // 在这里调用 io::replace_invalid_chars
                    cleaned_formatted_names.push(io::replace_invalid_chars(&name));
                }
                println!("Formatted and cleaned episode names: {:?}", cleaned_formatted_names);
                // 此处可以继续实现重命名逻辑
                dialog::message_default("重命名功能尚未实现。
请在控制台查看选定项和待处理文件名的调试信息。");

            } else {
                 dialog::message_default("错误：无法获取剧集数据进行处理。");
            }
        }
        Err(e) => {
            dialog::message_default(&e);
        }
    }
}
