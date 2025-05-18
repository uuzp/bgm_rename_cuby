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
use crate::{
    MAX_BUTTON_LABEL_LEN, WINDOW_WIDTH, WINDOW_HEIGHT, HALF_WIDTH, MENU_TRIGGER_HEIGHT,
    MENU_ITEMS_PANEL_EXPANDED_HEIGHT, PATH_DISPLAY_PANEL_EXPANDED_HEIGHT, // 添加导入
};

/// UI控件集合结构体
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct UIState {
    pub is_menu_expanded: Rc<RefCell<bool>>,
    pub is_path_panel_expanded: Rc<RefCell<bool>>,
    pub search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    pub episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>>,

    pub selected_anime_id_rc: Rc<RefCell<Option<String>>>,
}

/// 创建主窗口
fn init_window() -> Window {
    Window::new(
        100,
        100,
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        "Cuby",
    )
}

/// UI组件集合结构体，封装从initialize_ui函数返回的所有组件
pub struct UIComponents {
    pub base_path_rc: Rc<RefCell<Option<String>>>, // base_path_rc
    pub anime_path_rc: Rc<RefCell<Option<String>>>, // anime_path_rc
    pub file_browser: FileBrowser,                // file_browser
    pub search_results_browser: MultiBrowser,               // search_results_browser
    pub controls: UIControls,                 // 控件集合
    pub state: UIState,                    // UI状态
}

/// 创建UI并返回主窗口
pub fn init_ui(cli_args: &CliArgs) -> Window {
    let mut main_window = init_window();
    
    // 初始化UI并获取所有组件和状态
    let _ui_components = initialize_ui(cli_args, &mut main_window);
        
    // 设置窗口关闭回调
    main_window.set_callback(|_| {
        if app::event() == Event::Close {
            app::quit();
        }
    });
    
    main_window
}

/// 从命令行参数初始化路径
pub fn initialize_paths_from_cli(cli_args: &crate::CliArgs) -> (Rc<RefCell<Option<String>>>, Rc<RefCell<Option<String>>>) {
    (
        Rc::new(RefCell::new(cli_args.base_path.clone())),
        Rc::new(RefCell::new(cli_args.anime_path.clone())),
    )
}

