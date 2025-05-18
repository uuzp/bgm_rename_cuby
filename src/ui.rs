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
    menu::{SysMenuBar, MenuFlag},
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
    WINDOW_WIDTH, WINDOW_HEIGHT, THREE_FIFTHS_WIDTH, TWO_FIFTHS_WIDTH  // 使用新的宽度常量
};

/// UI控件集合结构体
#[allow(dead_code)]
pub struct UIControls {
    pub main_vertical_flex: Flex,
    pub btn_choose_base: Button,
    pub btn_choose_anime: Button,
    pub btn_done: Button,
    pub search_input: Input,
    pub search_button: Button,
    pub sys_menu_bar: SysMenuBar, // 添加sys_menu_bar替代原菜单按钮
    pub info_frame: Frame,
}

/// UI状态结构体
#[allow(dead_code)]
pub struct UIState {
    pub is_menu_expanded: Rc<RefCell<bool>>,
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

/// 创建UI并返回主窗口
pub fn init_ui(cli_args: &CliArgs) -> Window {
    let mut main_window = init_window();
    
    // 初始化UI并获取所有组件和状态
    initialize_ui(cli_args, &mut main_window);
        
    // 设置窗口关闭回调
    main_window.set_callback(|_| {
        if app::event() == Event::Close {
            app::quit();
        }
    });
    
    main_window
}

/// 从命令行参数初始化路径
fn initialize_paths_from_cli(cli_args: &crate::CliArgs) -> (Rc<RefCell<Option<String>>>, Rc<RefCell<Option<String>>>) {
    (
        Rc::new(RefCell::new(cli_args.base_path.clone())),
        Rc::new(RefCell::new(cli_args.anime_path.clone())),
    )
}

/// 完整初始化UI并返回所有必要的组件和状态
fn initialize_ui(cli_args: &crate::CliArgs, wind: &mut Window){
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
    register_path_selection_callbacks(
        &mut controls.btn_choose_base,
        base_path_rc,
        file_browser,
        &controls.search_input,
        &mut controls.btn_choose_anime,
        anime_path_rc,
    );

    register_search_callbacks(
        &mut controls.search_input,
        &mut controls.search_button,
        &ui_state.search_results_rc,
        search_results_browser,
    );

    register_browser_event_callbacks(
        file_browser,
        search_results_browser,
        &ui_state.search_results_rc,
        &ui_state.episode_list_rc,
        &ui_state.selected_anime_id_rc,
        &mut controls.info_frame,
        base_path_rc,
        anime_path_rc,
    );

    register_done_button_callback(
        &mut controls.btn_done,
        file_browser,
        &ui_state.episode_list_rc,
        base_path_rc,
        anime_path_rc,
        &ui_state.search_results_rc,
        &ui_state.selected_anime_id_rc,
        search_results_browser,
    );
}
/// 创建所有UI控件
fn create_ui_controls(wind: &mut Window) -> (UIControls, FileBrowser, MultiBrowser) {
    // 创建核心控件
    let (mut btn_choose_base, mut btn_choose_anime, btn_done, search_input, search_button) = 
        ui_core::create_core_controls();
    let (mut file_browser, mut search_results_browser) = 
        ui_core::create_main_browsers();
    
    // 创建系统菜单栏，直接使用常量设定宽度
    let mut sys_menu_bar = SysMenuBar::new(0, 0, THREE_FIFTHS_WIDTH, 30, "");
    
    // 添加菜单项 - 移除文本中的"&"符号，避免显示下划线
    sys_menu_bar.add("工具/注册右键菜单", fltk::enums::Shortcut::None, MenuFlag::Normal, |_| {
        ui_core::handle_register_context_menu();
    });
    sys_menu_bar.add("工具/注销右键菜单", fltk::enums::Shortcut::None, MenuFlag::Normal, |_| {
        ui_core::handle_unregister_context_menu();
    });
    sys_menu_bar.add("帮助/关于", fltk::enums::Shortcut::None, MenuFlag::Normal, |_| {
        ui_core::handle_about_button();
    });
    
    // 构建UI布局
    let (
        main_vertical_flex,
        info_frame,  
    ) = build_ui_layout(
        wind,
        &mut btn_choose_base, &mut btn_choose_anime, &btn_done, &search_input, &search_button,
        &mut sys_menu_bar, // 修改为可变引用
        &mut file_browser, &mut search_results_browser,
    );
    
    // 创建UI控件集合结构体
    let controls = UIControls {
        main_vertical_flex,
        btn_choose_base,
        btn_choose_anime,
        btn_done,
        search_input,
        search_button,
        sys_menu_bar,
        info_frame,
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
    let search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>> = Rc::new(RefCell::new(None));
    let episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>> = Rc::new(RefCell::new(None));
    let selected_anime_id_rc: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    // 设置按钮颜色而非标签
    btn_choose_base.set_label("");
    btn_choose_anime.set_label("");
    btn_choose_base.set_color(fltk::enums::Color::from_hex(0x39C5BB));
    btn_choose_anime.set_color(fltk::enums::Color::from_hex(0x9999FF));

    if let Some(cli_base_path_str) = base_path_rc.borrow().as_deref() {
        io::load_files_to_file_browser(cli_base_path_str, file_browser);
        if let Some(extracted_anime_name) = io::extract_anime_name_from_path(cli_base_path_str) {
            search_input.set_value(&extracted_anime_name);
            println!("从命令行路径 {} 提取到番剧名: {}", cli_base_path_str, extracted_anime_name);
        }
    }

    // 创建UI状态结构体
    UIState {
        is_menu_expanded,
        search_results_rc,
        episode_list_rc,
        selected_anime_id_rc,
    }
}

/// 构建UI主布局
#[allow(clippy::too_many_arguments)]
fn build_ui_layout(
    _wind: &mut Window,
    btn_choose_base: &mut Button,
    btn_choose_anime: &mut Button,
    btn_done: &Button, 
    search_input: &Input, 
    search_button: &Button,
    sys_menu_bar: &mut SysMenuBar, // 保持可变引用，以便其他可能的操作
    file_browser: &mut FileBrowser, 
    search_results_browser: &mut MultiBrowser,
) -> (Flex, Frame) {
    // 创建顶部水平布局，菜单栏在左，搜索框在右
    let mut top_bar_flex = Flex::new(0, 0, WINDOW_WIDTH, 30, "");
    top_bar_flex.set_type(fltk::group::FlexType::Row);
    
    // 添加菜单栏到顶部布局左侧，不再需要resize
    top_bar_flex.add(sys_menu_bar);
    
    // 创建右侧搜索区域，占据五分之二
    let mut search_area = Flex::new(0, 0, TWO_FIFTHS_WIDTH, 30, "");
    search_area.set_type(fltk::group::FlexType::Row);
    search_area.add(search_input);
    search_area.add(search_button);
    search_area.fixed(search_button, 40);
    search_area.end();
    
    top_bar_flex.add(&search_area);
    top_bar_flex.fixed(&search_area, TWO_FIFTHS_WIDTH); // 固定搜索区域宽度
    top_bar_flex.end();
    
    // 创建主垂直Flex布局，从菜单栏下方开始
    let mut main_vertical_flex = Flex::new(0, 30, WINDOW_WIDTH, WINDOW_HEIGHT - 30, "");
    main_vertical_flex.set_type(fltk::group::FlexType::Column);
    
    // 创建内容区域 - 使用水平Flex
    let mut content_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, "");
    content_flex.set_type(fltk::group::FlexType::Row);
    content_flex.set_spacing(0); // 设置为0，去除子元素之间的间距
    content_flex.set_margin(0);  // 移除外边距
    
    // 左侧文件浏览区 - 现在占据五分之三
    let mut left_flex = Flex::new(0, 0, 0, 0, "");
    left_flex.set_type(fltk::group::FlexType::Column);
    left_flex.set_margin(0);  // 移除所有外边距
    left_flex.set_pad(0);     // 移除所有内边距
    
    // 设置文件浏览器的边框类型为FlatBox
    file_browser.set_frame(fltk::enums::FrameType::FlatBox);
    
    left_flex.add(file_browser);
    left_flex.end();
    
    // 中间按钮区，垂直排列B按钮和A按钮
    let mut mid_flex = Flex::new(0, 0, 10, 0, "");
    mid_flex.set_type(fltk::group::FlexType::Column);
    mid_flex.set_spacing(0); // 确保没有间隙
    mid_flex.set_margin(0);  // 移除所有外边距
    mid_flex.set_pad(0);     // 移除所有内边距
    
    // 设置按钮样式，确保它们完全填满容器
    btn_choose_base.set_down_frame(fltk::enums::FrameType::GtkDownBox);
    btn_choose_anime.set_down_frame(fltk::enums::FrameType::GtkDownBox);
    btn_choose_base.set_frame(fltk::enums::FrameType::GtkUpBox);
    btn_choose_anime.set_frame(fltk::enums::FrameType::GtkUpBox);
    
    // 添加按钮边缘阴影效果
    btn_choose_base.set_selection_color(fltk::enums::Color::from_hex(0x29B5AB)); // 按下时的颜色，稍深
    btn_choose_anime.set_selection_color(fltk::enums::Color::from_hex(0x8989EF)); // 按下时的颜色，稍深
    
    // 移除按钮的内边距并设置为完全填充
    btn_choose_base.clear_visible_focus();
    btn_choose_anime.clear_visible_focus();
    
    mid_flex.add(btn_choose_base);
    mid_flex.add(btn_choose_anime);
    mid_flex.end();
    
    // 右侧搜索结果区 - 现在占据五分之二
    let mut right_flex = Flex::new(0, 0, 0, 0, "");
    right_flex.set_type(fltk::group::FlexType::Column);
    right_flex.set_margin(0);  // 移除所有外边距
    right_flex.set_pad(0);     // 移除所有内边距
    
    // 设置搜索结果浏览器的边框类型为FlatBox
    search_results_browser.set_frame(fltk::enums::FrameType::FlatBox);
    
    right_flex.add(search_results_browser);
    right_flex.end();
    
    // 添加所有部分到内容区域，调整比例为3:2
    content_flex.add(&left_flex);
    content_flex.add(&mid_flex);
    content_flex.fixed(&mid_flex, 15); // 固定中间区域宽度为15px
    content_flex.add(&right_flex);
    content_flex.fixed(&right_flex, TWO_FIFTHS_WIDTH - 15); // 右侧固定为五分之二减去中间按钮的宽度
    content_flex.end();
    
    main_vertical_flex.add(&content_flex);
    
    // 创建信息框
    let mut info_frame = Frame::new(0, 0, WINDOW_WIDTH, 25, "");
    info_frame.set_label_size(12);
    info_frame.set_align(fltk::enums::Align::Left | fltk::enums::Align::Inside);
    
    // 使用底部面板布局，将info_frame和btn_done放在同一行
    let bottom_panel = setup_bottom_panel(&info_frame, btn_done);
    main_vertical_flex.add(&bottom_panel);
    main_vertical_flex.fixed(&bottom_panel, 25);
    
    main_vertical_flex.end();
    _wind.resizable(&main_vertical_flex);
    _wind.end();
    
    (
        main_vertical_flex,
        info_frame,
    )
}

/// 设置底部面板布局
fn setup_bottom_panel(info_frame: &Frame, btn_done: &Button) -> Flex {
    let mut bottom_panel_flex = Flex::new(0, 0, WINDOW_WIDTH, 25, "");
    bottom_panel_flex.set_type(fltk::group::FlexType::Row);
    bottom_panel_flex.add(info_frame);
    // 移除原有的空白占位符
    bottom_panel_flex.add(btn_done);
    bottom_panel_flex.fixed(btn_done, 80); // 固定"完成"按钮的宽度为80
    bottom_panel_flex.end();
    bottom_panel_flex
}

/// 设置内容区域
fn setup_content_area() -> (Flex, Flex, Flex, Flex) {
    let mut content_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, "");
    content_flex.set_type(fltk::group::FlexType::Row);
    content_flex.set_spacing(0); // 设置横向元素间距为0
    
