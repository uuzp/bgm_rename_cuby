#![windows_subsystem = "windows"]
use fltk::{app, browser, button::Button, dialog, input, prelude::*, window::Window};
use jsonpath_lib as jsonpath;
use open;
use reqwest::{self, header::USER_AGENT};
use serde_json;

use std::{
    cell::RefCell,
    cmp::Ordering,
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
};

struct BgmData {
    name: Vec<String>,
    url: Vec<String>,
}
struct Ep {
    name: Vec<String>,
    year: String,
    //    total: i8,
}
struct Files {
    video: Vec<PathBuf>,
    sub: Vec<PathBuf>,
    sub_sc: Vec<PathBuf>,
    sub_tc: Vec<PathBuf>,
}

impl BgmData {
    fn new(keywords: String) -> Result<BgmData, Box<dyn Error>> {
        let url = format!(
            "https://api.bgm.tv/search/subject/{}?type=2&responseGroup=small",
            keywords
        );
        let json_body: serde_json::Value = reqwest::blocking::get(url)?.json()?;

        let select = |x: &str| {
            let mut v = Vec::new();

            for s in jsonpath::selector(&json_body)(x).unwrap() {
                v.push(s.as_str().unwrap().to_string());
            }
            v
        };

        let name_cn = select("$.list.*.name_cn");
        let url = select("$.list.*.url");
        let name = {
            let mut v = Vec::new();
            for i in 0..url.len() {
                let s = match name_cn[i].len() {
                    0 => jsonpath::selector(&json_body)("$.list.*.name").unwrap()[i]
                        .as_str()
                        .unwrap()
                        .to_string(),
                    _ => name_cn[i].to_owned(),
                };
                v.push(s);
            }
            v
        };

        //TODO 或许还可以优化逻辑，争取只创建一次vec。

        Ok(BgmData { name, url })
    }
}
impl Ep {
    fn new(bgm_id: &str) -> Result<Ep, Box<dyn Error>> {
        let client = reqwest::blocking::Client::new();
        let url = format!(
            "https://api.bgm.tv/v0/episodes?subject_id={}&type=0&limit=100&offset=0",
            bgm_id
        );

        let json_body: serde_json::Value = client
            .get(url)
            .header(USER_AGENT, "bgm_rename_rust_cuby")
            .send()?
            .json()?;
        let mut ep_list = Vec::new();
        let year = jsonpath::select(&json_body, "$.data.*.airdate").unwrap();

        let year = year[0].as_str().unwrap()[0..4].to_string();
        // let total = jsonpath::select(&json_body, "$.total").unwrap();
        // let total = total[0].as_i64().unwrap();
        // let total = total as usize;

        let mut epn = Vec::new();
        for s in jsonpath::selector(&json_body)("$.data.*.sort").unwrap() {
            epn.push(s.as_i64().unwrap());
        }
        for s in jsonpath::selector(&json_body)("$.data.*.name_cn").unwrap() {
            ep_list.push(s.as_str().unwrap());
        }
        let mut name = vec![];
        for i in 0..ep_list.len() {
            let ep = match epn[i].cmp(&10) {
                Ordering::Less => "ep0",
                _ => "ep",
            };
            let epl = format!("{}{} - {}", ep, epn[i], ep_list[i]);
            name.push(epl);
        }
        let ep = Ep { name, year };

        Ok(ep)
    }
}
impl Files {
    fn new(root: &Path) -> io::Result<Files> {
        let mut sub = vec![];
        let mut sub_sc = vec![];
        let mut sub_tc = vec![];
        let mut video = vec![];
        let videos = "mp4 mkv";
        let subs = "ass str";
        let sc = "chs sc gb";
        let tc = "cht tc big5";

        for path in fs::read_dir(root).unwrap() {
            let path = path.unwrap().path();

            let ts1 = match path.to_str().unwrap().rsplit_once(".") {
                Some(x) => x,
                None => ("None", "None"),
            };

            let ts = match ts1.0.rsplit_once(".") {
                Some((_x, y)) => y,
                None => "None",
            };
            let suf = ts1.1;

            match subs.contains(suf) {
                true => match sc.contains(&ts.to_lowercase()) {
                    true => sub_sc.push(path.to_owned()),
                    false => match tc.contains(&ts.to_lowercase()) {
                        true => sub_tc.push(path.to_owned()),
                        false => sub.push(path.to_owned()),
                    },
                },
                false => match videos.contains(suf) {
                    true => video.push(path.to_owned()),
                    false => (),
                },
            }
        }
        let files = Files {
            video,
            sub,
            sub_sc,
            sub_tc,
        };

        Ok(files)
    }
}

