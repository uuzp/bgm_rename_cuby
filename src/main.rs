use fltk::{
    app,
    browser::FileBrowser,
    button::Button,
    dialog::FileDialog,
    enums::{Event, Color},
    group::Flex,
    input::Input,
    prelude::*,
    text::{TextBuffer, TextDisplay},
    window::Window,
};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;
const HALF_WIDTH: i32 = WINDOW_WIDTH / 2;

fn main() {
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    let mut wind = Window::new(
        100,
        100,
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        "File Explorer and Search",
    );

    let mut main_flex = Flex::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, "");
    main_flex.set_type(fltk::group::FlexType::Row); // 水平排列

    // 左半部分
    let mut left_flex = Flex::new(0, 0, HALF_WIDTH, WINDOW_HEIGHT, "");
    left_flex.set_type(fltk::group::FlexType::Column); // 垂直排列
    left_flex.set_margin(5); // 添加一些边距

    let mut button_row_flex = Flex::new(0, 0, 0, 30, ""); // 高度固定，宽度由 left_flex 控制
    button_row_flex.set_type(fltk::group::FlexType::Row);
    let mut btn_choose_folder = Button::new(0, 0, 0, 0, "Choose Folder"); // 大小由 Flex 控制
    let _btn_done = Button::new(0, 0, 0, 0, "完成"); // 大小由 Flex 控制
    button_row_flex.end();
    left_flex.fixed(&button_row_flex, 30); // 固定按钮行的高度

    let mut file_browser = FileBrowser::new(0, 0, 0, 0, ""); // 大小由 Flex 控制
    file_browser.set_selection_color(Color::Yellow);
    file_browser.set_type(fltk::browser::BrowserType::Hold); // 单选模式
    file_browser.set_damage(true); // Ensure redraws

    // For drag-and-drop reordering
    let dragged_line_index: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
    
    let d_idx_for_handle = dragged_line_index.clone();

    file_browser.handle(move |b, ev| {
        let mut d_idx = d_idx_for_handle.borrow_mut();
        
        match ev {
            Event::Push => {
                if app::event_clicks() {
                    // 获取FLTK FileBrowser自己检测到的行号
                    let line_num = b.value();
                    if line_num > 1 { // 忽略头部行
                        *d_idx = Some(line_num);
                        
                        // 打印选中行的信息用于调试
                        if let Some(text) = b.text(line_num) {
                            println!("开始拖拽第 {} 行: \"{}\"", line_num, text);
                        }
                        
                        return true;
                    }
                    *d_idx = None;
                }
                false
            },
            Event::Drag => {
                if d_idx.is_some() {
                    // 拖拽期间，让行选中效果跟随鼠标位置
                    // 让FLTK自行处理鼠标位置与行号的对应关系
                    let mouse_y = app::event_y();
                    let widget_y = b.y();
                    let widget_h = b.h();
                    
                    if mouse_y >= widget_y && mouse_y < widget_y + widget_h {
                        // 传递事件让FileBrowser内部自行检测行
                        b.handle_event(Event::Move);
                        
                        // 获取当前鼠标悬停在哪一行
                        let drag_to_line = b.value();
                        
                        // 打印当前拖拽位置的行号
                        if drag_to_line > 0 {
                            println!("拖拽到第 {} 行", drag_to_line);
                        }
                    }
                    
                    return true;
                }
                false
            },
            Event::Released => {
                if let Some(from_line) = d_idx.take() {
                    // 获取释放位置的行号
                    let to_line = b.value();
                    
                    println!("从第 {} 行移动到第 {} 行", from_line, to_line);
                    
                    // 执行移动操作
                    if to_line > 1 && to_line != from_line {
                        // 在移动前打印源和目标行的内容
                        if let Some(from_text) = b.text(from_line) {
                            println!("源行 {} 内容: \"{}\"", from_line, from_text);
                        }
                        
                        if let Some(to_text) = b.text(to_line) {
                            println!("目标行 {} 内容: \"{}\"", to_line, to_text);
                        }
                        
                        // 根据您的测试，参数顺序是 (to, from)
                        b.move_item(to_line, from_line);
                        
                        // 高亮目标行
                        b.select(to_line);
                        
                        // 打印移动后的内容
                        if let Some(new_text) = b.text(to_line) {
                            println!("移动后位置 {} 内容: \"{}\"", to_line, new_text);
                        }
                    }
                    
                    return true;
                }
                false
            },
            _ => false,
        }
    });

    let mut file_browser_clone = file_browser.clone();
    btn_choose_folder.set_callback(move |_| {
        let mut dialog = FileDialog::new(fltk::dialog::FileDialogType::BrowseDir);
        dialog.show();
        let chosen_path = dialog.filename();
        if !chosen_path.as_os_str().is_empty() {
            let path = Path::new(&chosen_path);
            if path.is_dir() {
                file_browser_clone.clear(); // 清空浏览器
                if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                    file_browser_clone.add(&format!("├─ {}", folder_name)); // 修改文件夹名称格式
                }

                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let file_path = entry.path();
                            if file_path.is_file() {
                                if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                                    match ext.to_lowercase().as_str() {
                                        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => {
                                            if let Some(file_name) =
                                                file_path.file_name().and_then(|n| n.to_str())
                                            {
                                                file_browser_clone.add(&format!("├─── {}", file_name)); // 修改文件名称格式，移除空格，延长横线
                                            }
                                        }
                                        _ => (),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    left_flex.end();
    main_flex.add(&left_flex); // 将左侧 Flex 添加到主 Flex

    // 右半部分
    let mut right_flex = Flex::new(HALF_WIDTH, 0, HALF_WIDTH, WINDOW_HEIGHT, "");
    right_flex.set_type(fltk::group::FlexType::Column);
    right_flex.set_margin(5);

    let mut search_input = Input::new(0, 0, 0, 30, ""); // 高度固定，宽度由 Flex 控制
    search_input.set_tooltip("Enter search query here");
    right_flex.fixed(&search_input, 30); // 固定搜索框高度

    let mut search_results_display = TextDisplay::new(0, 0, 0, 0, ""); // 大小由 Flex 控制
    let buffer = TextBuffer::default();
    search_results_display.set_buffer(buffer);
    search_results_display.set_text_size(14);
    search_results_display.wrap_mode(fltk::text::WrapMode::AtBounds, 0);

    right_flex.end();
    main_flex.add(&right_flex); // 将右侧 Flex 添加到主 Flex

    main_flex.end();
    wind.add(&main_flex); // 将主 Flex 添加到窗口

    wind.resizable(&main_flex); // 使主 Flex 可调整大小
    wind.end();
    wind.show();

    app.run().unwrap();
}
