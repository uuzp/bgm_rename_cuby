// src/ui.rs
// UI 布局和组装

use fltk::{
    app,
    browser::{FileBrowser, MultiBrowser},
    button::Button,
    enums::{Event, Key},
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

use crate::bangumi_api as api;
use crate::io;
use crate::ui_core;
use crate::CliArgs;
use crate::{MAX_BUTTON_LABEL_LEN, WINDOW_WIDTH, WINDOW_HEIGHT, HALF_WIDTH, MENU_TRIGGER_HEIGHT};

/// 创建主窗口
pub fn create_main_window() -> Window {
    Window::new(
        100,
        100,
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        "BGM Rename Cuby - 番剧文件批量重命名工具",
    )
}

/// 创建UI并返回主窗口
pub fn create_ui(cli_args: &CliArgs) -> Window {
    let mut main_window = create_main_window();
    
    // 初始化UI并获取所有组件和状态
    let (_base_path_rc, _anime_path_rc, _file_browser, _search_results_browser, _controls, _ui_state) = 
        initialize_ui(cli_args, &mut main_window);
        
    // 设置窗口关闭回调
    main_window.set_callback(|_| {
        if app::event() == Event::Close {
            app::quit();
        }
    });
    
    main_window
}

/// 完整初始化UI并返回所有必要的组件和状态
pub fn initialize_ui(cli_args: &crate::CliArgs, wind: &mut Window) -> (
    Rc<RefCell<Option<String>>>, // base_path_rc
    Rc<RefCell<Option<String>>>, // anime_path_rc
    FileBrowser,                // file_browser
    MultiBrowser,               // search_results_browser
    UIControls,                 // 控件集合
    UIState,                    // UI状态
) {
    // 从命令行参数初始化路径
    let (base_path_rc, anime_path_rc) = initialize_paths_from_cli(cli_args);
    
    // 创建核心控件
    let (mut btn_choose_base, mut btn_choose_anime, mut btn_done, mut search_input, mut search_button) = 
        ui_core::create_core_controls();
    let (mut settings_button, mut unregister_button, mut about_button) = 
        ui_core::create_menu_buttons();
    let (mut file_browser, mut search_results_browser) = 
        ui_core::create_main_browsers();
    
    // 构建UI布局
    let (
        mut main_vertical_flex,
        mut menu_trigger_button,
        mut path_trigger_button,
        mut menu_items_panel_flex,
        mut path_display_panel_flex,
    ) = build_ui_layout(
        wind,
        &btn_choose_base, &btn_choose_anime, &btn_done, &search_input, &search_button,
        &settings_button, &unregister_button, &about_button,
        &mut file_browser, &mut search_results_browser,
    );
    
    // 初始化UI状态并应用命令行参数
    let (is_menu_expanded, is_path_panel_expanded, search_results_rc, episode_list_rc, selected_anime_id_rc) =
        initialize_ui_state_and_apply_cli_args(
            &base_path_rc, &anime_path_rc,
            &mut btn_choose_base, &mut btn_choose_anime,
            &mut file_browser, &mut search_input,
        );
    
    // 为回调准备克隆
    let file_browser_clone_for_base_cb = file_browser.clone();
    let search_input_clone_for_base_cb = search_input.clone();
    let search_results_browser_clone_for_search_actions = search_results_browser.clone();
    let file_browser_clone_for_done_cb = file_browser.clone();
    let search_results_browser_clone_for_done_cb = search_results_browser.clone();
    
    // 注册所有回调
    register_all_callbacks(
        wind,
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

    // 创建UI控件集合结构体
    let controls = UIControls {
        main_vertical_flex,
        menu_trigger_button,
        path_trigger_button,
        menu_items_panel_flex,
        path_display_panel_flex,
        btn_choose_base,
        btn_choose_anime,
        btn_done,
        search_input,
        search_button,
        settings_button,
        unregister_button,
        about_button,
    };

    // 创建UI状态结构体
    let ui_state = UIState {
        is_menu_expanded,
        is_path_panel_expanded,
        search_results_rc,
        episode_list_rc,
        selected_anime_id_rc,
    };
    
    (base_path_rc, anime_path_rc, file_browser, search_results_browser, controls, ui_state)
}

/// UI控件集合结构体
pub struct UIControls {
    pub main_vertical_flex: Flex,
    pub menu_trigger_button: Button,
    pub path_trigger_button: Button,
    pub menu_items_panel_flex: Flex,
    pub path_display_panel_flex: Flex,
    pub btn_choose_base: Button,
    pub btn_choose_anime: Button,
    pub btn_done: Button,
    pub search_input: Input,
    pub search_button: Button,
    pub settings_button: Button,
    pub unregister_button: Button,
    pub about_button: Button,
}

/// UI状态结构体
pub struct UIState {
    pub is_menu_expanded: Rc<RefCell<bool>>,
    pub is_path_panel_expanded: Rc<RefCell<bool>>,
    pub search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    pub episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>>,
    pub selected_anime_id_rc: Rc<RefCell<Option<String>>>,
}

/// 从命令行参数初始化路径
pub fn initialize_paths_from_cli(cli_args: &crate::CliArgs) -> (Rc<RefCell<Option<String>>>, Rc<RefCell<Option<String>>>) {
    (
        Rc::new(RefCell::new(cli_args.base_path.clone())),
        Rc::new(RefCell::new(cli_args.anime_path.clone())),
    )
}

/// 构建UI布局
pub fn build_ui_layout(
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

/// 初始化UI状态并应用命令行参数
pub fn initialize_ui_state_and_apply_cli_args(
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
    Rc<RefCell<Option<api::EpisodeCollection>>>,
    Rc<RefCell<Option<String>>>,
) {
    let is_menu_expanded = Rc::new(RefCell::new(false));
    let is_path_panel_expanded = Rc::new(RefCell::new(false));
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

/// 设置顶部触发器布局
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

/// 设置菜单项面板
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

/// 设置路径显示面板
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

/// 设置内容区域
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

/// 注册所有回调
#[allow(clippy::too_many_arguments)]
pub fn register_all_callbacks(
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
    episode_list_rc_for_dblclick: Rc<RefCell<Option<api::EpisodeCollection>>>,
    selected_anime_id_rc_for_dblclick: Rc<RefCell<Option<String>>>,
    file_browser: &mut FileBrowser,
    btn_done: &mut Button,
    file_browser_for_done_cb: FileBrowser,
    episode_list_rc_for_done: Rc<RefCell<Option<api::EpisodeCollection>>>,
    base_path_rc_for_done: Rc<RefCell<Option<String>>>,
    anime_path_rc_for_done: Rc<RefCell<Option<String>>>,
    search_results_rc_for_done: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    selected_anime_id_rc_for_done: Rc<RefCell<Option<String>>>,
    search_results_browser_for_done_cb: MultiBrowser,
) {
    // Menu Toggle Callback
    let is_menu_expanded_cb = is_menu_expanded.clone();
    let mut main_flex_cb_menu = main_vertical_flex.clone();
    let mut menu_panel_cb_menu = menu_items_panel_flex.clone();
    let mut wind_cb_menu = wind.clone();
    menu_trigger_button.set_callback(move |_| {
        ui_core::handle_menu_toggle(
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
        ui_core::handle_path_panel_toggle(
            is_path_panel_expanded_cb.clone(),
            &mut main_flex_cb_path,
            &mut path_panel_cb_path,
            &mut wind_cb_path,
        );
    });

    // Menu Item Callbacks
    settings_button.set_callback(|_| ui_core::handle_register_context_menu());
    unregister_button.set_callback(|_| ui_core::handle_unregister_context_menu());
    about_button.set_callback(|_| ui_core::handle_about_button());

    // Path Choose Callbacks
    let base_path_cb_b = base_path_rc.clone();
    let btn_choose_base_cb_b = btn_choose_base.clone();
    btn_choose_base.set_callback(move |_| {
        ui_core::handle_choose_base_path_callback(
            base_path_cb_b.clone(),
            btn_choose_base_cb_b.clone(),
            file_browser_for_b_cb.clone(),
            search_input_for_b_cb.clone(),
        );
    });

    let anime_path_cb_a = anime_path_rc.clone();
    let btn_choose_anime_cb_a = btn_choose_anime.clone();
    btn_choose_anime.set_callback(move |_| {
        ui_core::handle_choose_anime_path_callback(
            anime_path_cb_a.clone(),
            btn_choose_anime_cb_a.clone(),
        );
    });

    // Search Callbacks
    let search_input_cb_search_btn = search_input_for_search_cb.clone();
    let search_results_cb_search_btn = search_results_rc_for_search.clone();
    let srb_for_search_button_closure = search_results_browser_for_search_actions.clone();
    search_button.set_callback(move |_| {
        ui_core::handle_search_button_callback(
            search_input_cb_search_btn.clone(),
            search_results_cb_search_btn.clone(),
            srb_for_search_button_closure.clone(),
        );
    });    let search_input_cb_enter = search_input_for_search_cb.clone();
    let search_results_cb_enter = search_results_rc_for_search.clone();
    search_input_for_search_cb.handle(move |_, ev| {
        if ev == Event::KeyDown && app::event_key() == Key::Enter {
            return crate::handle_search_input_enter_key(
                search_input_cb_enter.clone(),
                search_results_cb_enter.clone(),
                search_results_browser_for_search_actions.clone(),
            );
        }
        false
    });

    // Search Results Browser Callback
    search_results_browser.set_callback(move |b| {
        ui_core::handle_search_results_double_click(
            b,
            search_results_rc_for_dblclick.clone(),
            episode_list_rc_for_dblclick.clone(),
            selected_anime_id_rc_for_dblclick.clone(),
        );
    });

    // File Browser Callback
    file_browser.handle(move |b, ev| {
        ui_core::handle_file_browser_events(b, ev)
    });

    // Done Button Callback
    btn_done.set_callback(move |_| {
        ui_core::handle_done_button_callback(
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