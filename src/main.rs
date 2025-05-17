#![windows_subsystem = "windows"] // 禁止在 Windows 上显示控制台窗口

use fltk::{
    app,
    browser::{FileBrowser, MultiBrowser},
    button::Button,
    dialog, // 保持对 dialog 模块的导入
    enums::{Color, Event, Key}, // 移除了 Font, Align
    frame::Frame,
    group::Flex,
    input::Input,
    prelude::*,
    window::Window,
};
use std::{
    cell::RefCell,
    env, // 新增：导入 env 模块
    path::{Path, Component},
    rc::Rc,
};
use winreg::enums::*; // 新增：导入 winreg 枚举
use winreg::RegKey;   // 新增：导入 RegKey

use clap::Parser;
use miniserde::{Deserialize, Serialize, json};
use minreq;
use urlencoding;
use webbrowser; // 确保 webbrowser 已导入

/// 替换文件名中的特殊字符
pub fn replace_invalid_chars(s: &str) -> String {
    s.replace("/", "／")
     .replace("\\\\", "＼")
     .replace("<", "＜")
     .replace(">", "＞")
}

#[derive(Deserialize, Serialize)]
struct SearchResult {
    list: Vec<Subject>,
}

#[derive(Deserialize, Serialize)]
struct Subject {
    id: u64,
    name: String,
    name_cn: String,
}

#[derive(Deserialize, Serialize)]
struct EpisodesResult {
    data: Vec<Episode>,
}

#[derive(Deserialize, Serialize)]
struct Episode {
    airdate: String,
    sort: u64,
    name: String,
    name_cn: String,
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

#[derive(Debug, Clone)] // 确保BgmApi也是Clone，如果它被类似Ep的方式处理
pub struct BgmApi {
    pub name: Vec<String>,
    pub id: Vec<String>,
}

impl BgmApi {
    pub fn new() -> Self {
        Self {
            name: Vec::new(),
            id: Vec::new(),
        }
    }      
    