fn select_folder() -> String {
    let mut dialog = dialog::NativeFileChooser::new(dialog::NativeFileChooserType::BrowseDir);
    dialog.show();
    dialog.filename().as_path().display().to_string()
}
fn main() -> Result<(), Box<dyn Error>> {
    let app = app::App::default();
    let mut window = Window::new(100, 100, 850, 600, "Cuby");

    let sf = input::Input::new(120, 110, 700, 25, "");
    let sf = Rc::new(RefCell::new(sf));
    let ef = input::Input::new(120, 160, 700, 25, "");
    let ef = Rc::new(RefCell::new(ef));

    let mut sf_btn = Button::new(20, 100, 80, 40, "源路径");
    let mut ef_btn = Button::new(20, 150, 80, 40, "输出路径");

    let search = input::Input::new(100, 230, 500, 25, "");
    let mut search_btn = Button::new(650, 220, 80, 40, "搜索");

    let mut list_title = browser::Browser::new(20, 280, 700, 20, "");
    list_title.set_column_widths(&[50, 10, 400, 10, 100]);
    list_title.set_column_char('\t');
    list_title.add("@c序号\t@c|\t@c番剧名\t@c|\t@cURL");
    let mut list = browser::MultiBrowser::new(20, 300, 700, 280, "");

    list.set_column_widths(&[50, 10, 400, 10, 100]);
    list.set_column_char('\t');

    let list = Rc::new(RefCell::new(list));

    let mut url_btn = Button::new(750, 400, 80, 40, "前往URL");
    let mut start_btn = Button::new(750, 500, 80, 40, "开始");

    window.end(); //窗口绘制停止
    window.show();
    let sf1 = Rc::clone(&sf);
    let ef1 = Rc::clone(&ef);
    sf_btn.set_callback(move |_| {
        sf1.borrow_mut().set_value(&select_folder());
    });
    ef_btn.set_callback(move |_| {
        ef1.borrow_mut().set_value(&select_folder());
    });

    search_btn.set_callback(move |_| {
        //dialog::message_default(&search.value());
        let keywords = search.value();

        let bgm = BgmData::new(keywords).unwrap();

        list.borrow_mut().clear();
        for i in 0..bgm.name.len() {
            // let s = match bgm.name_cn[i].len() {
            //     0 => format!("@c{}\t|\t{}\t|\t{}", i, bgm.name[i], bgm.url[i]),
            //     _ => format!("@c{}\t|\t{}\t|\t{}", i, bgm.name_cn[i], bgm.url[i]),
            // };
            let s = format!("@c{}\t|\t{}\t|\t{}", i, bgm.name[i], bgm.url[i]);
            list.borrow_mut().add(&s);
        }

        let sf2 = Rc::clone(&sf);
        let ef2 = Rc::clone(&ef);

        let list1 = Rc::clone(&list);
        let list2 = Rc::clone(&list);
        let bgm_url = bgm.url.clone();
        //URL按钮
        url_btn.set_callback(move |_| {
            let url = &bgm_url[(list1.borrow().value() - 1) as usize];
            open::that(url).unwrap();
        });
        //开始按钮
        start_btn.set_callback(move |_| {
            let key = (list2.borrow().value() - 1) as usize;

            let bgm_name = &bgm.name[key];
            let str = format!("确认<{}>无误？", bgm_name);

            let choice = dialog::choice2_default(&str, "No", "Yes", "");

            if let Some(1) = choice {
                let bgm_id = &bgm.url[key].rsplit_once("/").unwrap().1;

                let ep = Ep::new(bgm_id).unwrap();

                let val1 = sf2.borrow().value();
                let path1 = Path::new(&val1);
                let mut path2 = PathBuf::new();

                let val2 = ef2.borrow().value();

                path2.push(val2);
                let bgm_name = bgm_name.replace("/", "／");
                let bgm_name = bgm_name.replace(":", "：");
                path2.push(format!("{} ({})", bgm_name, ep.year));
                let path2 = Path::new(&path2);

                match fs::create_dir(path2) {
                    Ok(floder) => floder,
                    Err(err) => {
                        let err = format!("{}", err);
                        dialog::message_default(&err);
                    }
                };
                assert!(env::set_current_dir(path2).is_ok()); //设置工作目录

                let files = Files::new(path1).unwrap();

                //设定一个开始的剧集数,一个输入框、
                let start = 0;

                match files.video.len() {
                    0 => (),
                    _ => {
                        let max = files.video.len();
                        for i in start..max {
                            let video = format!(
                                "{}.{}",
                                ep.name[i],
                                &files.video[i].extension().unwrap().to_string_lossy()
                            );
                            assert!(fs::hard_link(&files.video[i], video).is_ok());
                        }
                    }
                }

                match files.sub.len() {
                    0 => (),
                    _ => {
                        let max = files.sub.len();
                        for i in start..max {
                            let sub = format!(
                                "{}.{}",
                                ep.name[i],
                                &files.sub[i].to_str().unwrap().split_once(".").unwrap().1
                            );
                            assert!(fs::copy(&files.sub[i], sub).is_ok());
                        }
                    }
                }

                match files.sub_sc.len() {
                    0 => (),
                    _ => {
                        let max = files.sub_sc.len();
                        for i in start..max {
                            let sub_sc = format!(
                                "{}.{}",
                                ep.name[i],
                                &files.sub_sc[i].to_str().unwrap().split_once(".").unwrap().1
                            );
                            assert!(fs::copy(&files.sub_sc[i], sub_sc).is_ok());
                        }
                    }
                }

                match files.sub_tc.len() {
                    0 => (),
                    _ => {
                        let max = files.sub_tc.len();
                        for i in start..max {
                            let sub_tc = format!(
                                "{}.{}",
                                ep.name[i],
                                &files.sub_tc[i].to_str().unwrap().split_once(".").unwrap().1
                            );
                            assert!(fs::copy(&files.sub_tc[i], sub_tc).is_ok());
                        }
                    }
                }

                dialog::message_default("成功！");
            };
        });
    });

    app.run().unwrap();
    Ok(())
}
