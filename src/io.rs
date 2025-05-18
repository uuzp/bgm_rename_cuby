// src/io.rs

use std::path::{Path, Component};
use std::env;
use winreg::enums::*; 
use winreg::RegKey;
use fltk::dialog;
use fltk::prelude::*;
use fltk::browser::FileBrowser; // 明确导入 FileBrowser

// 新增：支持的视频文件扩展名常量
static SUPPORTED_VIDEO_EXTENSIONS: &[&str] = &["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm"];

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

/// 辅助函数：创建单个右键菜单注册表项
fn create_context_menu_entry(
    hkey_classes_root: &RegKey,
    shell_path: &str,
    key_name: &str,
    command_val: &str,
    exe_path: &str,
    errors: &mut Vec<String>,
) {
    match hkey_classes_root.create_subkey(format!("{}\\{}", shell_path, key_name)) {
        Ok((key, _disp)) => {
            if let Err(e) = key.set_value("", &key_name) {
                errors.push(format!("设置菜单默认值失败 ({}): {}", shell_path, e));
            }
            if let Err(e) = key.set_value("Icon", &format!("\"{}\",0", exe_path)) {
                errors.push(format!("设置菜单Icon失败 ({}): {}", shell_path, e));
            }
            match key.create_subkey("command") {
                Ok((cmd_key, _)) => {
                    if let Err(e) = cmd_key.set_value("", &command_val) {
                        errors.push(format!("设置命令失败 ({}): {}", shell_path, e));
                    }
                }
                Err(e) => errors.push(format!("创建命令子键失败 ({}): {}", shell_path, e)),
            }
        }
        Err(e) => errors.push(format!(
            "创建菜单主键 (HKCR\\{}\\{}) 失败: {}",
            shell_path, key_name, e
        )),
    }
}

/// 注册右键菜单
pub fn register_context_menu() {
    match env::current_exe() {
        Ok(exe_path_buf) => {
            let exe_path = exe_path_buf.to_string_lossy().to_string();
            let mut errors = Vec::new();
            let hkey_classes_root = RegKey::predef(HKEY_CLASSES_ROOT);
            let key_name = "Add To Cuby"; // 统一键名

            // 注册文件夹右键菜单
            let dir_shell_path = "Directory\\shell";
            let dir_command_val = format!("\"{}\" -b \"%1\" -a \"%1\\anime\"", exe_path);
            create_context_menu_entry(
                &hkey_classes_root,
                dir_shell_path,
                key_name,
                &dir_command_val,
                &exe_path,
                &mut errors,
            );

            // 注册文件夹背景右键菜单
            let dir_bg_shell_path = "Directory\\Background\\shell";
            let dir_bg_command_val = format!("\"{}\" -b \"%V\" -a \"%V\\anime\"", exe_path);
            create_context_menu_entry(
                &hkey_classes_root,
                dir_bg_shell_path,
                key_name, // 使用相同的键名
                &dir_bg_command_val,
                &exe_path,
                &mut errors,
            );

            if errors.is_empty() {
                dialog::message_default("注册表项已成功添加");
            } else {
                dialog::message_default(&format!("注册表操作时发生错误:\n{}\n\n请确保以管理员身份运行本程序", errors.join("\n")));
            }
        }
        Err(e) => {
            dialog::message_default(&format!("获取程序路径失败: {}", e));
        }
    }
}

/// 辅助函数：删除单个右键菜单注册表项
fn delete_context_menu_entry(
    hkey_classes_root: &RegKey,
    key_path: &str,
    errors: &mut Vec<String>,
    deleted_anything: &mut bool,
) {
    match hkey_classes_root.delete_subkey_all(key_path) {
        Ok(_) => {
            *deleted_anything = true;
        }
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!(
                    "删除菜单项失败 (HKCR\\{}): {}",
                    key_path, e
                ));
            }
        }
    }
}

