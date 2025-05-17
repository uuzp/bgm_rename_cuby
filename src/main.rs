#![windows_subsystem = "windows"] // 禁止在 Windows 上显示控制台窗口

// --- 模块导入 ---
mod bangumi_api;
mod io;
mod ui;
mod ui_core;
mod ui_reg_callbacks;

// --- 使用声明 ---
use fltk::{
    app, 
    prelude::*,
}; 
use clap::Parser;

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
