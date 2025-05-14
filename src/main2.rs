#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, env, path::PathBuf, rc::Rc};
use fltk::{app, prelude::{WidgetExt, BrowserExt, WidgetBase, GroupExt, InputExt, WindowExt}, window::{WindowType, Window}, browser::MultiBrowser, input::Input, button::Button,dialog, image, group::{Tabs, Group}};
use cuby::{Msg, Bgm};

fn main() {
    // GUI
    let app = app::App::default().with_scheme(app::Scheme::Gtk);
    app::background(221, 221, 221);

    // 使用 Rc<RefCell<>> 包装需要跨线程访问的控件
    let i1_rc = Rc::new(RefCell::new(Input::default()));
    let i2_rc = Rc::new(RefCell::new(Input::default()));
    let br1_rc = Rc::new(RefCell::new(MultiBrowser::default()));
    let br2_rc = Rc::new(RefCell::new(MultiBrowser::default()));
    let b1_rc = Rc::new(RefCell::new(Button::default()));
    let b2_rc = Rc::new(RefCell::new(Button::default()));
    let b3_rc = Rc::new(RefCell::new(Button::default()));
    let b4_rc = Rc::new(RefCell::new(Button::default())); // "<=" button
    let b5_rc = Rc::new(RefCell::new(Button::default())); // "🔎" button
    let b6_rc = Rc::new(RefCell::new(Button::default())); // "Link" button
    let b7_rc = Rc::new(RefCell::new(Button::default())); // "Start" button


    // 主窗口
    let mut w = Window::new(602, 502, 820, 320, "cuby");
    w.set_type(WindowType::Normal);
    w.set_label_size(20);

    // 创建 Tabs 控件，留出底部空间
    let mut tabs = Tabs::new(0, 0, w.width(), w.height() - 60, "");

    // Tab 1: 主界面
    let mut grp1 = Group::new(0, 25, tabs.width(), tabs.height() - 25, "主界面\t\t");
    grp1.begin();
    // 在 Tab 1 中创建主界面控件 (坐标相对于 grp1 的左上角，增加垂直偏移)
    *i1_rc.borrow_mut() = Input::new(50, 10 + 30, 500, 25, None);
    *b1_rc.borrow_mut() = Button::new(10, 10 + 30, 25, 25, "➕");
    *br1_rc.borrow_mut() = MultiBrowser::new(50, 50 + 30, 500, 150, None); // Adjusted height
    let widths = &[260,40,200];
    br1_rc.borrow_mut().set_column_widths(widths);
    br1_rc.borrow_mut().set_column_char('\t');
    *b2_rc.borrow_mut() = Button::new(10, 50 + 30, 25, 25, "➕");
    *b3_rc.borrow_mut() = Button::new(10, 75 + 30, 25, 25, "➖");
    grp1.end();

    // Tab 2: 搜索功能
    let mut grp2 = Group::new(0, 25, tabs.width(), tabs.height() - 25, "搜索\t\t");
    grp2.begin();
    // 在 Tab 2 中创建搜索控件 (坐标相对于 grp2 的左上角，增加垂直偏移)
    *i2_rc.borrow_mut() = Input::new(10, 10 + 30, 160, 25, None);
    *b5_rc.borrow_mut() = Button::new(180, 10 + 30, 25, 25, "🔎");
    *br2_rc.borrow_mut() = MultiBrowser::new(10, 50 + 30, 200, 150, None); // Adjusted position and height
    *b4_rc.borrow_mut() = Button::new(220, 50 + 30, 40, 25, "<="); // Moved from Tab 1
    b4_rc.borrow_mut().set_label_size(20);
    grp2.end();

    tabs.end(); // 结束 Tabs 组

    // 底部按钮 (放在 Tabs 下方)
    *b6_rc.borrow_mut() = Button::new(600, 265, 70, 50, "Link");
    *b7_rc.borrow_mut() = Button::new(700, 265, 100, 50, "Start");


    // 主窗口绘制结束
    w.end();
    // 显示主窗口
    w.show();
    // 设置窗口图标
    let a = cuby::get_png();
    let img = image::PngImage::from_data(a.data.as_ref()).unwrap();
    w.set_icon(Some(img));

    // 消息传递 (更新 emit 调用以使用 Rc<RefCell<>>)
    let (s,r) = app::channel::<Msg>();
    b1_rc.borrow_mut().emit(s, Msg::AddOut);
    b2_rc.borrow_mut().emit(s, Msg::AddIn);
    b3_rc.borrow_mut().emit(s, Msg::RemoveOut);
    b4_rc.borrow_mut().emit(s, Msg::Set); // b4 is now in Tab 2
    b5_rc.borrow_mut().emit(s, Msg::Search);
    b6_rc.borrow_mut().emit(s, Msg::Link); // b6 is now outside tabs
    b7_rc.borrow_mut().emit(s, Msg::Start); // b7 is now outside tabs

    // 授权码
    let access_token = env::var("BGM_RC_ACCESS_TOKEN").ok().unwrap_or("NULL".to_string());
    // 临时数据，堆占用内存，或许可以用数据库代替？
    let in_paths:Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));
    let bgms:Rc<RefCell<Vec<(String,String)>>> = Rc::new(RefCell::new(Vec::new()));
    let search_bgms:Rc<RefCell<Vec<Bgm>>> = Rc::new(RefCell::new(Vec::new()));

    // 消息接收 (更新消息处理逻辑以使用 Rc<RefCell<>>)
    while app.wait() {
        if let Some(msg) = r.recv(){
            match msg {
                Msg::AddOut => {
                    // 选择&&显示输出文件夹
                    i1_rc.borrow_mut().set_value(cuby::select_folder().to_str().unwrap());

                }
                Msg::AddIn => {
                    // 选择输入文件夹
                    let p = cuby::select_folder();
                    let p = match p.to_str() {
                        None => continue,
                        Some(_) => p,
                    };
                    // 显示输入文件夹
                    br1_rc.borrow_mut().add(&p.to_str().unwrap().to_string());
                    // 存储数据
                    in_paths.borrow_mut().push(p);

                }
                Msg::RemoveOut => {
                    // 删除选择的行
                    let line = br1_rc.borrow().value();
                    br1_rc.borrow_mut().remove(line);
                    // 删除数据
                    in_paths.borrow_mut().remove((line as usize)-1);

                }
                Msg::Set => {
                    // 错误处理，防止选择番剧为空。
                    match br2_rc.borrow().value() > 0 {
                        true => (),
                        false => {
                            dialog::message_default("未选择番剧！");
                            continue;
                        }
                    }

                    // 将选择的番剧名和id存储到bgms
                    let u = (br2_rc.borrow().value() as usize)-1;
                    let s = (search_bgms.borrow()[0].id[u].clone(),search_bgms.borrow()[0].name[u].clone());
                    bgms.borrow_mut().push(s.clone());
                    // 错误处理，判断对应的输入文件夹是否存在。
                    let b = bgms.borrow().len() > in_paths.borrow().len();// 因为引用规则限制，必须使用中间变量。
                    // 若不存在，删除添加的数据，并跳出循环。
                    match b  {
                        true => {
                            dialog::message_default("未选择番剧文件夹！");
                            bgms.borrow_mut().pop();
                            continue;
                        },
                        false => (),
                    };
                    // 先清除显示内容
                    search_bgms.borrow_mut().clear();
                    br1_rc.borrow_mut().clear();
                    br2_rc.borrow_mut().clear();
                    // 然后将从br2中选择的番剧名结合输入文件夹路径显示到br1列表
                    let len = bgms.borrow().len();
                    for i in 0..len{
                        let s = format!("{}\t => \t{}",in_paths.borrow()[i].to_str().unwrap(),bgms.borrow()[i].1);
                        br1_rc.borrow_mut().add(&s);
                    }
                    // 显示未对应番剧的输入文件夹路径
                    for s in in_paths.borrow().iter().skip(len) {
                        br1_rc.borrow_mut().add(&s.to_str().unwrap());
                    }

                }
                Msg::Search => {
                    // 获取搜索关键词
                    let keywords = i2_rc.borrow().value();
                    // 获取搜索到的番剧名和id
                    let bgms = Bgm::new().get(&keywords);
                    // 存储数据，并显示。
                    search_bgms.borrow_mut().clear();
                    search_bgms.borrow_mut().push(bgms.clone());
                    br2_rc.borrow_mut().clear();
                    for s in bgms.name {
                        br2_rc.borrow_mut().add(&s)
                    }
                }
                Msg::Link => {
                    // 获取输出文件夹路径
                    let path = cuby::set_out_path(&i1_rc.borrow().value());
                    // 错误处理，防止输出文件夹路径为空。
                    let out_path = match path.to_str().unwrap().is_empty() {
                        true => {
                            dialog::message_default("未设置输出文件夹！");
                            continue;
                        },
                        false => path,
                    };
                    // 获取输入文件夹路径
                    let v_inpaths:Vec<PathBuf> = in_paths.borrow().clone();
                    // 获取番剧名，与输出文件夹路径结合，获得每个番剧的输出文件夹。
                    let v_names:Vec<String> = bgms.borrow().clone().into_iter().map(|x| x.1).collect();
                    let v_outpaths = cuby::get_out_file_paths(out_path, v_names);
                    // 新建每个番剧的输出文件夹
                    cuby::mkdir(&v_outpaths);
                    // 开始硬链接视频文件和复制字幕
                    cuby::just_link(v_inpaths,v_outpaths);

                    // 清理数据
                    br1_rc.borrow_mut().clear();
                    in_paths.borrow_mut().clear();
                    bgms.borrow_mut().clear();

                }
                Msg::Start => {
                    // 获取输出文件夹路径
                    let path = cuby::set_out_path(&i1_rc.borrow().value());
                    // 错误处理，防止输出文件夹路径为空。
                    let path = match path.to_str().unwrap().is_empty() {
                        true => {
                            dialog::message_default("未设置输出文件夹！");
                            continue;
                        },
                        false => path,
                    };
                    // 获取输入文件夹路径
                    let v_inpaths = in_paths.borrow().clone();
                    // 获取番剧名和id
                    let v_bgms= bgms.borrow().clone();
                    // 获取每个番剧文件夹里的每个截止到集数名的路径（无后缀名）
                    let (v_ep,v_outpaths) = cuby::out_ep_path(path, v_bgms,&access_token);
                    // 新建每个番剧的输出文件夹
                    cuby::mkdir(&v_outpaths);
                    // 开始硬链接、复制和重命名。
                    cuby::link_rename(v_inpaths , v_outpaths, v_ep);
                    // 清理数据。
                    br1_rc.borrow_mut().clear();
                    in_paths.borrow_mut().clear();
                    bgms.borrow_mut().clear();
                }
           }
        }
    }
}