/// 注销右键菜单
pub fn unregister_context_menu() {
    let mut errors = Vec::new();
    let hkey_classes_root = RegKey::predef(HKEY_CLASSES_ROOT);
    let mut deleted_anything = false;
    let key_name = "Add To Cuby"; // 统一键名，与注册时一致

    let dir_key_path = format!("Directory\\shell\\{}", key_name);
    delete_context_menu_entry(
        &hkey_classes_root,
        &dir_key_path,
        &mut errors,
        &mut deleted_anything,
    );

    let dir_bg_key_path = format!("Directory\\Background\\shell\\{}", key_name);
    delete_context_menu_entry(
        &hkey_classes_root,
        &dir_bg_key_path,
        &mut errors,
        &mut deleted_anything,
    );

    if errors.is_empty() {
        if deleted_anything { dialog::message_default("相关注册表项已成功删除"); } 
        else { dialog::message_default("未找到相关的注册表项，无需注销"); }
    } else {
        dialog::message_default(&format!("注销操作时发生错误:\n{}\n\n请确保以管理员身份运行本程序", errors.join("\n")));
    }
}

/// 加载文件到文件浏览器
pub fn load_files_to_file_browser(path_str: &str, browser: &mut FileBrowser) {
    browser.clear();
    if let Ok(entries) = std::fs::read_dir(path_str) {
        for entry_result in entries {
            if let Ok(entry) = entry_result {
                let file_path = entry.path();

                // 使用 match 结合文件属性进行判断
                match (
                    file_path.is_file(),
                    file_path.file_name().and_then(|n| n.to_str()),
                    file_path.extension().and_then(|e| e.to_str()),
                ) {
                    // 仅当是文件、有文件名、有扩展名时才处理
                    (true, Some(file_name), Some(ext_str)) => {
                        // 使用常量检查扩展名
                        if SUPPORTED_VIDEO_EXTENSIONS.contains(&ext_str.to_lowercase().as_str()) {
                            browser.add(file_name);
                        }
                        // 如果不匹配，则不执行任何操作（与之前的 _ => {} 行为一致）
                    }
                    _ => { /* 不是文件，或缺少文件名/扩展名，不处理 */ }
                }
            }
            // 忽略读取单个条目时可能发生的错误
        }
    } else {
        // 读取目录失败，可能是路径无效或权限问题
        dialog::message_default(&format!("无法读取目录: {}", path_str));
    }
}

/// 缩短路径以便在UI上显示
pub fn shorten_path_for_display(path_str: &str, max_len: usize) -> String {
    if path_str.is_empty() {
        return "".to_string();
    }
    if path_str.chars().count() <= max_len {
        return path_str.to_string();
    }

    let ellipsis = "...";
    let ellipsis_len = ellipsis.chars().count();

    // 如果 max_len 太小，无法容纳省略号，则直接从开头截取
    if max_len <= ellipsis_len {
        return path_str.chars().take(max_len).collect();
    }

    let path = Path::new(path_str);
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let filename_len = filename.chars().count();

    // 尝试格式: "start...filename"
    // 条件：文件名非空，并且 max_len 足够容纳 "至少一个字符的start" + "..." + "filename"
    if !filename.is_empty() && max_len > filename_len + ellipsis_len {
        let space_for_start = max_len - filename_len - ellipsis_len;
        // 确保 start_part 不会与 filename 重叠（如果路径很短，这由初始的长度检查处理）
        // path_str.chars().count() > max_len 保证了这一点
        let start_part: String = path_str.chars().take(space_for_start).collect();
        return format!("{}{}{}", start_part, ellipsis, filename);
    }

    // 回退格式: "...end_of_path"
    // (如果文件名太长，或者 "start...filename" 格式不适用)
    let chars_to_take_from_end = max_len - ellipsis_len;
    let path_chars_count = path_str.chars().count();
    let skip_count = path_chars_count.saturating_sub(chars_to_take_from_end);
    let end_part: String = path_str.chars().skip(skip_count).collect();
    format!("{}{}", ellipsis, end_part)
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
