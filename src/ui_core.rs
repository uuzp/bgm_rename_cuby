// src/ui_core.rs
// 创建UI窗口的核心逻辑

use fltk::{
    app,
    browser::{BrowserType, FileBrowser, MultiBrowser},
    button::Button,
    dialog,
    enums::{Color, Event, Key},
    group::Flex,
    input::Input,
    prelude::*,
};
use std::{
    cell::RefCell,
    path::Path,
    rc::Rc,
};

use crate::bangumi_api as api;
use crate::io;
use crate::{
    MENU_TRIGGER_HEIGHT, 
    MENU_ITEMS_PANEL_EXPANDED_HEIGHT, 
    PATH_DISPLAY_PANEL_EXPANDED_HEIGHT,
    MAX_BUTTON_LABEL_LEN
};

// --- 创建窗口标题栏及控件 ---

/// 创建窗口标题栏及控件
pub fn create_core_controls() -> (Button, Button, Button, Input, Button) {
    let mut btn_choose_base = Button::new(0, 0, 0, 0, "选择源路径(B)");
    btn_choose_base.set_tooltip("选择包含视频文件的源文件夹(B)");
    let mut btn_choose_anime = Button::new(0, 0, 0, 0, "选择目标路径 (A)");
    btn_choose_anime.set_tooltip("选择重命名后文件存放的目标文件夹 (A)");
    let mut btn_done = Button::new(0, 0, 0, 0, "✔️ 完成");
    btn_done.set_tooltip("开始重命名操作");
    let mut search_input = Input::new(0, 0, 0, 0, "");
    search_input.set_tooltip("输入番剧名称关键字进行搜索");
    let mut search_button = Button::new(0, 0, 40, 0, "🔍");
    search_button.set_tooltip("点击搜索");
    (btn_choose_base, btn_choose_anime, btn_done, search_input, search_button)
}

/// 创建菜单按钮
pub fn create_menu_buttons() -> (Button, Button, Button) {
    let mut settings_button = Button::new(0, 0, 0, 30, "⚙️ 注册");
    settings_button.set_tooltip("注册右键菜单到系统");
    let mut unregister_button = Button::new(0, 0, 0, 30, "🗑️ 注销");
    unregister_button.set_tooltip("从系统注销右键菜单");
    let mut about_button = Button::new(0, 0, 0, 30, "ℹ️ 关于");
    about_button.set_tooltip("查看关于信息");
    (settings_button, unregister_button, about_button)
}

/// 创建文件浏览器
pub fn create_main_browsers() -> (FileBrowser, MultiBrowser) {
    let mut file_browser = FileBrowser::new(0, 0, 0, 0, "");
    file_browser.set_tooltip("选择包含视频文件的源文件夹");
    file_browser.set_selection_color(Color::Yellow);
    file_browser.set_type(BrowserType::Hold);
    file_browser.set_damage(true);

    let mut search_results_browser = MultiBrowser::new(0, 0, 0, 0, "");
    search_results_browser.set_tooltip("Bangumi API 搜索结果");
    search_results_browser.set_selection_color(Color::Yellow);
    search_results_browser.set_type(BrowserType::Hold);
    (file_browser, search_results_browser)
}

// --- UI 事件处理函数 ---

