// src/io.rs

use std::path::{Path, Component};
use std::env;
use winreg::enums::*; 
use winreg::RegKey;
use fltk::dialog;
use fltk::prelude::*;
use fltk::browser::FileBrowser; // 明确导入 FileBrowser

/// 替换文件名中的特殊字符
pub fn replace_invalid_chars(s: &str) -> String {
    s.replace("/", "／")
     .replace("\\", "＼") // Note: in a regular string, this would be a single backslash.
     .replace("<", "＜")
     .replace(">", "＞")
     // Consider adding other common problematic characters like : * ? " |
     .replace(":", "：")
     .replace("*", "＊")
     .replace("?", "？")
     .replace("\"", "＂")
     .replace("|", "｜")
}


/// 从路径中提取番剧名的函数
pub fn extract_anime_name_from_path(path_str: &str) -> Option<String> {
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


/// 注册右键菜单
pub fn register_context_menu() {
    match env::current_exe() {
        Ok(exe_path_buf) => {
            let exe_path = exe_path_buf.to_string_lossy().to_string();
            let mut errors = Vec::new();

            let hkey_classes_root = RegKey::predef(HKEY_CLASSES_ROOT);

            // 注册文件夹右键菜单
            let dir_shell_path = "Directory\\shell";
            let dir_key_name = "BgmRenameCuby";
            let dir_command_val = format!("\"{}\" -b \"%1\" -a \"%1\\anime\"", exe_path); 

            match hkey_classes_root.create_subkey(format!("{}\\{}", dir_shell_path, dir_key_name)) {
                Ok((key, _disp)) => {
                    if let Err(e) = key.set_value("", &"使用 BgmRenameCuby 处理文件夹") { errors.push(format!("设置文件夹菜单默认值失败: {}", e)); }
                    if let Err(e) = key.set_value("Icon", &format!("\"{}\",0", exe_path)) { errors.push(format!("设置文件夹菜单Icon失败: {}", e)); } 
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
            let dir_bg_command_val = format!("\"{}\" -b \"%V\" -a \"%V\\anime\"", exe_path); 

            match hkey_classes_root.create_subkey(format!("{}\\{}", dir_bg_shell_path, dir_key_name)) {
                Ok((key, _disp)) => {
                    if let Err(e) = key.set_value("", &"BgmRenameCuby 在此处理") { errors.push(format!("设置背景菜单默认值失败: {}", e)); }
                    if let Err(e) = key.set_value("Icon", &format!("\"{}\",0", exe_path)) { errors.push(format!("设置背景菜单Icon失败: {}", e)); } 
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

/// 注销右键菜单
pub fn unregister_context_menu() {
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

/// 加载文件到文件浏览器
pub fn load_files_to_file_browser(path_str: &str, browser: &mut FileBrowser) {
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

/// 缩短路径以便在UI上显示
pub fn shorten_path_for_display(path_str: &str, max_len: usize) -> String {
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
        let skip_count = path_str.chars().count().saturating_sub(chars_from_end_to_take); 
        format!("{}{}", fallback_ellipsis, &path_str.chars().skip(skip_count).collect::<String>())
    } else {
        path_str.chars().take(max_len).collect()
    }
}

/// 验证操作路径
pub fn validate_operation_paths(base_path_rc: &std::rc::Rc<std::cell::RefCell<Option<String>>>, anime_path_rc: &std::rc::Rc<std::cell::RefCell<Option<String>>>) -> Result<(String, String), String> {
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

/// 从浏览器收集源文件
pub fn collect_source_files_from_browser(file_browser: &FileBrowser) -> Vec<String> {
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
