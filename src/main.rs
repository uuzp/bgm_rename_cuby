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
    

    // 存储数据，v1存储path-out，v2存储path-in's,v3存储bgms
   // let v1:Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));
    let v2:Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));
    let v3:Rc<RefCell<Vec<(String,String)>>> = Rc::new(RefCell::new(Vec::new()));

    // 临时数据
    let v4:Rc<RefCell<Vec<Bgm>>> = Rc::new(RefCell::new(Vec::new()));
    
    // 消息接收
    while app.wait() {
        if let Some(msg) = r.recv(){
            match msg {
                Msg::AddOut => {
                    i1.set_value(cuby::select_folder().to_str().unwrap());

                }
                Msg::AddIn => {
                    let p = cuby::select_folder();
                    let p = match p.to_str() {
                        None => continue,
                        Some(_) => p,
                    }; 
                    br1.add(&p.to_str().unwrap().to_string());
                    v2.borrow_mut().push(p);

                }
                Msg::RemoveOut => {
                    let line = br1.value();
                    br1.remove(line);
                    v2.borrow_mut().remove((line as usize)-1);

                }
                Msg::Set => {
                    match br2.value() > 0 {
                        true => (),
                        false => {
                            dialog::message_default("未选择番剧！");
                            continue;
                        }
                    }
                    let n = (br2.value() as usize)-1;
                    let m = (v4.borrow()[0].id[n].clone(),v4.borrow()[0].name[n].clone());
                    v3.borrow_mut().push(m.clone());
                    let eq = v3.borrow().len() > v2.borrow().len();
                    match eq  {
                        true => {
                            dialog::message_default("未选择番剧文件夹！");
                            v3.borrow_mut().pop();
                            continue;
                        },
                        false => (),
                    };
                    v4.borrow_mut().clear();
                    br1.clear();
                    br2.clear();
                    let len = v3.borrow().len();
                    for i in 0..len{
                        let s = format!("{}\t => \t{}",v2.borrow()[i].to_str().unwrap(),v3.borrow()[i].1);
                        br1.add(&s);
                    }

                    for s in v2.borrow().iter().skip(len) {
                        br1.add(&s.to_str().unwrap());
                    }

                }
                Msg::Search => {
                    let keywords = i2.value();
                    let bgm = Bgm::new().get(&keywords);
                    
                    v4.borrow_mut().clear();
                    v4.borrow_mut().push(bgm.clone());
                    br2.clear();
                    for s in bgm.name {
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
                    let v_names:Vec<String> = v3.borrow().iter().map(|x| x.1.clone() ).collect();
                    let v_inpaths:Vec<PathBuf> = v2.borrow().iter().map(|x| x.clone()).collect();
                    let v_outpaths = cuby::mkoutpaths(path, v_names);
                    cuby::mkdir(&v_outpaths);
                    cuby::name_extension(v_inpaths,v_outpaths);
                   

                    br1.clear();
                    v2.borrow_mut().clear();
                    v3.borrow_mut().clear();

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
                    let v_names= v3.borrow().clone();
                   // let v_inpaths:Vec<PathBuf> = v2.borrow().iter().map(|x| x.clone()).collect();
                    let v_inpaths = v2.borrow().clone();
                    let (v_ep,v_outpaths) = cuby::out_ep_path(path, v_names);
                    cuby::tolink(v_inpaths , v_outpaths, v_ep);
                    br1.clear();
                    v2.borrow_mut().clear();
                    v3.borrow_mut().clear();
                }
                
            }
        }
        
    }
   
    
}
