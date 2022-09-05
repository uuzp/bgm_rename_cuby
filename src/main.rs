#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, rc::Rc, path::PathBuf};

use fltk::{app, prelude::{WidgetExt, BrowserExt, WidgetBase, GroupExt, InputExt, WindowExt}, window::{WindowType, Window}, browser::MultiBrowser, input::Input, button::Button,dialog, image};
use cuby::{Msg, Bgm};


fn main() {
    // GUI
    let app = app::App::default();
    // 主窗口
    let mut w = Window::new(602, 502, 820, 320, "cuby");
    w.set_type(WindowType::Normal);
    w.set_label_size(20);
    // 番剧源文件夹路径列表
    let mut br1 = MultiBrowser::new(50, 50, 500, 260, None);
    let widths = &[260,40,200];
    br1.set_column_widths(widths);
    br1.set_column_char('\t');
    // 搜索列表
    let mut br2 = MultiBrowser::new(600, 50, 200, 200, None);
    // 目标文件夹路径
    let mut i1 = Input::new(50, 10, 500, 25, None);
    // 搜索栏
    let i2 = Input::new(600, 10, 160, 25, None);
    // 按钮
    let mut b1 = Button::new(10, 10, 25, 25, "➕");
    let mut b2 = Button::new(10, 50, 25, 25, "➕");
    let mut b3 = Button::new(10, 75, 25, 25, "➖");
    let mut b4 = Button::new(555, 50, 40, 25, "<=");
    b4.set_label_size(20);
    let mut b5 = Button::new(775, 10, 25, 25, "🔎");
    let mut b6 = Button::new(600, 260, 70, 50, "Link");
    let mut b7 = Button::new(700, 260, 100, 50, "Start");
    // 主窗口绘制结束
    w.end();
    // 显示主窗口
    w.show();
    // 设置窗口图标
    let a = cuby::get_png();
    let img = image::PngImage::from_data(a.data.as_ref()).unwrap();
    w.set_icon(Some(img));
    
    // 消息传递
    let (s,r) = app::channel::<Msg>();
    b1.emit(s, Msg::AddOut);
    b2.emit(s, Msg::AddIn);
    b3.emit(s, Msg::RemoveOut);
    b4.emit(s, Msg::Set);
    b5.emit(s, Msg::Search);
    b6.emit(s, Msg::Link);
    b7.emit(s, Msg::Start);
    

    // 临时数据，堆占用内存，或许可以用数据库代替？
    let in_paths:Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));
    let bgms:Rc<RefCell<Vec<(String,String)>>> = Rc::new(RefCell::new(Vec::new()));
    let search_bgms:Rc<RefCell<Vec<Bgm>>> = Rc::new(RefCell::new(Vec::new()));
    
    // 消息接收
    while app.wait() {
        if let Some(msg) = r.recv(){
            match msg {
                Msg::AddOut => {
                    // 选择&&显示输出文件夹
                    i1.set_value(cuby::select_folder().to_str().unwrap());

                }
                Msg::AddIn => {
                    // 选择输入文件夹
                    let p = cuby::select_folder();
                    let p = match p.to_str() {
                        None => continue,
                        Some(_) => p,
                    };
                    // 显示输入文件夹 
                    br1.add(&p.to_str().unwrap().to_string());
                    // 存储数据
                    in_paths.borrow_mut().push(p);

                }
                Msg::RemoveOut => {
                    // 删除选择的行
                    let line = br1.value();
                    br1.remove(line);
                    // 删除数据
                    in_paths.borrow_mut().remove((line as usize)-1);

                }
                Msg::Set => {
                    // 错误处理，防止选择番剧为空。
                    match br2.value() > 0 {
                        true => (),
                        false => {
                            dialog::message_default("未选择番剧！");
                            continue;
                        }
                    }
                    
                    // 将选择的番剧名和id存储到bgms
                    let u = (br2.value() as usize)-1;
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
                    br1.clear();
                    br2.clear();
                    // 然后将从br2中选择的番剧名结合输入文件夹路径显示到br1列表
                    let len = bgms.borrow().len();
                    for i in 0..len{
                        let s = format!("{}\t => \t{}",in_paths.borrow()[i].to_str().unwrap(),bgms.borrow()[i].1);
                        br1.add(&s);
                    }
                    // 显示未对应番剧的输入文件夹路径
                    for s in in_paths.borrow().iter().skip(len) {
                        br1.add(&s.to_str().unwrap());
                    }

                }
                Msg::Search => {
                    // 获取搜索关键词
                    let keywords = i2.value();
                    // 获取搜索到的番剧名和id
                    let bgms = Bgm::new().get(&keywords);
                    // 存储数据，并显示。
                    search_bgms.borrow_mut().clear();
                    search_bgms.borrow_mut().push(bgms.clone());
                    br2.clear();
                    for s in bgms.name {
                        br2.add(&s)
                    }

                    

                }
                Msg::Link => {
                    //TODO v3是番剧名得和v1的输出路径合并，v2是输入文件夹，得读取每个文件夹里的视频和字幕文件，用for循环。然后 link v1+v2 => v3+Ep.name。
                    let path = cuby::set_out_path(&i1.value());
                    // 错误处理，防止输出文件夹路径为空
                    let path = match path.to_str().unwrap().is_empty() {
                        true => {
                            dialog::message_default("未设置输出文件夹！");
                            continue;
                        },
                        false => path,
                    };
                    println!("path:\n{:?}\n",path);
                    let v_names:Vec<String> = bgms.borrow().iter().map(|x| x.1.clone() ).collect();
                    let v_inpaths:Vec<PathBuf> = in_paths.borrow().iter().map(|x| x.clone()).collect();
                    let v_outpaths = cuby::mkoutpaths(path, v_names);
                    cuby::mkdir(&v_outpaths);
                    cuby::name_extension(v_inpaths,v_outpaths);
                   

                    br1.clear();
                    in_paths.borrow_mut().clear();
                    bgms.borrow_mut().clear();

                }
                Msg::Start => {
                    let path = cuby::set_out_path(&i1.value());
                    // 错误处理，防止输出文件夹路径为空
                    let path = match path.to_str().unwrap().is_empty() {
                        true => {
                            dialog::message_default("未设置输出文件夹！");
                            continue;
                        },
                        false => path,
                    };
                    let v_names= bgms.borrow().clone();
                   // let v_inpaths:Vec<PathBuf> = v2.borrow().iter().map(|x| x.clone()).collect();
                    let v_inpaths = in_paths.borrow().clone();
                    let (v_ep,v_outpaths) = cuby::out_ep_path(path, v_names);
                    cuby::tolink(v_inpaths , v_outpaths, v_ep);
                    br1.clear();
                    in_paths.borrow_mut().clear();
                    bgms.borrow_mut().clear();
                }
                
            }
        }
        
    }
   
    
}
