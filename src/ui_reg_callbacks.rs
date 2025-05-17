// --- 回调注册函数 ---

/// 注册菜单触发器回调
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
        handle_menu_toggle(
            is_menu_expanded_cb.clone(),
            &mut main_flex_cb,
            &mut menu_panel_cb,
            &mut path_panel_cb.parent().unwrap().as_window().unwrap(),
        );
    });
}

/// 注册路径触发器回调
pub fn register_path_trigger_callback(
    path_trigger_button: &mut Button, 
    is_path_panel_expanded: Rc<RefCell<bool>>,
    path_display_panel_flex: &mut Flex,
    menu_items_panel_flex: &mut Flex,
    is_menu_expanded: Rc<RefCell<bool>>,
    main_vertical_flex: &mut Flex,
) {
    let is_path_panel_expanded_cb = is_path_panel_expanded.clone();
    let mut main_flex_cb = main_vertical_flex.clone();
    let mut path_panel_cb = path_display_panel_flex.clone();
    let mut wind_cb = path_panel_cb.parent().unwrap().as_window().unwrap();
    
    path_trigger_button.set_callback(move |_| {
        handle_path_panel_toggle(
            is_path_panel_expanded_cb.clone(),
            &mut main_flex_cb,
            &mut path_panel_cb,
            &mut wind_cb,
        );
    });
}

/// 注册设置按钮回调
pub fn register_settings_button_callback(settings_button: &mut Button) {
    settings_button.set_callback(|_| handle_register_context_menu());
}

/// 注册注销按钮回调
pub fn register_unregister_button_callback(unregister_button: &mut Button) {
    unregister_button.set_callback(|_| handle_unregister_context_menu());
}

/// 注册关于按钮回调
pub fn register_about_button_callback(about_button: &mut Button) {
    about_button.set_callback(|_| handle_about_button());
}

/// 注册选择源路径按钮回调
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

/// 注册选择目标路径按钮回调
pub fn register_choose_anime_callback(
    btn_choose_anime: &mut Button, 
    anime_path_rc: Rc<RefCell<Option<String>>>, 
    btn_choose_base: &mut Button
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

/// 注册搜索输入框回调
pub fn register_search_input_callback(
    search_input: &mut Input, 
    search_button: &mut Button, 
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    search_results_browser: MultiBrowser
) {
    let search_input_cb = search_input.clone();
    let search_results_rc_cb = search_results_rc.clone();
    let search_results_browser_cb = search_results_browser.clone();
    
    search_input.handle(move |_, ev| {
        if ev == Event::KeyDown && app::event_key() == Key::Enter {
            // 按下回车键时执行搜索
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

/// 注册搜索按钮回调
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

/// 注册搜索结果浏览器回调
pub fn register_search_results_browser_callback(
    search_results_browser: &mut MultiBrowser,
    search_results_rc: Rc<RefCell<Option<Vec<api::BangumiSubject>>>>,
    episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>>,
    selected_anime_id_rc: Rc<RefCell<Option<String>>>,
    file_browser: &mut FileBrowser
) {
    let search_results_rc_cb = search_results_rc.clone();
    let episode_list_rc_cb = episode_list_rc.clone();
    let selected_anime_id_rc_cb = selected_anime_id_rc.clone();
    
    search_results_browser.set_callback(move |b| {
        // 只处理双击事件
        if app::event() != Event::Released || app::event_clicks() != true {
            return;
        }

        let selected_idx = b.value(); // 1-indexed
        if selected_idx <= 0 {
            return;
        }

        // 获取选中的番剧ID
        if let Some(subjects) = &*search_results_rc_cb.borrow() {
            let actual_idx = (selected_idx as usize) - 1;
            if actual_idx < subjects.len() {
                let subject = &subjects[actual_idx];
                let subject_id = subject.id.to_string();
                
                // 更新选中的番剧ID
                *selected_anime_id_rc_cb.borrow_mut() = Some(subject_id.clone());
                
                println!("获取番剧ID: {} 的剧集信息...", subject_id);
                match <api::EpisodeCollection as api::ResourceFetcher<&str>>::fetch(&subject_id) {
                    Ok(ep_collection) => {
                        *episode_list_rc_cb.borrow_mut() = Some(ep_collection);
                        println!("成功获取到剧集信息");
                    },
                    Err(err_msg) => {
                        *episode_list_rc_cb.borrow_mut() = None;
                        dialog::message_default(&format!("获取剧集信息失败：{}", err_msg));
                    }
                }
            }
        }
    });
}

/// 注册完成按钮回调
pub fn register_done_button_callback(
    btn_done: &mut Button, 
    file_browser: FileBrowser,
    episode_list_rc: Rc<RefCell<Option<api::EpisodeCollection>>>,
    base_path_rc: Rc<RefCell<Option<String>>>,
    anime_path_rc: Rc<RefCell<Option<String>>>,
    selected_anime_id_rc: Rc<RefCell<Option<String>>>,
    search_results_browser: MultiBrowser,
) {
    let file_browser_cb = file_browser.clone();
    let episode_list_rc_cb = episode_list_rc.clone();
    let base_path_rc_cb = base_path_rc.clone();
    let anime_path_rc_cb = anime_path_rc.clone();
    let search_results_rc = Rc::new(RefCell::new(None::<Vec<api::BangumiSubject>>));
    let selected_anime_id_rc_cb = selected_anime_id_rc.clone();
    let search_results_browser_cb = search_results_browser.clone();
    
    btn_done.set_callback(move |_| {
        handle_done_button_callback(
            file_browser_cb.clone(),
            episode_list_rc_cb.clone(),
            base_path_rc_cb.clone(),
            anime_path_rc_cb.clone(),
            search_results_rc.clone(),
            selected_anime_id_rc_cb.clone(),
            search_results_browser_cb.clone(),
        );
    });
}