/// 处理菜单的展开与收起
pub fn handle_menu_toggle<W: fltk::prelude::WindowExt>(is_menu_expanded: Rc<RefCell<bool>>, main_flex: &mut fltk::group::Flex, menu_panel: &mut fltk::group::Flex, window: &mut W) {
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

/// 处理路径面板的展开与收起
pub fn handle_path_panel_toggle<W: fltk::prelude::WindowExt>(is_path_panel_expanded: Rc<RefCell<bool>>, main_flex: &mut fltk::group::Flex, path_panel: &mut fltk::group::Flex, window: &mut W) {
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

/// 处理选择源文件夹按钮回调
pub fn handle_choose_base_path_callback(base_path_rc: Rc<RefCell<Option<String>>>, mut btn_choose_base: Button, mut file_browser: FileBrowser, mut search_input: Input) {
    let mut dialog = dialog::FileDialog::new(dialog::FileDialogType::BrowseDir);
    dialog.set_title("选择源文件夹 (B)");
    dialog.show();
    let chosen_path_pb = dialog.filename();
    if !chosen_path_pb.as_os_str().is_empty() {
        let path = Path::new(&chosen_path_pb); // 使用 std::path::Path 明确指定
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

/// 处理选择目标文件夹按钮回调
pub fn handle_choose_anime_path_callback(anime_path_rc: Rc<RefCell<Option<String>>>, mut btn_choose_anime: Button) {
    let mut dialog = dialog::FileDialog::new(dialog::FileDialogType::BrowseDir);
    dialog.set_title("选择目标文件夹 (A)");
    dialog.show();
    let chosen_path = dialog.filename();
    if !chosen_path.as_os_str().is_empty() {
        let path = Path::new(&chosen_path); // 使用 std::path::Path 明确指定
        if path.is_dir() {
            if let Some(path_str) = path.to_str() {
                *anime_path_rc.borrow_mut() = Some(path_str.to_string());
                btn_choose_anime.set_label(&io::shorten_path_for_display(path_str, MAX_BUTTON_LABEL_LEN));
            }
        }
    }
}

/// 处理文件浏览器事件
pub fn handle_file_browser_events(browser: &mut FileBrowser, event: Event) -> bool {
    // 使用静态变量来跟踪拖放项和高亮行
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
        },
        Event::KeyDown => {
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
                        println!("交换行: {} 和 {}", HIGHLIGHTED_LINE, current_line);
                        
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
                let _y = app::event_y();
                let item = browser.value();
                
                if DRAG_ITEM < 0 {
                    DRAG_ITEM = item; // 记录开始拖拽的项
                    return true;
                }
                // 计算当前移动到哪一行
                // 由于没有直接的方法，我们根据y坐标简单估计行号
                let _y_pos = app::event_y();
                let new_item = browser.value(); // 默认使用当前选中行
                
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
pub fn handle_register_context_menu() {
    io::register_context_menu();
}

pub fn handle_unregister_context_menu() {
    io::unregister_context_menu();
}

pub fn handle_about_button() {
    let repo_url = "https://github.com/uuzp/bgm_rename_cuby";
    if webbrowser::open(repo_url).is_err() {
        dialog::message_default(&format!("无法打开链接: {}", repo_url));
        println!("Error opening URL: {}", repo_url);
    }
}

/// 处理搜索按钮点击的回调
pub fn handle_search_button_callback(
    search_input: Input,
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    mut search_results_browser: MultiBrowser,
) {
    let query = search_input.value();
    if !query.is_empty() {
        println!("正在搜索: {}", query);
        match <Vec<api::BangumiSubject> as api::ResourceFetcher<&str>>::fetch(&query) {
            Ok(subjects) => {
                if subjects.is_empty() {
                    println!("未找到番剧: {}", query);
                    search_results_browser.clear();
                    *search_results_rc.borrow_mut() = None;
                    dialog::message_default(&format!("未找到与\\\"{}\\\"相关的番剧。", query));
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
                dialog::message_default(&format!("搜索\\\"{}\\\"失败：请检查网络连接或稍后再试。\\n详细错误: {}", query, err_msg));
            }
        }
    } else {
        println!("搜索查询为空，不执行搜索。");
        search_results_browser.clear(); // 清空浏览器
        *search_results_rc.borrow_mut() = None; // 清空数据
    }
}

/// 处理搜索输入框回车键事件
pub fn handle_search_input_enter_key(
    search_input: Input,
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    mut search_results_browser: MultiBrowser,
) -> bool {
    let query = search_input.value();
    if !query.is_empty() {
        println!("通过回车搜索: {}", query);
        match <Vec<api::BangumiSubject> as api::ResourceFetcher<&str>>::fetch(&query) {
            Ok(subjects) => {
                if subjects.is_empty() {
                    println!("未找到番剧: {}", query);
                    search_results_browser.clear();
                    *search_results_rc.borrow_mut() = None;
                    dialog::message_default(&format!("未找到与\\\"{}\\\"相关的番剧。", query));
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
                dialog::message_default(&format!("搜索\\\"{}\\\"失败：请检查网络连接或稍后再试。\\n详细错误: {}", query, err_msg));
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
pub fn handle_search_results_double_click(
    browser: &mut MultiBrowser,
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>>,
    selected_anime_id_rc: Rc<RefCell<Option<String>>>,
) {
    if app::event_clicks() { // Ensure it's a double click
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
                            *episode_list_rc.borrow_mut() = Some(ep_collection.clone());
                            *selected_anime_id_rc.borrow_mut() = Some(subject_id.to_string());
                            
                            // 清除现有的搜索结果列表
                            browser.clear();

                            // 显示剧集信息到列表中
                            if !ep_collection.episodes.is_empty() {
                                for ep in &ep_collection.episodes {
                                    let name = if !ep.name_cn.is_empty() {
                                        ep.name_cn.clone()
                                    } else {
                                        ep.name.clone()
                                    };
                                    let display_text = format!("Ep.{:02} - {}", ep.sort, name);
                                    browser.add(&display_text);
                                }
                            } else {
                                browser.add("未能获取到剧集信息或剧集列表为空。");
                            }
                        }
                        Err(err_msg) => {
                            println!("获取剧集列表失败: {}", err_msg);
                            browser.clear();
                            browser.add(&format!("获取剧集列表失败: {}", err_msg));
                            *episode_list_rc.borrow_mut() = None;
                            *selected_anime_id_rc.borrow_mut() = None;
                        }
                    }
                }
            }
        }
    }
}

/// 流程控制：获取选中番剧的剧集数据
pub fn get_episode_data_for_processing(
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

/// "完成" 按钮的回调处理
#[allow(clippy::too_many_arguments)]
pub fn handle_done_button_callback(
    file_browser: FileBrowser,
    episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>>,
    base_path_rc: Rc<RefCell<Option<String>>>,
    anime_path_rc: Rc<RefCell<Option<String>>>,
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    _selected_anime_id_rc: Rc<RefCell<Option<String>>>, // ID is used for fetching, name/year from elsewhere
    search_results_browser: MultiBrowser,
) {
    println!("Done button clicked!");

    if episode_list_rc.borrow().is_none() {
        dialog::message_default("错误: 剧集列表为空。
请先在右侧搜索并双击选定一部番剧。");
        return;
    }
    if base_path_rc.borrow().is_none() {
        dialog::message_default("错误: 源路径未设置，请选择包含视频文件的源文件夹。");
        return;
    }
    if anime_path_rc.borrow().is_none() {
        dialog::message_default("错误: 目标路径未设置，请选择重命名后文件存放的目标文件夹。");
        return;
    }

    match io::validate_operation_paths(&base_path_rc, &anime_path_rc) {
        Ok((base_path_str, anime_path_root_str)) => {
            println!("Base path: {}, Anime path root: {}", base_path_str, anime_path_root_str);
            let source_files = io::collect_source_files_from_browser(&file_browser);

            if source_files.is_empty() {
                dialog::message_default("错误: 未选择任何文件进行处理。
请在左侧文件浏览器中选择文件。");
                return;
            }
            println!("Source files selected: {:?}", source_files);

            if let Ok(ep_collection) = get_episode_data_for_processing(&episode_list_rc) {
                let year = ep_collection.year; // 获取 year 字段

                // 获取选择的番剧名称
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
                                dialog::message_default("错误: 选择的番剧无效。
请在右侧搜索结果中选择一个有效的番剧。");
                                return;
                            }
                        } else {
                            dialog::message_default("错误: 选择的番剧无效。
请在右侧搜索结果中选择一个有效的番剧。");
                            return;
                        }
                    } else {
                        dialog::message_default("错误: 请先搜索番剧并选择一个有效的番剧。");
                        return;
                    }
                };
                
                let cleaned_anime_name_for_folder = io::replace_invalid_chars(&anime_display_name);
                let target_anime_folder_name = format!("{}({})", cleaned_anime_name_for_folder, year); // 使用 year 字段
                let target_anime_dir = std::path::Path::new(&anime_path_root_str).join(target_anime_folder_name);

                println!("目标文件夹: {:?}", target_anime_dir);

                if let Err(e) = std::fs::create_dir_all(&target_anime_dir) {
                    dialog::message_default(&format!("错误: 创建目标文件夹失败: {}", e));
                    return;
                }

                let formatted_episode_names = ep_collection.get_formatted_names();
                
                if source_files.len() > formatted_episode_names.len() {
                     let msg = format!(
                        "警告: 选择的文件数量 ({}) 多于剧集数量 ({}).
这可能导致部分文件无法被处理。
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
                        let err_msg = format!("跳过文件 '{}': 超出剧集范围 (源文件总数: {}，剧集数: {})。",
                        source_file_name_str, source_files.len(), formatted_episode_names.len());
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

                    println!("重命名: {:?} -> {:?}", source_file_path, target_file_path);

                    if source_file_path == target_file_path {
                        let msg = format!("跳过文件 '{}': 源文件和目标文件相同，无需更名。", source_file_name_str);
                        println!("{}", msg);
                        errors_log.push(msg);
                        continue;
                    }
                    if target_file_path.exists() {
                        let msg = format!("跳过文件 '{}': 目标文件已存在 ('{}')。", source_file_name_str, target_file_path.display());
                         println!("{}", msg);
                        errors_log.push(msg);
                        failed_renames +=1;
                        continue;
                    }


                    if let Err(e) = std::fs::rename(&source_file_path, &target_file_path) {
                        let err_msg = format!("重命名文件 '{}' 失败: {}", source_file_name_str, e);
                        println!("{}", err_msg);
                        errors_log.push(err_msg);
                        failed_renames += 1;
                    } else {
                        successful_renames += 1;
                    }
                }
                
                // Reload files in base_path_str to reflect changes
                io::load_files_to_file_browser(&base_path_str, &mut file_browser.clone());


                let mut summary_message = format!("重命名完成报告:
成功: {}
失败: {}", successful_renames, failed_renames);
                if !errors_log.is_empty() {
                    summary_message.push_str("

详细错误信息:
");
                    summary_message.push_str(&errors_log.join("
"));
                }
                dialog::message_default(&summary_message);

            } else {
                 dialog::message_default("错误: 无法获取剧集信息。
请确保网络连接正常，并且可以访问 Bangumi API。");
            }
        }
        Err(e) => {
            dialog::message_default(&e);
        }
    }
}

// --- 注册事件处理函数 ---

/// 注册菜单触发按钮的回调
#[allow(dead_code)]
pub fn register_menu_trigger_callback(
    menu_trigger_button: &mut Button, 
    is_menu_expanded: Rc<RefCell<bool>>,
    menu_items_panel_flex: &mut Flex,
    path_display_panel_flex: &mut Flex,
    is_path_panel_expanded: Rc<RefCell<bool>>,
    main_vertical_flex: &mut Flex,
) {
    let is_menu_expanded_cb = is_menu_expanded.clone();
    let mut main_flex_cb = main_vertical_flex.clone();
    let mut menu_panel_cb = menu_items_panel_flex.clone();
    let mut path_panel_cb = path_display_panel_flex.clone();
    let is_path_panel_expanded_cb = is_path_panel_expanded.clone();
    
    menu_trigger_button.set_callback(move |_| {
        let mut expanded = is_menu_expanded_cb.borrow_mut();
        *expanded = !*expanded;
        
        if *expanded {
            // 展开菜单面板
            main_flex_cb.fixed(&menu_panel_cb, MENU_ITEMS_PANEL_EXPANDED_HEIGHT);
            menu_panel_cb.show();
            
            // 如果路径面板已展开，调整其位置
            if *is_path_panel_expanded_cb.borrow() {
                path_panel_cb.set_pos(0, MENU_TRIGGER_HEIGHT + MENU_ITEMS_PANEL_EXPANDED_HEIGHT);
            }
        } else {
            // 收起菜单面板
            menu_panel_cb.hide();
            main_flex_cb.fixed(&menu_panel_cb, 0);
            
            // 如果路径面板已展开，调整其位置
            if *is_path_panel_expanded_cb.borrow() {
                path_panel_cb.set_pos(0, MENU_TRIGGER_HEIGHT);
            }
        }
        
        main_flex_cb.layout();
        app::redraw();
    });
}

/// 注册路径触发按钮的回调
#[allow(dead_code)]
#[allow(unused_variables)]
pub fn register_path_trigger_callback(
    path_trigger_button: &mut Button, 
    is_path_panel_expanded: Rc<RefCell<bool>>,
    path_display_panel_flex: &mut Flex,
    menu_items_panel_flex: &mut Flex, // 当前菜单项面板
    is_menu_expanded: Rc<RefCell<bool>>,
    main_vertical_flex: &mut Flex,
) {
    let is_path_panel_expanded_cb = is_path_panel_expanded.clone();
    let mut main_flex_cb = main_vertical_flex.clone();
    let mut path_panel_cb = path_display_panel_flex.clone();    let is_menu_expanded_cb = is_menu_expanded.clone();
    // 链接当前菜单项面板的变量
    // let menu_panel_cb = menu_items_panel_flex.clone();
    
    path_trigger_button.set_callback(move |_| {
        let mut expanded = is_path_panel_expanded_cb.borrow_mut();
        *expanded = !*expanded;
        
        if *expanded {
            // 展开路径面板
            main_flex_cb.fixed(&path_panel_cb, PATH_DISPLAY_PANEL_EXPANDED_HEIGHT);
            
            // 根据菜单是否展开调整位置
            if *is_menu_expanded_cb.borrow() {
                path_panel_cb.set_pos(0, MENU_TRIGGER_HEIGHT + MENU_ITEMS_PANEL_EXPANDED_HEIGHT);
            } else {
                path_panel_cb.set_pos(0, MENU_TRIGGER_HEIGHT);
            }
            
            path_panel_cb.show();
        } else {
            // 收起路径面板
            path_panel_cb.hide();
            main_flex_cb.fixed(&path_panel_cb, 0);
        }
        
        main_flex_cb.layout();
        app::redraw();
    });
}

/// 注册设置按钮的回调
#[allow(dead_code)]
pub fn register_settings_button_callback(settings_button: &mut Button) {
    settings_button.set_callback(|_| handle_register_context_menu());
}

/// 注册注销按钮的回调
pub fn register_unregister_button_callback(unregister_button: &mut Button) {
    unregister_button.set_callback(|_| handle_unregister_context_menu());
}

/// 注册关于按钮的回调
pub fn register_about_button_callback(about_button: &mut Button) {
    about_button.set_callback(|_| handle_about_button());
}

/// 注册选择源路径按钮的回调
pub fn register_choose_base_callback(
    btn_choose_base: &mut Button, 
    base_path_rc: Rc<RefCell<Option<String>>>, 
    file_browser: FileBrowser,
    search_input: Input
) {
    let base_path_rc_cb = base_path_rc.clone();
    let btn_choose_base_cb = btn_choose_base.clone();
    let file_browser_cb = file_browser.clone();
    let search_input_cb = search_input.clone();
    
    btn_choose_base.set_callback(move |_| {
        handle_choose_base_path_callback(
            base_path_rc_cb.clone(),
            btn_choose_base_cb.clone(),
            file_browser_cb.clone(),
            search_input_cb.clone(),
        );
    });
}

/// 注册选择目标路径按钮的回调
pub fn register_choose_anime_callback(
    btn_choose_anime: &mut Button, 
    anime_path_rc: Rc<RefCell<Option<String>>>
) {
    let anime_path_rc_cb = anime_path_rc.clone();
    let btn_choose_anime_cb = btn_choose_anime.clone();
    
    btn_choose_anime.set_callback(move |_| {
        handle_choose_anime_path_callback(
            anime_path_rc_cb.clone(),
            btn_choose_anime_cb.clone(),
        );
    });
}

/// 注册搜索输入框的回调
pub fn register_search_input_callback(
    search_input: &mut Input, 
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    search_results_browser: MultiBrowser
) {
    let search_input_cb = search_input.clone();
    let search_results_rc_cb = search_results_rc.clone();
    let search_results_browser_cb = search_results_browser.clone();
    
    search_input.handle(move |_, ev| {
        if ev == Event::KeyDown && app::event_key() == Key::Enter {
            // 按回车键时触发搜索
            handle_search_button_callback(
                search_input_cb.clone(),
                search_results_rc_cb.clone(),
                search_results_browser_cb.clone(),
            );
            return true;
        }
        false
    });
}

/// 注册搜索按钮的回调
pub fn register_search_button_callback(
    search_button: &mut Button,
    search_input: &mut Input,
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    search_results_browser: MultiBrowser
) {
    let search_input_cb = search_input.clone();
    let search_results_rc_cb = search_results_rc.clone();
    let search_results_browser_cb = search_results_browser.clone();
    
    search_button.set_callback(move |_| {
        handle_search_button_callback(
            search_input_cb.clone(),
            search_results_rc_cb.clone(),
            search_results_browser_cb.clone(),
        );
    });
}

/// 注册搜索结果浏览器的回调
pub fn register_search_results_browser_callback(
    search_results_browser: &mut MultiBrowser,
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>>,
    selected_anime_id_rc: Rc<RefCell<Option<String>>>,
) {
    search_results_browser.set_callback(move |b| {
        handle_search_results_double_click(
            b,
            search_results_rc.clone(),
            episode_list_rc.clone(),
            selected_anime_id_rc.clone(),
        );
    });
}