    pub fn get(&mut self, keywords: &str) -> Result<(), String> {
        let encoded_keywords = urlencoding::encode(keywords);
        let url = format!(
            "https://api.bgm.tv/search/subject/{}?type=2&responseGroup=small",
            encoded_keywords
        );

        let response = minreq::get(url).send().map_err(|e| format!("网络请求失败: {}", e))?;
        let response_text = response.as_str().map_err(|e| format!("响应文本编码错误: {}", e))?;
        
        // 尝试解析 JSON，如果失败则返回错误
        let search_result: SearchResult = match json::from_str(&response_text) {
            Ok(res) => res,
            Err(e) => {
                // 尝试解析另一种可能的错误格式
                #[derive(Deserialize)]
                struct ErrorResponse {
                    title: Option<String>,
                    #[serde(rename = "type")]
                    error_type: Option<String>,
                    detail: Option<String>,
                }
                if let Ok(err_resp) = json::from_str::<ErrorResponse>(&response_text) {
                    return Err(format!(
                        "BGM API 错误: {}. 类型: {}. 详情: {}", 
                        err_resp.title.unwrap_or_default(), 
                        err_resp.error_type.unwrap_or_default(),
                        err_resp.detail.unwrap_or_default()
                    ));
                }
                return Err(format!("JSON 解析失败: {}. 原始响应: {}", e, response_text));
            }
        };

        self.name.clear(); // 清空旧数据
        self.id.clear();

        for subject in search_result.list {
            self.id.push(subject.id.to_string());
            self.name.push(
                if subject.name_cn.is_empty() {
                    subject.name
                } else {
                    subject.name_cn
                }
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Ep {
    pub name: Vec<String>,
    pub year: i32,
}

impl Ep {
    pub fn new() -> Self{
        Self { name: Vec::new(), year: 1970 }
    } 
    
    pub fn get(id: &str) -> Result<Self, String> {
        let url = format!(
            "https://api.bgm.tv/v0/episodes?subject_id={}&type=0&limit=100&offset=0", // 增加了 limit 参数
            id
        );
        let response = minreq::get(url)
            .with_header("User-Agent", "uuzp/bgm_rename_cuby") // 添加 User-Agent
            .send()
            .map_err(|e| format!("网络请求失败: {}", e))?;
        let response_text = response.as_str().map_err(|e| format!("响应文本编码错误: {}", e))?;
        
        // 尝试解析 JSON，如果失败则返回错误
        let episodes_result: EpisodesResult = match json::from_str(&response_text) {
            Ok(res) => res,
            Err(e) => {
                 // 尝试解析另一种可能的错误格式
                #[derive(Deserialize)]
                struct ErrorResponse {
                    title: Option<String>,
                    #[serde(rename = "type")]
                    error_type: Option<String>,
                    detail: Option<String>,
                }
                if let Ok(err_resp) = json::from_str::<ErrorResponse>(&response_text) {
                    return Err(format!(
                        "BGM API 错误 (剧集): {}. 类型: {}. 详情: {}", 
                        err_resp.title.unwrap_or_default(), 
                        err_resp.error_type.unwrap_or_default(),
                        err_resp.detail.unwrap_or_default()
                    ));
                }
                return Err(format!("JSON 解析失败 (剧集): {}. 原始响应: {}", e, response_text));
            }
        };

        let mut ep_list = Vec::new();
        let mut ep_sort_numbers = Vec::new(); // 修改变量名以更清晰
        let mut year = 1970; // 默认年份

        if let Some(first_episode) = episodes_result.data.first() {
             year = first_episode.airdate.get(0..4).unwrap_or("").parse().unwrap_or(1970);
        }        
        for episode in episodes_result.data {
            ep_sort_numbers.push(episode.sort);
            let name_to_use = if !episode.name_cn.is_empty() {
                episode.name_cn
            } else {
                episode.name
            };
            // 使用之前定义的 replace_invalid_chars 函数
            ep_list.push(replace_invalid_chars(&name_to_use));
        }

        let mut formatted_names = vec![]; // 修改变量名
        let max_ep_num = ep_sort_numbers.iter().max().cloned().unwrap_or(0);
        let num_digits = if max_ep_num == 0 { 1 } else { (max_ep_num as f64).log10() as usize + 1 };

        for i in 0..ep_list.len() {
            let ep_num_str = format!("{:0width$}", ep_sort_numbers[i], width = num_digits);
            let formatted_ep_name = format!("ep{} - {}", ep_num_str, ep_list[i]); // 修改变量名
            formatted_names.push(formatted_ep_name);
        }

        Ok(Ep { name: formatted_names, year })
    }
}

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
            let dir_command_val = format!("\\\"{}\\\" -b \\\"%1\\\" -a \\\"%1\\anime\\\"", exe_path);

            match hkey_classes_root.create_subkey(format!("{}\\{}", dir_shell_path, dir_key_name)) {
                Ok((key, _disp)) => {
                    if let Err(e) = key.set_value("", &"使用 BgmRenameCuby 处理文件夹") { errors.push(format!("设置文件夹菜单默认值失败: {}", e)); }
                    if let Err(e) = key.set_value("Icon", &exe_path) { errors.push(format!("设置文件夹菜单Icon失败: {}", e)); }
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
            let dir_bg_command_val = format!("\\\"{}\\\" -b \\\"%V\\\" -a \\\"%V\\anime\\\"", exe_path);

            match hkey_classes_root.create_subkey(format!("{}\\{}", dir_bg_shell_path, dir_key_name)) {
                Ok((key, _disp)) => {
                    if let Err(e) = key.set_value("", &"BgmRenameCuby 在此处理") { errors.push(format!("设置背景菜单默认值失败: {}", e)); }
                    if let Err(e) = key.set_value("Icon", &exe_path) { errors.push(format!("设置背景菜单Icon失败: {}", e)); }
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
        Ok(_) => {
            deleted_anything = true;
        }
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!("删除文件夹菜单项失败 (HKCR\\{}): {}", dir_key_path, e));
            }
        }
    }

    match hkey_classes_root.delete_subkey_all(dir_bg_key_path) {
        Ok(_) => {
            deleted_anything = true;
        }
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                errors.push(format!("删除背景菜单项失败 (HKCR\\{}): {}", dir_bg_key_path, e));
            }
        }
    }

    if errors.is_empty() {
        if deleted_anything {
            dialog::message_default("相关注册表项已成功删除。\n部分更改可能需要重启资源管理器或重新登录才能生效。");
        } else {
            dialog::message_default("未找到相关的注册表项，无需注销。");
        }
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

// --- 辅助函数，用于加载文件到 FileBrowser ---
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

// --- 移除不再使用的 run_reg_command_elevated 函数 ---

// --- 辅助函数，用于缩短路径以在按钮上显示 ---
fn shorten_path_for_display(path_str: &str, max_len: usize) -> String {
    if path_str.is_empty() {
        return "".to_string(); // 空路径直接返回空字符串
    }
    if path_str.len() <= max_len {
        return path_str.to_string(); // 路径长度未超限，直接返回
    }

    let path = Path::new(path_str);
    let ellipsis = "...\\"; // Windows 风格的省略号加路径分隔符

    // 1. 提取路径的盘符或UNC前缀
    //    例如："C:\" 或 "\\\\server\\share\"
    let drive_prefix_str = path.components().next().and_then(|c| match c {
        Component::Prefix(prefix_component) => {
            let prefix_os_str = prefix_component.as_os_str();
            let prefix_cow = prefix_os_str.to_string_lossy();
            let s = prefix_cow.as_ref();
            if s.ends_with(':') { // 标准盘符，如 "C:"
                Some(format!("{}:\\", s.trim_end_matches(':')))
            } else if s.starts_with("\\\\") { // UNC 路径，如 "\\server\share"
                // 确保 UNC 路径以反斜杠结尾，以便后续拼接
                Some(format!("{}\\", s.trim_end_matches('\\')))
            } else { // 其他类型的前缀 (例如 Verbatim 前缀)
                Some(format!("{}\\", s.trim_end_matches('\\')))
            }
        },
        _ => None, // 不是预期的前缀组件
    }).unwrap_or_else(|| {
        // 回退逻辑：处理 Component::Prefix 未能覆盖的简单 Windows 路径情况
        if path_str.len() > 1 && path_str.chars().nth(1) == Some(':') && path_str.chars().nth(2) == Some('\\') {
            // 例如 "C:\path"
            format!("{}:\\", path_str.chars().next().unwrap_or_default())
        } else if path_str.starts_with("\\\\") {
            // 进一步回退处理 UNC 路径，例如 "\\server\share\folder"
            // 尝试提取 \\server\share\ 部分
            let parts: Vec<&str> = path_str.splitn(4, '\\').filter(|s| !s.is_empty()).collect();
            if parts.len() >= 2 { format!("\\\\{}\\{}\\", parts[0], parts[1]) } else { "".to_string() }
        }
        else { "".to_string() } // 未能识别为标准 Windows 路径前缀
    });

    // 特殊情况：如果路径本身就是盘符前缀 (例如 "C:\" 或 "C:")
    if !drive_prefix_str.is_empty() && 
       (path_str == drive_prefix_str.trim_end_matches('\\') || path_str == drive_prefix_str) {
        return if drive_prefix_str.len() <= max_len {
            drive_prefix_str // 如果盘符本身未超长，直接返回
        } else {
            drive_prefix_str.chars().take(max_len).collect() // 如果盘符超长，则截断
        };
    }

    // 获取路径的最后一部分（文件名或文件夹名）
    let final_component_name = path.file_name()
        .and_then(|os_str| os_str.to_str())
        .unwrap_or("");

    // 获取路径的倒数第二部分（父文件夹名）
    let parent_folder_name = path.parent()
        .and_then(|p| p.file_name()) 
        .and_then(|os_str| os_str.to_str())
        .unwrap_or("");

    // 策略 1 & 2: 尝试格式 "盘符:\...\父文件夹\最终组件名"
    if !drive_prefix_str.is_empty() && !parent_folder_name.is_empty() && !final_component_name.is_empty() {
        let len_with_parent_and_final = drive_prefix_str.len()
            + ellipsis.len() 
            + parent_folder_name.len()
            + 1 // 父文件夹和最终组件之间的 '\'
            + final_component_name.len();

        if len_with_parent_and_final <= max_len {
            // 完整显示 "盘符:\...\父文件夹\最终组件名"
            return format!("{}{}{}\\{}", drive_prefix_str, ellipsis, parent_folder_name, final_component_name);
        }

        // 策略 2: 尝试格式 "盘符:\...\父文件夹\最终组..." (缩短最终组件)
        let space_for_final_after_parent = max_len.saturating_sub(
            drive_prefix_str.len()
            + ellipsis.len()
            + parent_folder_name.len()
            + 1 // 分隔符
        );

        if space_for_final_after_parent > 0 { // 确保有空间给最终组件（哪怕是缩短的）
            let shortened_final: String = final_component_name.chars().take(space_for_final_after_parent).collect();
            if !shortened_final.is_empty() { // 避免结果是 "盘符:\...\父文件夹\" 这样的形式
                return format!("{}{}{}\\{}", drive_prefix_str, ellipsis, parent_folder_name, shortened_final);
            }
        }
    }

    // 策略 3 & 4: 尝试格式 "盘符:\...\最终组件名" (当无法显示父文件夹或空间不足时)
    if !drive_prefix_str.is_empty() && !final_component_name.is_empty() {
        let len_with_final_only = drive_prefix_str.len() + ellipsis.len() + final_component_name.len();
        if len_with_final_only <= max_len {
            // 完整显示 "盘符:\...\最终组件名"
            return format!("{}{}{}", drive_prefix_str, ellipsis, final_component_name);
        }

        // 策略 4: 尝试格式 "盘符:\...\最终组..." (缩短最终组件)
        let remaining_space_for_filename = max_len
            .saturating_sub(drive_prefix_str.len())
            .saturating_sub(ellipsis.len());
        
        if remaining_space_for_filename > 0 { // 确保有空间给最终组件
            let shortened_filename: String = final_component_name.chars().take(remaining_space_for_filename).collect();
            if !shortened_filename.is_empty() { // 避免结果是 "盘符:\...\\"
                return format!("{}{}{}", drive_prefix_str, ellipsis, shortened_filename);
            }
        }
    }
    
    // 最终回退逻辑:
    // 1. "...路径末尾部分"
    // 2. "路径开头部分" (如果空间连 "...末尾" 都放不下)
    let fallback_ellipsis = "..."; // 此处使用不带反斜杠的省略号
    if max_len > fallback_ellipsis.len() {
        let chars_from_end_to_take = max_len - fallback_ellipsis.len();
        // 计算需要跳过的字符数，确保不超出字符串长度
        let skip_count = path_str.len().saturating_sub(chars_from_end_to_take);
        format!("{}{}", fallback_ellipsis, &path_str.chars().skip(skip_count).collect::<String>())
    } else {
        // 如果最大长度太小，连 "...X" 都放不下，则直接截取路径的开头部分
        path_str.chars().take(max_len).collect()
    }
}

// --- UI 辅助设置函数 ---

/// 设置顶部触发器Flex容器（包含菜单、路径按钮和搜索区域）
fn setup_top_triggers_flex(
    btn_done: &Button,
    search_input: &Input,
    search_button: &Button,
) -> (Button, Button, Flex) { // 返回 (菜单按钮, 路径按钮, 顶部Flex)
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

    let top_spacer = Frame::new(0,0,0,0,""); // 占位符，将右侧控件推向右边
    top_triggers_flex.add(&top_spacer);

    top_triggers_flex.add(btn_done); // 添加已定义的“完成”按钮
    top_triggers_flex.fixed(btn_done, 80);

    let search_gap_spacer = Frame::new(0,0,10,0,""); // 搜索区与完成按钮的间隔
    top_triggers_flex.add(&search_gap_spacer);
    top_triggers_flex.fixed(&search_gap_spacer, 10);

    top_triggers_flex.add(search_input); // 添加已定义的搜索输入框
    top_triggers_flex.add(search_button); // 添加已定义的搜索按钮
    top_triggers_flex.fixed(search_button, 40);

    top_triggers_flex.end();
    (menu_trigger_button, path_trigger_button, top_triggers_flex)
}

/// 设置可折叠的菜单项面板
fn setup_menu_items_panel(
    settings_button: &Button,
    unregister_button: &Button,
    about_button: &Button,
) -> Flex {
    let mut menu_items_panel_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, ""); // 初始高度为0，因此隐藏
    menu_items_panel_flex.set_type(fltk::group::FlexType::Row);
    menu_items_panel_flex.set_margin(2);

    menu_items_panel_flex.add(settings_button);
    menu_items_panel_flex.fixed(settings_button, 80);
    menu_items_panel_flex.add(unregister_button);
    menu_items_panel_flex.fixed(unregister_button, 80);
    menu_items_panel_flex.add(about_button);
    menu_items_panel_flex.fixed(about_button, 100);

    menu_items_panel_flex.end();
    menu_items_panel_flex.hide(); // 初始隐藏
    menu_items_panel_flex
}

/// 设置可折叠的路径显示和选择面板
fn setup_path_display_panel(
    btn_choose_base: &Button,
    btn_choose_anime: &Button,
) -> Flex {
    let mut path_display_panel_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, ""); // 初始高度为0，因此隐藏
    path_display_panel_flex.set_type(fltk::group::FlexType::Row);
    path_display_panel_flex.set_margin(2);
    path_display_panel_flex.add(btn_choose_base);
    path_display_panel_flex.add(btn_choose_anime);
    path_display_panel_flex.end();
    path_display_panel_flex.hide(); // 初始隐藏
    path_display_panel_flex
}

/// 设置主内容区域 (包含左右两个Flex面板)
fn setup_content_area() -> (Flex, Flex, Flex) { // 返回 (主内容Flex, 左侧Flex, 右侧Flex)
    let mut content_flex = Flex::new(0, 0, WINDOW_WIDTH, 0, ""); // 高度将由父Flex自动分配
    content_flex.set_type(fltk::group::FlexType::Row);

    let mut left_flex = Flex::new(0, 0, HALF_WIDTH, 0, ""); // 左半部分
    left_flex.set_type(fltk::group::FlexType::Column);
    left_flex.set_margin(5);
    left_flex.end();

    let mut right_flex = Flex::new(HALF_WIDTH, 0, HALF_WIDTH, 0, ""); // 右半部分
    right_flex.set_type(fltk::group::FlexType::Column);
    right_flex.set_margin(5);
    right_flex.end();

    content_flex.add(&left_flex);
    content_flex.add(&right_flex);
    content_flex.end();

    (content_flex, left_flex, right_flex)
}

// --- 回调处理函数 ---

/// 处理菜单展开/折叠按钮的点击事件
fn handle_menu_toggle(
    is_menu_expanded: Rc<RefCell<bool>>,
    main_flex: &mut Flex,
    menu_panel: &mut Flex,
    window: &mut Window,
) {
    let mut expanded = is_menu_expanded.borrow_mut();
    *expanded = !*expanded; // 切换状态

    if *expanded {
        main_flex.fixed(menu_panel, MENU_ITEMS_PANEL_EXPANDED_HEIGHT);
        menu_panel.show();
    } else {
        menu_panel.hide();
        main_flex.fixed(menu_panel, 0); // 设置为0以隐藏并释放空间
    }
    main_flex.layout(); // 重新计算布局
    window.redraw(); // 重绘窗口
}

/// 处理路径面板展开/折叠按钮的点击事件
fn handle_path_panel_toggle(
    is_path_panel_expanded: Rc<RefCell<bool>>,
    main_flex: &mut Flex,
    path_panel: &mut Flex,
    window: &mut Window,
) {
    let mut expanded = is_path_panel_expanded.borrow_mut();
    *expanded = !*expanded; // 切换状态

    if *expanded {
        main_flex.fixed(path_panel, PATH_DISPLAY_PANEL_EXPANDED_HEIGHT);
        path_panel.show();
    } else {
        path_panel.hide();
        main_flex.fixed(path_panel, 0); // 设置为0以隐藏并释放空间
    }
    main_flex.layout(); // 重新计算布局
    window.redraw(); // 重绘窗口
}

/// 处理选择源路径按钮（B按钮）的回调
fn handle_choose_base_path_callback(
    base_path_rc: Rc<RefCell<Option<String>>>,
    mut btn_choose_base: Button, // 克隆的按钮控件
    mut file_browser: FileBrowser, // 克隆的文件浏览器控件
    mut search_input: Input,    // 克隆的搜索输入框控件
) {
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

                // 尝试从路径中提取番剧名并填充搜索框
                if let Some(extracted_anime_name) = extract_anime_name_from_path(path_str) {
                    search_input.set_value(&extracted_anime_name);
                    println!("从路径 {} 提取到番剧名: {}", path_str, extracted_anime_name);
                }
            }
        }
    }
}