    // 左侧文件浏览区 - 修改为占五分之三
    let mut left_flex = Flex::new(0, 0, THREE_FIFTHS_WIDTH - 5, 0, "");
    left_flex.set_type(fltk::group::FlexType::Column);
    left_flex.set_margin(5);
    left_flex.end();
    
    // 中间按钮区，宽度设置为10
    let mut mid_flex = Flex::new(0, 0, 10, 0, "");
    mid_flex.set_type(fltk::group::FlexType::Column);
    mid_flex.end();
    
    // 右侧搜索结果区 - 修改为占五分之二
    let mut right_flex = Flex::new(0, 0, TWO_FIFTHS_WIDTH - 5, 0, "");
    right_flex.set_type(fltk::group::FlexType::Column);
    right_flex.set_margin(5);
    right_flex.end();
    
    content_flex.add(&left_flex);
    content_flex.add(&mid_flex);
    content_flex.fixed(&mid_flex, 10); // 固定中间区域宽度为10
    content_flex.add(&right_flex);
    content_flex.end();
    
    (content_flex, left_flex, mid_flex, right_flex)
}

// --- 回调注册辅助函数 ---

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
    file_browser: &FileBrowser, 
    search_input: &Input,      
    btn_choose_anime: &mut Button,
    anime_path_rc: &Rc<RefCell<Option<String>>>
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
        ui_core::handle_search( // <--- 修改此处
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
            ui_core::handle_search( // <--- 修改此处
                search_input_clone_enter.clone(),
                search_results_rc_clone_enter.clone(),
                srb_clone_enter.clone(),
            );
            return true; // <--- 确保返回 true
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
    info_frame: &mut Frame,
    base_path_rc: &Rc<RefCell<Option<String>>>,
    anime_path_rc: &Rc<RefCell<Option<String>>>,
) {
    // 搜索结果浏览器双击回调
    let search_results_rc_clone = search_results_rc.clone();
    let episode_list_rc_clone = episode_list_rc.clone();
    let selected_anime_id_rc_clone = selected_anime_id_rc.clone();
    let mut info_frame_clone = info_frame.clone();
    let base_path_rc_clone = base_path_rc.clone();
    let anime_path_rc_clone = anime_path_rc.clone();
    
    search_results_browser.set_callback(move |b| {
        // 更新info_frame显示anime_path
        if let Some(path) = anime_path_rc_clone.borrow().clone() {
            info_frame_clone.set_label(&format!("番剧路径: {}", path));
        } else {
            info_frame_clone.set_label("番剧路径: 未选择");
        }
        
        ui_core::handle_search_results_double_click(
            b,
            search_results_rc_clone.clone(),
            episode_list_rc_clone.clone(),
            selected_anime_id_rc_clone.clone(),
        );
    });

    // 文件浏览器事件回调
    let mut info_frame_clone = info_frame.clone();
    let _base_path_rc_clone = base_path_rc.clone(); // 添加下划线前缀避免警告
    file_browser.set_callback(move |_| {
        // 更新info_frame显示base_path
        if let Some(path) = _base_path_rc_clone.borrow().clone() {
            info_frame_clone.set_label(&format!("基础路径: {}", path));
        } else {
            info_frame_clone.set_label("基础路径: 未选择");
        }
    });
    
    // 保留原有的事件处理，添加_前缀避免未使用警告
    let _original_handle = file_browser.handle(move |b, ev| {
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