/// 完整初始化UI并返回所有必要的组件和状态
fn initialize_ui(cli_args: &crate::CliArgs, wind: &mut Window) -> UIComponents {
    // 从命令行参数初始化路径
    let (base_path_rc, anime_path_rc) = initialize_paths_from_cli(cli_args);
    
    // 创建所有UI控件
    let (mut controls, mut file_browser, mut search_results_browser) = create_ui_controls(wind);
    
    // 初始化UI状态并应用命令行参数
    let ui_state = initialize_ui_state(
        &base_path_rc, 
        &anime_path_rc, 
        &mut controls.btn_choose_base, 
        &mut controls.btn_choose_anime,
        &mut file_browser, 
        &mut controls.search_input
    );
    
    // 设置所有回调
    register_callbacks(
        wind,
        &mut controls, // 传递可变引用
        &ui_state,
        &base_path_rc, 
        &anime_path_rc,
        &mut file_browser, // 传递可变引用
        &mut search_results_browser // 传递可变引用
    );
    
    // 返回组合到一起的UI组件
    UIComponents {
        base_path_rc,
        anime_path_rc,
        file_browser,
        search_results_browser,
        controls,
        state: ui_state,
    }
}
/// 注册所有回调的总入口函数
fn register_callbacks(
    wind: &mut Window,
    controls: &mut UIControls,
    ui_state: &UIState,
    base_path_rc: &Rc<RefCell<Option<String>>>,
    anime_path_rc: &Rc<RefCell<Option<String>>>,
    file_browser: &mut FileBrowser,
    search_results_browser: &mut MultiBrowser,
) {
    register_toggle_callbacks(
        wind,
        &mut controls.main_vertical_flex,
        &mut controls.menu_trigger_button,
        ui_state.is_menu_expanded.clone(),
        &mut controls.menu_items_panel_flex,
        &mut controls.path_trigger_button,
        ui_state.is_path_panel_expanded.clone(),
        &mut controls.path_display_panel_flex,
    );

    register_menu_item_callbacks(
        &mut controls.settings_button,
        &mut controls.unregister_button,
        &mut controls.about_button,
    );

    register_path_selection_callbacks(
        &mut controls.btn_choose_base,
        base_path_rc,
        file_browser, // 传递原始 FileBrowser 的引用
        &controls.search_input, // 传递原始 Input 的引用
        &mut controls.btn_choose_anime,
        anime_path_rc,
    );

    register_search_callbacks(
        &mut controls.search_input,
        &mut controls.search_button,
        &ui_state.search_results_rc,
        search_results_browser, // 传递原始 MultiBrowser 的 引用
    );

    register_browser_event_callbacks(
        file_browser,
        search_results_browser,
        &ui_state.search_results_rc,
        &ui_state.episode_list_rc,
        &ui_state.selected_anime_id_rc,
    );

    register_done_button_callback(
        &mut controls.btn_done,
        file_browser, // 传递原始 FileBrowser 的 引用
        &ui_state.episode_list_rc,
        base_path_rc,
        anime_path_rc,
        &ui_state.search_results_rc,
        &ui_state.selected_anime_id_rc,
        search_results_browser, // 传递原始 MultiBrowser 的 引用
    );
}
/// 创建所有UI控件
fn create_ui_controls(wind: &mut Window) -> (UIControls, FileBrowser, MultiBrowser) {
    // 创建核心控件
    let (btn_choose_base,btn_choose_anime,btn_done,search_input,search_button) = 
        ui_core::create_core_controls();
    let (settings_button,unregister_button,about_button) = 
        ui_core::create_menu_buttons();
    let (mut file_browser, mut search_results_browser) = 
        ui_core::create_main_browsers();
    
    // 构建UI布局
    let (
        main_vertical_flex,
        menu_trigger_button,
        path_trigger_button,
        menu_items_panel_flex,
        path_display_panel_flex,
    ) = build_ui_layout(
        wind,
        &btn_choose_base, &btn_choose_anime, &btn_done, &search_input, &search_button,
        &settings_button, &unregister_button, &about_button,
        &mut file_browser, &mut search_results_browser,
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
    
    (controls, file_browser, search_results_browser)
}

/// 初始化UI状态并应用命令行参数
fn initialize_ui_state(
    base_path_rc: &Rc<RefCell<Option<String>>>,
    anime_path_rc: &Rc<RefCell<Option<String>>>,
    btn_choose_base: &mut Button,
    btn_choose_anime: &mut Button,
    file_browser: &mut FileBrowser,
    search_input: &mut Input,
) -> UIState {
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

    // 创建UI状态结构体
    UIState {
        is_menu_expanded,
        is_path_panel_expanded,
        search_results_rc,
        episode_list_rc,
        selected_anime_id_rc,
    }
}

/// 构建UI主布局
#[allow(clippy::too_many_arguments)]
fn build_ui_layout(
    wind: &mut Window,
    btn_choose_base: &Button, 
    btn_choose_anime: &Button, 
    btn_done: &Button, 
    search_input: &Input, 
    search_button: &Button,
    settings_button: &Button, 
    unregister_button: &Button, 
    about_button: &Button,
    file_browser: &mut FileBrowser, 
    search_results_browser: &mut MultiBrowser,
) -> (Flex, Button, Button, Flex, Flex) {
    // 创建主垂直Flex布局
    let mut main_vertical_flex = Flex::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, "");
    main_vertical_flex.set_type(fltk::group::FlexType::Column);
    
    // 设置顶部触发器布局
    let (menu_trigger_button, path_trigger_button, top_triggers_flex) = 
        setup_top_triggers_flex(btn_done, search_input, search_button);
    main_vertical_flex.add(&top_triggers_flex);
    main_vertical_flex.fixed(&top_triggers_flex, MENU_TRIGGER_HEIGHT);
    
    // 设置菜单项面板
    let menu_items_panel_flex = setup_menu_items_panel(settings_button, unregister_button, about_button);
    main_vertical_flex.add(&menu_items_panel_flex);
    main_vertical_flex.fixed(&menu_items_panel_flex, 35);
    
    // 设置路径显示面板
    let path_display_panel_flex = setup_path_display_panel(btn_choose_base, btn_choose_anime);
    main_vertical_flex.add(&path_display_panel_flex);
    main_vertical_flex.fixed(&path_display_panel_flex, 35);
    
    // 设置内容区域
    let (content_flex, mut left_flex, mut right_flex) = setup_content_area();
    
    // 添加浏览器到左右布局
    left_flex.add(file_browser);
    right_flex.add(search_results_browser);
    
    main_vertical_flex.add(&content_flex);
    
    main_vertical_flex.end();
    wind.resizable(&main_vertical_flex); // 使窗口可调整大小，并让 main_vertical_flex 填充
    wind.end();
    
    (
        main_vertical_flex,
        menu_trigger_button,
        path_trigger_button,
        menu_items_panel_flex,
        path_display_panel_flex,
    )
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

// --- 回调注册辅助函数 ---

/// 注册菜单和路径面板的切换回调
fn register_toggle_callbacks(
    wind: &mut Window,
    main_vertical_flex: &mut Flex,
    menu_trigger_button: &mut Button,
    is_menu_expanded: Rc<RefCell<bool>>,
    menu_items_panel_flex: &mut Flex,
    path_trigger_button: &mut Button,
    is_path_panel_expanded: Rc<RefCell<bool>>,
    path_display_panel_flex: &mut Flex,
) {
    // 菜单切换回调
    let is_menu_expanded_cb = is_menu_expanded.clone();
    let mut main_flex_cb_menu = main_vertical_flex.clone();
    let mut menu_panel_cb_menu = menu_items_panel_flex.clone();
    let mut wind_cb_menu = wind.clone();
    menu_trigger_button.set_callback(move |_| {
        ui_core::handle_panel_toggle( // 调用新的通用函数
            is_menu_expanded_cb.clone(),
            &mut main_flex_cb_menu,
            &mut menu_panel_cb_menu,
            &mut wind_cb_menu,
            MENU_ITEMS_PANEL_EXPANDED_HEIGHT, // 传递菜单展开高度
        );
    });

    // 路径面板切换回调
    let is_path_panel_expanded_cb = is_path_panel_expanded.clone();
    let mut main_flex_cb_path = main_vertical_flex.clone();
    let mut path_panel_cb_path = path_display_panel_flex.clone();
    let mut wind_cb_path = wind.clone();
    path_trigger_button.set_callback(move |_| {
        ui_core::handle_panel_toggle( // 调用新的通用函数
            is_path_panel_expanded_cb.clone(),
            &mut main_flex_cb_path,
            &mut path_panel_cb_path,
            &mut wind_cb_path,
            PATH_DISPLAY_PANEL_EXPANDED_HEIGHT, // 传递路径面板展开高度
        );
    });
}

/// 注册菜单项按钮的回调
fn register_menu_item_callbacks(
    settings_button: &mut Button,
    unregister_button: &mut Button,
    about_button: &mut Button,
) {
    settings_button.set_callback(|_| ui_core::handle_register_context_menu());
    unregister_button.set_callback(|_| ui_core::handle_unregister_context_menu());
    about_button.set_callback(|_| ui_core::handle_about_button());
}

/// 注册路径选择按钮的回调
fn register_path_selection_callbacks(
    btn_choose_base: &mut Button,
    base_path_rc: &Rc<RefCell<Option<String>>>,
    file_browser: &FileBrowser, // 接收引用，在闭包中克隆
    search_input: &Input,       // 接收引用，在闭包中克隆
    btn_choose_anime: &mut Button,
    anime_path_rc: &Rc<RefCell<Option<String>>>,
) {
    // "选择基本路径" 按钮回调
    let base_path_rc_clone = base_path_rc.clone();
    let btn_choose_base_clone = btn_choose_base.clone();
    let file_browser_clone = file_browser.clone();
    let search_input_clone = search_input.clone();
    btn_choose_base.set_callback(move |_| {
        ui_core::handle_choose_base_path_callback(
            base_path_rc_clone.clone(),
            btn_choose_base_clone.clone(),
            file_browser_clone.clone(),
            search_input_clone.clone(),
        );
    });

    // "选择番剧路径" 按钮回调
    let anime_path_rc_clone = anime_path_rc.clone();
    let btn_choose_anime_clone = btn_choose_anime.clone();
    btn_choose_anime.set_callback(move |_| {
        ui_core::handle_choose_anime_path_callback(
            anime_path_rc_clone.clone(),
            btn_choose_anime_clone.clone(),
        );
    });
}

/// 注册搜索相关控件的回调
fn register_search_callbacks(
    search_input: &mut Input,
    search_button: &mut Button,
    search_results_rc: &Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    search_results_browser: &MultiBrowser, // 接收引用，在闭包中克隆
) {
    // 搜索按钮回调
    let search_input_clone_btn = search_input.clone();
    let search_results_rc_clone_btn = search_results_rc.clone();
    let srb_clone_btn = search_results_browser.clone();
    search_button.set_callback(move |_| {
        ui_core::handle_search_button_callback(
            search_input_clone_btn.clone(),
            search_results_rc_clone_btn.clone(),
            srb_clone_btn.clone(),
        );
    });

    // 搜索输入框回车键回调
    let search_input_clone_enter = search_input.clone();
    let search_results_rc_clone_enter = search_results_rc.clone();
    let srb_clone_enter = search_results_browser.clone();
    search_input.handle(move |_, ev| {
        if ev == Event::KeyDown && app::event_key() == Key::Enter {
            return ui_core::handle_search_input_enter_key(
                search_input_clone_enter.clone(),
                search_results_rc_clone_enter.clone(),
                srb_clone_enter.clone(),
            );
        }
        false
    });
}

/// 注册文件浏览器和搜索结果浏览器的事件回调
fn register_browser_event_callbacks(
    file_browser: &mut FileBrowser,
    search_results_browser: &mut MultiBrowser,
    search_results_rc: &Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    episode_list_rc: &Rc<RefCell<Option<api::EpisodeCollection>>>,
    selected_anime_id_rc: &Rc<RefCell<Option<String>>>,
) {
    // 搜索结果浏览器双击回调
    let search_results_rc_clone = search_results_rc.clone();
    let episode_list_rc_clone = episode_list_rc.clone();
    let selected_anime_id_rc_clone = selected_anime_id_rc.clone();
    search_results_browser.set_callback(move |b| {
        ui_core::handle_search_results_double_click(
            b,
            search_results_rc_clone.clone(),
            episode_list_rc_clone.clone(),
            selected_anime_id_rc_clone.clone(),
        );
    });

    // 文件浏览器事件回调
    file_browser.handle(move |b, ev| {
        ui_core::handle_file_browser_events(b, ev)
    });
}

/// 注册 "完成" 按钮的回调
fn register_done_button_callback(
    btn_done: &mut Button,
    file_browser: &FileBrowser, // 接收引用
    episode_list_rc: &Rc<RefCell<Option<api::EpisodeCollection>>>,
    base_path_rc: &Rc<RefCell<Option<String>>>,
    anime_path_rc: &Rc<RefCell<Option<String>>>,
    search_results_rc: &Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    selected_anime_id_rc: &Rc<RefCell<Option<String>>>,
    search_results_browser: &MultiBrowser, // 接收引用
) {
    let fb_clone = file_browser.clone();
    let el_rc_clone = episode_list_rc.clone();
    let bp_rc_clone = base_path_rc.clone();
    let ap_rc_clone = anime_path_rc.clone();
    let sr_rc_clone = search_results_rc.clone();
    let said_rc_clone = selected_anime_id_rc.clone();
    let srb_clone = search_results_browser.clone();

    btn_done.set_callback(move |_| {
        ui_core::handle_done_button_callback(
            fb_clone.clone(),
            el_rc_clone.clone(),
            bp_rc_clone.clone(),
            ap_rc_clone.clone(),
            sr_rc_clone.clone(),
            said_rc_clone.clone(),
            srb_clone.clone(),
        );
    });
}