/// 处理选择目标路径按钮（A按钮）的回调
fn handle_choose_anime_path_callback(
    anime_path_rc: Rc<RefCell<Option<String>>>,
    mut btn_choose_anime: Button, // 克隆的按钮控件
) {
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

/// 处理搜索按钮点击的回调
fn handle_search_button_callback(
    search_input: Input, 
    search_results_rc: Rc<RefCell<Option<BgmApi>>>,

    mut search_results_browser: MultiBrowser, 
) {
    let query = search_input.value();
    if !query.is_empty() {
        println!("正在搜索: {}", query);
        let mut bgm_api_instance = BgmApi::new(); // 创建实例
        match bgm_api_instance.get(&query) { // 调用实例方法
            Ok(_) => { // get 现在返回 Ok(())
                println!("搜索成功，找到 {} 个结果", bgm_api_instance.name.len());
                search_results_browser.clear();
                for name in &bgm_api_instance.name {
                    search_results_browser.add(&name.replace("&", "&&"));
                }
                *search_results_rc.borrow_mut() = Some(bgm_api_instance); // 存储更新后的实例
            }
            Err(err_msg) => {
                println!("搜索失败: {}", err_msg);
                dialog::message_default(&format!("搜索失败: {}", err_msg));
            }
        }
    } else {
        println!("搜索查询为空，不执行搜索。");
    }
}

/// 处理搜索输入框回车键事件
fn handle_search_input_enter_key(
    search_input: Input, 
    search_results_rc: Rc<RefCell<Option<BgmApi>>>,

    mut search_results_browser: MultiBrowser, 
) -> bool { 
    let query = search_input.value();
    if !query.is_empty() {
        println!("通过回车搜索: {}", query);
        let mut bgm_api_instance = BgmApi::new(); // 创建实例
        match bgm_api_instance.get(&query) { // 调用实例方法
            Ok(_) => { // get 现在返回 Ok(())
                println!("搜索成功，找到 {} 个结果", bgm_api_instance.name.len());
                search_results_browser.clear();
                for name in &bgm_api_instance.name {
                    search_results_browser.add(&name.replace("&", "&&"));
                }
                *search_results_rc.borrow_mut() = Some(bgm_api_instance); // 存储更新后的实例
            }
            Err(err_msg) => {
                println!("搜索失败: {}", err_msg);
                dialog::message_default(&format!("搜索失败: {}", err_msg));
            }
        }
    } else {
        println!("搜索查询为空，不执行搜索。");
    }
    true 
}

/// 处理搜索结果列表项双击事件
fn handle_search_results_double_click(
    browser: &mut MultiBrowser, // 搜索结果浏览器本身
    search_results_rc: Rc<RefCell<Option<BgmApi>>>,
    episode_list_rc: Rc<RefCell<Option<Ep>>>,
    selected_anime_id_rc: Rc<RefCell<Option<String>>>,
) {
    if app::event_clicks() { // 检测是否为双击事件
        let line = browser.value(); // 获取选中行号 (1-based)
        if line > 0 && line <= browser.size() {
            if let Some(bgm_data) = &*search_results_rc.borrow() {
                let idx = (line as usize) - 1; // 转换为 0-based 索引
                if idx < bgm_data.id.len() {
                    let anime_id_str = bgm_data.id[idx].to_string(); // 获取并克隆番剧ID
                    println!("双击选中番剧ID: {}", anime_id_str); // 添加日志

                    match Ep::get(&anime_id_str) {
                        Ok(ep_data) => {
                            println!("获取到剧集信息: {} 个", ep_data.name.len());
                            *episode_list_rc.borrow_mut() = Some(ep_data);
                            *selected_anime_id_rc.borrow_mut() = Some(anime_id_str);
                        }
                        Err(err_msg) => {
                            println!("获取剧集列表失败: {}", err_msg); // 添加日志
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
    episode_list_rc: &Rc<RefCell<Option<Ep>>>
) -> Result<Ep, String> {
    match episode_list_rc.borrow().as_ref() {
        Some(ep_data) => Ok(ep_data.clone()), // 克隆 Ep 数据
        None => {
            let err_msg = "剧集列表为空，无法执行操作。";
            println!("{}", err_msg);
            // dialog::message_default(err_msg); // 对话框通常在调用方处理
            Err(err_msg.to_string())
        }
    }
}

/// 验证操作所需的路径是否已设置
fn validate_operation_paths(
    base_path_rc: &Rc<RefCell<Option<String>>>,
    anime_path_rc: &Rc<RefCell<Option<String>>>
) -> Result<(String, String), String> {
    let base_path_str = match base_path_rc.borrow().as_ref() {
        Some(path) => path.clone(),
        None => {
            return Err("错误: 未设置源文件路径（B按钮）".to_string());
        }
    };
    let anime_path_str = match anime_path_rc.borrow().as_ref() {
        Some(path) => path.clone(),
        None => {
            return Err("错误: 未设置目标位置路径（A按钮）".to_string());
        }
    };
    Ok((base_path_str, anime_path_str))
}

/// 从文件浏览器收集选中的源文件名称
fn collect_source_files_from_browser(file_browser: &FileBrowser) -> Vec<String> {
    let mut file_names = Vec::new();
    for i in 1..=file_browser.size() {
        if let Some(text) = file_browser.text(i) {
            file_names.push(text.to_string());
        }
    }
    file_names
}

/// 执行文件的重命名（通过硬链接）和字幕处理
fn perform_file_renaming_and_linking(
    base_path_str: &str,
    anime_path_str: &str,
    source_file_names: &[String],
    ep_data: &Ep,
) -> Result<(usize, usize), String> { // 返回 (成功视频链接数, 成功字幕链接数) 或 错误信息
    let mut matched_pairs = Vec::new();
    println!("文件与剧集匹配结果:");
    let mut matched_count = 0;
    for (i, file_name) in source_file_names.iter().enumerate() {
        if i < ep_data.name.len() {
            let src_path = Path::new(base_path_str).join(file_name);
            let extension = src_path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
            let dst_name_with_ext = if extension.is_empty() {
                ep_data.name[i].clone()
            } else {
                format!("{}.{}", ep_data.name[i], extension)
            };
            println!("{} -> {}", file_name, dst_name_with_ext);
            matched_pairs.push((file_name.clone(), ep_data.name[i].clone()));
            matched_count += 1;
        }
    }
    println!("成功匹配: {}/{} 个文件", matched_count, source_file_names.len());
    if matched_count < source_file_names.len() {
        println!("警告: 有 {} 个文件没有对应的剧集信息", source_file_names.len() - matched_count);
    }

    if matched_pairs.is_empty() {
        return Err("没有匹配到任何文件和剧集，或者剧集列表为空。".to_string());
    }

    let dest_path = Path::new(anime_path_str);
    if !dest_path.exists() {
        if let Err(e) = std::fs::create_dir_all(dest_path) {
            let err_msg = format!("创建目标目录失败: {}", e);
            println!("{}", err_msg);
            return Err(err_msg);
        }
    }

    let mut subtitle_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(base_path_str) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if let Some(_file_name_str) = path.file_name().and_then(|n| n.to_str()) {
                let is_subtitle = path.extension()
                    .and_then(|ext| ext.to_str())
                    .map_or(false, |ext_str| {
                        let lower_ext = ext_str.to_lowercase();
                        matches!(lower_ext.as_str(), "ass" | "srt" | "ssa" | "sub")
                    });
                if is_subtitle {
                    if let Some(name) = path.file_name().and_then(|os_str| os_str.to_str()) {
                        subtitle_files.push(name.to_string());
                    }
                }
            }
        }
    }
    println!("找到 {} 个字幕文件", subtitle_files.len());
    if !subtitle_files.is_empty() {
        println!("字幕文件列表:");
        for sub_file in &subtitle_files {
            println!("  - {}", sub_file);
        }
    }

    let mut success_link_count = 0;
    let mut subtitle_link_count = 0;

    for (src_file, dst_name) in &matched_pairs {
        let src_path = Path::new(base_path_str).join(src_file);
        let extension_osstr = src_path.extension();
        let extension_str = extension_osstr.and_then(|s| s.to_str());
        let dst_name_with_ext = if extension_str.is_some() && !extension_str.unwrap().is_empty() {
            format!("{}.{}", dst_name, extension_str.unwrap())
        } else {
            dst_name.clone()
        };
        let dest_file_path = dest_path.join(&dst_name_with_ext);

        match std::fs::hard_link(&src_path, &dest_file_path) {
            Ok(_) => {
                println!("成功创建硬链接: {} => {}", src_path.display(), dest_file_path.display());
                success_link_count += 1;
            }
            Err(e) => {
                println!("创建硬链接失败 ({}): {} => {}", e, src_path.display(), dest_file_path.display());
            }
        }

        let video_file_stem = src_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        for sub_file_name_str in &subtitle_files {
            if let Some(sub_file_stem_str) = Path::new(sub_file_name_str).file_stem().and_then(|s| s.to_str()) {
                if sub_file_stem_str.starts_with(video_file_stem) {
                    let sub_src_path = Path::new(base_path_str).join(sub_file_name_str);
                    let sub_extension = sub_src_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    
                    let mut new_sub_file_name_str = dst_name.clone();
                    // 保留原字幕文件名中，视频文件名之后的部分（通常是语言标识等）
                    if let Some(original_sub_tags) = sub_file_stem_str.get(video_file_stem.len()..) {
                        new_sub_file_name_str.push_str(original_sub_tags);
                    }
                    if !sub_extension.is_empty() {
                        new_sub_file_name_str.push('.');
                        new_sub_file_name_str.push_str(sub_extension);
                    }

                    let sub_dest_file_path = dest_path.join(&new_sub_file_name_str);
                    match std::fs::hard_link(&sub_src_path, &sub_dest_file_path) {
                        Ok(_) => {
                            println!("成功创建字幕硬链接: {} => {}", sub_src_path.display(), sub_dest_file_path.display());
                            subtitle_link_count += 1;
                        }
                        Err(e) => {
                            println!("创建字幕硬链接失败 ({}): {} => {}", e, sub_src_path.display(), sub_dest_file_path.display());
                        }
                    }
                }
            }
        }
    }
    Ok((success_link_count, subtitle_link_count))
}

/// 操作完成后重置UI状态
fn reset_ui_state_after_operation(
    mut file_browser: FileBrowser,
    mut search_results_browser: MultiBrowser,
    episode_list_rc: Rc<RefCell<Option<Ep>>>,
    search_results_rc: Rc<RefCell<Option<BgmApi>>>,
    selected_anime_id_rc: Rc<RefCell<Option<String>>>,
) {
    file_browser.clear();
    search_results_browser.clear();
    *episode_list_rc.borrow_mut() = None;
    *search_results_rc.borrow_mut() = None;
    *selected_anime_id_rc.borrow_mut() = None;
}

/// 处理“完成”按钮点击事件的回调
fn handle_done_button_callback(
    mut file_browser: FileBrowser,
    episode_list_rc: Rc<RefCell<Option<Ep>>>,
    base_path_rc: Rc<RefCell<Option<String>>>,
    anime_path_rc: Rc<RefCell<Option<String>>>,
    search_results_rc: Rc<RefCell<Option<BgmApi>>>,
    selected_anime_id_rc: Rc<RefCell<Option<String>>>,
    mut search_results_browser: MultiBrowser,
) {
    println!("注意: 该程序会自动查找并处理与视频文件对应的字幕文件，保留原有语言标识");
    println!("      支持的字幕格式: .ass, .srt, .ssa, .sub");

    // 1. 验证路径
    let (base_path_str, anime_path_str) = match validate_operation_paths(&base_path_rc, &anime_path_rc) {
        Ok(paths) => paths,
        Err(err_msg) => {
            println!("{}", err_msg);
            dialog::message_default(&err_msg);
            return;
        }
    };

    // 2. 获取剧集数据
    let ep_data = match get_episode_data_for_processing(&episode_list_rc) {
        Ok(data) => data,
        Err(err_msg) => {
            // get_episode_data_for_processing 内部已打印日志，此处仅显示对话框
            dialog::message_default(&err_msg);
            return;
        }
    };

    // 3. 收集源文件
    let source_files = collect_source_files_from_browser(&file_browser);
    if source_files.is_empty() {
        let msg = "源文件列表为空，请先选择源文件夹并加载文件。";
        println!("{}", msg);
        dialog::message_default(msg);
        return;
    }

    // 4. 执行重命名和链接
    match perform_file_renaming_and_linking(&base_path_str, &anime_path_str, &source_files, &ep_data) {
        Ok((success_link_count, subtitle_link_count)) => {
            let message = format!("操作完成！\\n成功创建 {} 个视频硬链接。\\n成功创建 {} 个字幕硬链接。", success_link_count, subtitle_link_count);
            println!("{}", message);
            dialog::message_default(&message);

            // 5. 重置UI
            reset_ui_state_after_operation(
                file_browser,
                search_results_browser,
                episode_list_rc,
                search_results_rc,
                selected_anime_id_rc,
            );
        }
        Err(err_msg) => {
            println!("处理文件时发生错误: {}", err_msg);
            dialog::message_default(&format!("处理文件时发生错误: {}", err_msg));
            // 错误发生后，也考虑是否重置部分UI或状态
        }
    }
}

/// 处理文件浏览器的事件 (目前主要用于捕获事件，具体功能未完全实现)
fn handle_file_browser_events(
    _browser: &mut FileBrowser, // 文件浏览器本身 (参数前加 _ 表示可能未使用)
    event: Event,               // 触发的事件
) -> bool { // 返回bool表示事件是否已处理
    match event {
        Event::Push => {
            // 鼠标按下事件
            false // 未处理
        },
        Event::KeyDown => {
            // 键盘按下事件
            false // 未处理
        },
        Event::Drag => {
            // 拖动事件
            false // 未处理
        },
        Event::Released => {
            // 鼠标释放事件 (可能用于拖放结束)
            false // 未处理
        },
        _ => false, // 其他事件不处理
    }
}

// --- UI 状态结构体 ---
#[derive(Clone)]
struct UiState {
    base_path: Rc<RefCell<String>>,
    anime_path: Rc<RefCell<String>>,
    bgm_api: Rc<RefCell<BgmApi>>,
    selected_bgm_id: Rc<RefCell<Option<String>>>,
    ep_data: Rc<RefCell<Option<Ep>>>,
    file_browser: FileBrowser, // 确保这里没有 mut
    search_results_browser: MultiBrowser, // 确保这里没有 mut
    path_display_panel_expanded: Rc<RefCell<bool>>,
    menu_items_panel_expanded: Rc<RefCell<bool>>,
}

impl UiState {
    fn new(
        file_browser: FileBrowser, // 确保这里参数没有 mut
        search_results_browser: MultiBrowser, // 确保这里参数没有 mut
    ) -> Self {
        Self {
            base_path: Rc::new(RefCell::new("".to_string())),
            anime_path: Rc::new(RefCell::new("".to_string())),
            bgm_api: Rc::new(RefCell::new(BgmApi::new())),
            selected_bgm_id: Rc::new(RefCell::new(None)),
            ep_data: Rc::new(RefCell::new(None)), // 初始化为空
            file_browser, // 直接使用传入的 browser
            search_results_browser, // 直接使用传入的 browser
            path_display_panel_expanded: Rc::new(RefCell::new(false)), // 默认不展开
            menu_items_panel_expanded: Rc::new(RefCell::new(false)),   // 默认不展开
        }
    }
}

fn main() {
    let cli_args = CliArgs::parse(); // 解析命令行参数

    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    let mut wind = Window::new(
        100,
        100,
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        "BGM Rename Cuby - 番剧文件批量重命名工具"
    );

    // 使用命令行参数初始化路径 (Rc<RefCell<>> 用于共享可变状态)
    let base_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(cli_args.base_path.clone()));
    let anime_path: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(cli_args.anime_path.clone()));

    // --- 提前定义需要在多个Flex容器中共享或提前配置的UI控件 ---
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
    
    // --- 主垂直Flex布局容器 ---
    let mut main_vertical_flex = Flex::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, "");
    main_vertical_flex.set_type(fltk::group::FlexType::Column); // 垂直排列子元素

    // --- 创建并添加顶部触发器栏 (菜单按钮、路径按钮、搜索区) ---
    let (
        mut menu_trigger_button, // 从 setup 函数获取按钮实例
        mut path_trigger_button, // 从 setup 函数获取按钮实例
        top_triggers_flex
    ) = setup_top_triggers_flex(&btn_done, &search_input, &search_button);
    main_vertical_flex.add(&top_triggers_flex);
    main_vertical_flex.fixed(&top_triggers_flex, MENU_TRIGGER_HEIGHT); // 固定高度

    // --- 创建并添加可展开的菜单项面板 ---
    let mut settings_button = Button::new(0, 0, 0, 30, "📝 注册");
    settings_button.set_tooltip("注册右键菜单到系统");
    let mut unregister_button = Button::new(0, 0, 0, 30, "🗑️ 注销");
    unregister_button.set_tooltip("从系统注销右键菜单");
    let mut about_button = Button::new(0, 0, 0, 30, "📦 关于");
    about_button.set_tooltip("查看项目信息");
    let menu_items_panel_flex = setup_menu_items_panel(&settings_button, &unregister_button, &about_button);
    main_vertical_flex.add(&menu_items_panel_flex); // 添加到主布局，初始高度为0 (隐藏)

    // --- 创建并添加可折叠的路径显示/选择面板 ---
    let path_display_panel_flex = setup_path_display_panel(&btn_choose_base, &btn_choose_anime);
    main_vertical_flex.add(&path_display_panel_flex); // 添加到主布局，初始高度为0 (隐藏)
    
    // --- 创建并添加主内容区域 (文件浏览器和搜索结果浏览器) ---
    let (content_flex, mut left_flex, mut right_flex) = setup_content_area();
    
    // 初始化文件浏览器 (左侧)
    let mut file_browser = FileBrowser::new(0, 0, 0, 0, "");
    file_browser.set_tooltip("源文件夹中的文件列表");
    file_browser.set_selection_color(Color::Yellow);
    file_browser.set_type(fltk::browser::BrowserType::Hold); // 单选模式
    file_browser.set_damage(true); // 确保重绘
    left_flex.add(&file_browser); // 将文件浏览器添加到左侧Flex

    // 初始化搜索结果浏览器 (右侧)
    let mut search_results_browser = MultiBrowser::new(0, 0, 0, 0, "");
    search_results_browser.set_tooltip("Bangumi API 搜索结果");
    search_results_browser.set_selection_color(Color::Yellow);
    search_results_browser.set_type(fltk::browser::BrowserType::Hold); // 单选模式
    right_flex.add(&search_results_browser); // 将搜索结果浏览器添加到右侧Flex

    main_vertical_flex.add(&content_flex); // 内容区域将填充剩余空间
    main_vertical_flex.end();
    
    wind.add(&main_vertical_flex);
    wind.resizable(&main_vertical_flex);
    wind.end();
    wind.show();

    // --- 状态变量 ---
    let is_menu_expanded = Rc::new(RefCell::new(false)); // 菜单面板是否展开
    let is_path_panel_expanded = Rc::new(RefCell::new(false)); // 路径面板是否展开

    // --- 如果通过命令行参数设置了路径，则更新UI并加载文件 ---
    if let Some(cli_base_path_str) = base_path.borrow().as_deref() {
        btn_choose_base.set_label(&shorten_path_for_display(cli_base_path_str, MAX_BUTTON_LABEL_LEN));
        load_files_to_file_browser(cli_base_path_str, &mut file_browser);
        if let Some(extracted_anime_name) = extract_anime_name_from_path(cli_base_path_str) {
            search_input.set_value(&extracted_anime_name);
            println!("从命令行路径 {} 提取到番剧名: {}", cli_base_path_str, extracted_anime_name);
        }
    }
    if let Some(cli_anime_path_str) = anime_path.borrow().as_deref() {
        btn_choose_anime.set_label(&shorten_path_for_display(cli_anime_path_str, MAX_BUTTON_LABEL_LEN));
    }

    // --- 设置菜单栏按钮的回调 ---
    let is_menu_expanded_cb = is_menu_expanded.clone();
    let mut main_vertical_flex_cb_menu = main_vertical_flex.clone();
    let mut menu_items_panel_flex_cb_menu = menu_items_panel_flex.clone();
    let mut wind_cb_menu = wind.clone();
    menu_trigger_button.set_callback(move |_| {
        handle_menu_toggle(
            is_menu_expanded_cb.clone(),
            &mut main_vertical_flex_cb_menu,
            &mut menu_items_panel_flex_cb_menu,
            &mut wind_cb_menu,
        );
    });

    // --- 设置路径栏按钮的回调 ---
    let is_path_panel_expanded_cb = is_path_panel_expanded.clone();
    let mut main_vertical_flex_cb_path = main_vertical_flex.clone();
    let mut path_display_panel_flex_cb_path = path_display_panel_flex.clone();
    let mut wind_cb_path = wind.clone();
    path_trigger_button.set_callback(move |_| {
        handle_path_panel_toggle(
            is_path_panel_expanded_cb.clone(),
            &mut main_vertical_flex_cb_path,
            &mut path_display_panel_flex_cb_path,
            &mut wind_cb_path,
        );
    });
    
    // --- 设置菜单项按钮的回调 (已提取到独立函数) ---
    settings_button.set_callback(|_| handle_register_context_menu());
    unregister_button.set_callback(|_| handle_unregister_context_menu());
    about_button.set_callback(|_| handle_about_button());

    // --- 共享数据状态 ---
    let search_results: Rc<RefCell<Option<BgmApi>>> = Rc::new(RefCell::new(None)); // 存储BGM API搜索结果
    let episode_list: Rc<RefCell<Option<Ep>>> = Rc::new(RefCell::new(None));       // 存储选定番剧的剧集列表
    let selected_anime_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None)); // 存储当前选中的番剧ID

    // --- 设置路径选择按钮 (B和A) 的回调 ---
    let base_path_cb_b = base_path.clone();
    let btn_choose_base_cb_b = btn_choose_base.clone();
    let file_browser_cb_b = file_browser.clone();
    let search_input_cb_b = search_input.clone();
    btn_choose_base.set_callback(move |_| {
        handle_choose_base_path_callback(
            base_path_cb_b.clone(),
            btn_choose_base_cb_b.clone(),
            file_browser_cb_b.clone(),
            search_input_cb_b.clone(),
        );
    });

    let anime_path_cb_a = anime_path.clone();
    let btn_choose_anime_cb_a = btn_choose_anime.clone();
    btn_choose_anime.set_callback(move |_| {
        handle_choose_anime_path_callback(
            anime_path_cb_a.clone(),
            btn_choose_anime_cb_a.clone(),
        );
    });
    
    // --- 设置搜索相关控件的回调 ---
    let search_input_cb_search = search_input.clone();
    let search_results_cb_search = search_results.clone();
    let search_results_browser_cb_search = search_results_browser.clone();
    search_button.set_callback(move |_| {
        handle_search_button_callback(
            search_input_cb_search.clone(),
            search_results_cb_search.clone(),
            search_results_browser_cb_search.clone(),
        );
    });

    let search_input_cb_enter = search_input.clone();
    let search_results_cb_enter = search_results.clone();
    let search_results_browser_cb_enter = search_results_browser.clone();
    search_input.handle(move |_, ev| { // 处理回车键
        if ev == Event::KeyDown && app::event_key() == Key::Enter {
            return handle_search_input_enter_key(
                search_input_cb_enter.clone(),
                search_results_cb_enter.clone(),
                search_results_browser_cb_enter.clone(),
            );
        }
        false
    });

    let search_results_cb_dblclick = search_results.clone();
    let episode_list_cb_dblclick = episode_list.clone();
    let selected_anime_id_cb_dblclick = selected_anime_id.clone();
    search_results_browser.set_callback(move |b| { // b 是 MultiBrowser 本身
        handle_search_results_double_click(
            b,
            search_results_cb_dblclick.clone(),
            episode_list_cb_dblclick.clone(),
            selected_anime_id_cb_dblclick.clone(),
        );
    });
    
    // --- 设置文件浏览器事件处理回调 ---
    // 更新了回调的签名，移除了未使用的 dragged_line_idx_rc 和 marked_line_idx_rc
    file_browser.handle(move |b, ev| {
        handle_file_browser_events(
            b,
            ev,
        )
    });
    
    // --- 设置"完成"按钮的回调 ---
    let file_browser_cb_done = file_browser.clone();
    let episode_list_cb_done = episode_list.clone();
    let base_path_cb_done = base_path.clone();
    let anime_path_cb_done = anime_path.clone();
    let search_results_cb_done = search_results.clone();
    let selected_anime_id_cb_done = selected_anime_id.clone();
    let search_results_browser_cb_done = search_results_browser.clone();
    btn_done.set_callback(move |_| {
        handle_done_button_callback(
            file_browser_cb_done.clone(),
            episode_list_cb_done.clone(),
            base_path_cb_done.clone(),
            anime_path_cb_done.clone(),
            search_results_cb_done.clone(),
            selected_anime_id_cb_done.clone(),
            search_results_browser_cb_done.clone(),
        );
    });

    app.run().unwrap(); // 启动 FLTK 事件循环
}
