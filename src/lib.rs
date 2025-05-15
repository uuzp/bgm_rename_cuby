use std::{path::{PathBuf, Path}, fs, ffi::OsStr};

use fltk::dialog;
use anitomy::{Anitomy, ElementCategory};
use rust_embed::{RustEmbed, EmbeddedFile};

use miniserde::{Deserialize, Serialize};
use miniserde::json;
use minreq; // 添加 minreq 导入

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
    // total: u64, // 移除未使用的字段
}

#[derive(Deserialize, Serialize)]
struct Episode {
    airdate: String,
    sort: u64,
    name: String,
    name_cn: String,
}

#[derive(Debug, Clone, Copy)]
pub enum  Msg {
    AddIn,
    AddOut,
    RemoveOut,
    Search,
    Set,
    Link,
    Start,
}
#[derive(Debug, Clone)]
pub struct Bgm {
    pub name: Vec<String>,
    pub id: Vec<String>,
}

impl Bgm {
    pub fn new() -> Self {
        Self {
            name: Vec::new(),
            id: Vec::new(),
        }
    }
    
    pub fn get(mut self,keywords: &str) -> Self {
        let url = format!(
            "https://api.bgm.tv/search/subject/{}?type=2&responseGroup=small",
            keywords
        );
        // 获取响应文本并使用 miniserde 解析
        // 使用 minreq 发送 GET 请求并获取文本
        let response = minreq::get(url).send().unwrap();
        let response_text = response.as_str().unwrap();
        let search_result: SearchResult = json::from_str(&response_text).unwrap();

        // 存储番剧数据到Bgm结构体
        for subject in search_result.list {
            self.id.push(subject.id.to_string());
            self.name.push(
                match subject.name_cn.is_empty() {
                    true => subject.name,
                    false => subject.name_cn,
                }
            );
        }
        self
    }
}

#[derive(Debug)] // 添加 Debug trait
pub struct Ep {
    pub name: Vec<String>,
    pub year: i32,
}

impl Ep {
    pub fn new() -> Self{
        Self { name: Vec::new(), year: 1970 }
    } 
    fn get(id: &str) -> Self { // 移除 access_token 参数
        // let client = reqwest::blocking::Client::new(); // 移除 reqwest client
        let url = format!(
            "https://api.bgm.tv/v0/episodes?subject_id={}&type=0&limit=100&offset=0",
            id
        );
        // 使用 minreq 发送 GET 请求
        let response = minreq::get(url)
            .with_header("User-Agent", "uuzp/bgm_rename_cuby")
            // 移除 Authorization header
            .send()
            .unwrap();
        // 获取响应文本并使用 miniserde 解析
        let response_text = response.as_str().unwrap();
        let episodes_result: EpisodesResult = json::from_str(&response_text).unwrap();

        let mut ep_list = Vec::new();
        let mut epn = Vec::new();
        let mut year = 1970; // 默认年份

        // 从第一个剧集中提取年份
        if let Some(first_episode) = episodes_result.data.first() {
             // 安全地解析年份，如果失败则使用默认值
             year = first_episode.airdate.get(0..4).unwrap_or("").parse().unwrap_or(1970);
        }        // 提取剧集编号和名称（优先使用中文名称，如果为空则使用原名）
        for episode in episodes_result.data {
            epn.push(episode.sort);
            // 如果 name_cn 不为空，则使用 name_cn，否则使用 name
            let s = if !episode.name_cn.is_empty() {
                episode.name_cn
            } else {
                episode.name
            };
            // 替换 HTML 实体
            let s = s.replace("<", "＜");
            let s = s.replace(">", "＞");
            ep_list.push(s);
        }

        let mut name = vec![];
        // 计算最大剧集编号所需的位数，用于格式化前导零
        let max_ep_num = epn.iter().max().cloned().unwrap_or(0);
        // 避免 log10(0) 导致 panic
        let num_digits = if max_ep_num == 0 { 1 } else { (max_ep_num as f64).log10() as usize + 1 };

        // 格式化剧集名称
        for i in 0..ep_list.len() {
            // 使用计算出的位数格式化剧集编号，添加前导零
            let ep_num_str = format!("{:0width$}", epn[i], width = num_digits);
            let ep = format!("ep{} - {}", ep_num_str, ep_list[i]);
            name.push(ep);
        }

        Ep { name, year }
    }
}


pub fn select_folder() -> PathBuf{
    let mut dialog = dialog::NativeFileChooser::new(dialog::NativeFileChooserType::BrowseDir);
    dialog.show();
    dialog.filename()
}
pub fn mkdir(outpaths:&Vec<PathBuf>) {
    for path in outpaths {

        match fs::create_dir(path) {
            Ok(_) => (),
            Err(e) => {
                dialog::message_default(&e.to_string());
            }
        }
    }


}

// 移除不再需要的 collect 函数

pub fn replace2(s:&str) -> String {
    let s = s.replace("/", "／");
    let s = s.replace("\\", "＼");
    let s = s.replace("<", "＜");
    let s = s.replace(">", "＞");
    s
}

pub fn set_out_path(s:&str) -> PathBuf {
    
    let mut path = PathBuf::new();
    path.push(s);

    path

}

pub fn get_out_file_paths(path:PathBuf,v_names:Vec<String>) -> Vec<PathBuf> {

    v_names.iter().map(|x| {
        let x = replace2(x);
        let mut x2 =path.clone();
        x2.push(x);
        x2 
    }).collect()

}
pub fn out_ep_path(p:PathBuf,v:Vec<(String,String)>,a:&str) -> (Vec<Ep>,Vec<PathBuf>) {
    let ep:Vec<Ep> =  v.iter().map(|(x,_y)| {
         Ep::get(&x) // 移除 a 参数
     }).collect();
    let year = ep[0].year;
    let v_p:Vec<PathBuf> = v.iter().map(|(_x,y)| {
        let y = format!("{} ({})",replace2(y),year);
        let mut p2 = p.clone();
        p2.push(y);
        p2
    }).collect();
    
    (ep,v_p)
}

pub fn out_file_names(p:&PathBuf,e:&Ep) -> Vec<PathBuf>{
    e.name.iter().map(|x|{
        let mut p2 = p.clone();
        p2.push(x);
        p2
    }).collect()
}

pub fn link_rename(v1:Vec<PathBuf>,v2:Vec<PathBuf>,v3:Vec<Ep>) {
        //TODO 可以用结构体包一下
        let video_suf = "mp4 mkv avi flv";
        let sub_suf = "ass srt";
        let sub_suf_sc = "sc chs gb";
        let sub_suf_tc = "tc cht big5";
        //TODO v是一堆输入文件夹路径，得扫描每个文件夹里的文件。
        
        for i in 0..v1.len() {
            let vvv = filenames(&v1[i]);
            let mut video = panduan(vvv.clone(), video_suf);
            let sub = panduan(vvv.clone(), sub_suf);
            let mut sub_sc = panduan2(sub.clone(), sub_suf_sc); 
            let mut sub_tc = panduan2(sub.clone(), sub_suf_tc);
            
            let sub_sub = format!("{} {}",sub_suf_sc,sub_suf_tc);
            let mut sub = remove_st(sub, sub_sub);
            
            let out_files_names =  out_file_names(&v2[i], &v3[i]);
            let video_suf = &video[0].clone().extension().unwrap().to_str().unwrap().to_string();
            
            let mut files1 = Files::new();
            let mut files2 = Files::new();

            for j in &out_files_names {
                let mut j = j.clone();
                j.set_extension(video_suf);
                files2.video.push(j);
            }

            if let false = sub.is_empty() {
                let sub_suf = &sub[0].clone().extension().unwrap().to_str().unwrap().to_string();
                for j in &out_files_names {
                    let mut j = j.clone();
                    j.set_extension(sub_suf);
                    files2.sub.push(j);
                }
            }
            
            if let false = sub_sc.is_empty() {
                let sub_sc_suf = &sub_sc[0].clone().extension().unwrap().to_str().unwrap().to_string();
                for j in &out_files_names {
                    let mut j = j.clone();
                    j.set_extension(format!("sc.{}",sub_sc_suf));
                    files2.sub_sc.push(j);
                }
            }
            if let false = sub_tc.is_empty() {
                let sub_tc_suf = &sub_tc[0].clone().extension().unwrap().to_str().unwrap().to_string();
                for j in &out_files_names {
                    let mut j = j.clone();
                    j.set_extension(format!("tc.{}",sub_tc_suf));
                    files2.sub_tc.push(j);
                }
            }
            let video = file_sort(&mut video);
            let sub = file_sort(&mut sub);
            let sub_sc = file_sort(&mut sub_sc);
            let sub_tc = file_sort(&mut sub_tc);
    
            for j in video {
                files1.video.push(j);
            }
            for j in sub {
                files1.sub.push(j);
            }
            for j in sub_sc {
                files1.sub_sc.push(j);
            }
            for j in sub_tc {
                files1.sub_tc.push(j);
            }
         

            for i in 0..files1.video.len(){
                fs::hard_link(&files1.video[i], &files2.video[i]).unwrap();
                if let false = &files1.sub.is_empty() {

                    fs::copy(&files1.sub[i], &files2.sub[i]).unwrap();
                }
                
                if let false = &files1.sub_sc.is_empty(){

                    fs::copy(&files1.sub_sc[i], &files2.sub_sc[i]).unwrap();
                }
                if let false = &files1.sub_tc.is_empty(){

                    fs::copy(&files1.sub_tc[i], &files2.sub_tc[i]).unwrap();
                }
            }
    
        } 
}




pub fn files(s:&String) -> Vec<PathBuf> {
    let path = fs::read_dir(s).unwrap();
    path.map(|x| x.unwrap().path()).collect()    
}
fn filenames2(s:&Vec<PathBuf>) -> Vec<String>{
   s.iter().map(|x|x.file_name().unwrap().to_str().unwrap().to_string()).collect()
}
fn filenames(dir:&PathBuf) -> Vec<PathBuf> {
    fs::read_dir(dir).unwrap()
    .into_iter()
    .filter(|r| r.is_ok()) // Get rid of Err variants for Result<DirEntry>
    .map(|r| r.unwrap().path()) // This is safe, since we only have the Ok variants
    .filter(|r| r.is_file()) // Filter out non-folders
    .collect()
}

#[derive(Debug, Clone)]
pub struct Files{
    pub video : Vec<PathBuf>,
    pub sub: Vec<PathBuf>,
    pub sub_sc: Vec<PathBuf>,
    pub sub_tc: Vec<PathBuf>,
}
impl Files {
    fn new() -> Self {
        Self { video: Vec::new(), sub: Vec::new(), sub_sc: Vec::new(), sub_tc: Vec::new() }
    }
    pub fn sort(self) -> Self {
        
        
    

        Files::new()
    }
}
// ->Vec<PathBuf>
pub fn file_sort(v:&mut Vec<PathBuf>) ->Vec<PathBuf> {
    let mut anitomy = Anitomy::new();

    v.sort_by_key(|x| {
        let  xx = x.file_name().unwrap().to_str().unwrap();
        let s:i32 = match anitomy.parse(xx).unwrap().get(ElementCategory::EpisodeNumber) {
            Some(m) => m.parse().unwrap(),
            None => 0,
        }; 
        s
    });
  
    v.clone()

}


pub fn just_link(v1:Vec<PathBuf>,v2:Vec<PathBuf>) {
    //TODO 可以用结构体包一下
    let video_suf = "mp4 mkv";
    let sub_suf = "ass srt";
    let sub_suf_sc = "sc chs gb";
    let sub_suf_tc = "tc cht big5";
    //TODO v是一堆输入文件夹路径，得扫描每个文件夹里的文件。
    
    for i in 0..v1.len() {
        let vvv = filenames(&v1[i]);
        let video = panduan(vvv.clone(), video_suf);
        let sub = panduan(vvv.clone(), sub_suf);
        let sub_sc = panduan2(sub.clone(), sub_suf_sc); 
        let sub_tc = panduan2(sub.clone(), sub_suf_tc);
        let sub_sub = format!("{} {}",sub_suf_sc,sub_suf_tc);
        let sub = remove_st(sub, sub_sub);
        let video_names = filenames2(&video);
        let sub_names = filenames2(&sub);
        let sub_sc_names = filenames2(&sub_sc);
        let sub_tc_naems = filenames2(&sub_tc);
        
        let video_outs = mkouts(&v2[i], video_names);
        let sub_outs = mkouts(&v2[i], sub_names);
        let sub_sc_outs = mkouts(&v2[i], sub_sc_names);
        let sub_tc_outs = mkouts(&v2[i], sub_tc_naems); 


        let mut files1 = Files::new();
        let mut files2 = Files::new();
        for j in video {
            files1.video.push(j);
        }
        for j in sub {
            files1.sub.push(j);
        }
        for j in sub_sc {
            files1.sub_sc.push(j);
        }
        for j in sub_tc {
            files1.sub_tc.push(j);
        }
        for j in video_outs {
            files2.video.push(j);
        }
        for j in sub_outs {
            files2.sub.push(j);
        }
        for j in sub_sc_outs {
            files2.sub_sc.push(j);
        }
        for j in sub_tc_outs {
            files2.sub_tc.push(j);
        }
        
        println!("{:?}\n{:?}\n",files1,files2);
        for i in 0..files1.video.len(){
            fs::hard_link(&files1.video[i], &files2.video[i]).unwrap();
            fs::copy(&files1.sub[i], &files2.sub[i]).unwrap();
            fs::copy(&files1.sub_sc[i], &files2.sub_sc[i]).unwrap();
            fs::copy(&files1.sub_tc[i], &files2.sub_tc[i]).unwrap();
        }

    } 


   
// .map(|x| Path::new(x).extension().and_then(OsStr::to_str).unwrap().to_string()).collect();
}

fn mkouts(p:&PathBuf,v:Vec<String>) -> Vec<PathBuf> {
   v.iter().map(|x|{
    let mut pp = p.clone();
    pp.push(x);
    pp
   }).collect()
} 

fn panduan(names:Vec<PathBuf>, sufs: &str) -> Vec<PathBuf>{
    println!("names:\n{:?}\nsufs:\n{:?}\n",names,sufs);
    let names:Vec<PathBuf> = names.into_iter().map(|x| {
        let suf = Path::new(&x).extension().and_then(OsStr::to_str).unwrap();
        match sufs.contains(&suf.to_lowercase()) {
         true => x,
         false => PathBuf::new(),
        }
     }).filter(|x| !x.to_str().unwrap().is_empty()).collect();
    
     names
}

fn panduan2(subs:Vec<PathBuf>,sufs:&str) -> Vec<PathBuf> {
   
    let suf = match subs.is_empty() {
       false => subs[0].extension().unwrap().to_str().unwrap(),
       true => "None",
    };
    let subs = subs.iter().map(|x| {
        let m =x.to_str().unwrap().to_string();
        let m = match m.rsplit_once(".") {
            Some((x,_y)) => x.to_string(),
            None => "None".to_string(),  
        };
        m
    }).filter(|x|{
        let m = match x.rsplit_once(".") {
            Some(s) => s.1,
            None => "None",
        }; 
        sufs.contains(&m.to_lowercase())
    }).map(|x| {
        let x = format!("{}.{}",x,suf);
        let mut p = PathBuf::new();
        p.push(x);
        p
    }).collect();
    println!("sufs:\n{:?}\nsubs:\n{:?}\n",sufs,subs);
    subs
}
fn remove_st(v:Vec<PathBuf>,s:String) -> Vec<PathBuf> {
   // let suf = v[0].extension().unwrap().to_str().unwrap();
    let sub = v.iter().map(|x|{
        let ss = x.to_str().unwrap();
        ss
    }).filter(|x| {
        let ss = x.split_once(".").unwrap().1;
        let ss = match ss.split_once("."){
            Some(m) => m.0,
            None => "None",
        };
        !s.contains(&ss.to_lowercase())
    }).map(|x| {
     //   let x = format!("{}.{}",x,suf);
        let mut p = PathBuf::new();
        p.push(x);
        p
    }).collect();
    sub
}

#[derive(RustEmbed)]
#[folder = "images/"]
#[exclude = "GUI.png"]
#[exclude = "ico.rc"]
pub struct Link;

impl Link {
    pub fn get_ico() -> EmbeddedFile {
        Link::get("ice-cubes.ico").unwrap()
    }
}

pub fn get_png() -> EmbeddedFile{
    Link::get("ice-cubes.png").unwrap()
}


#[cfg(test)]
mod tests {
    use super::*;
    use miniserde::{Serialize, Deserialize}; // 在测试模块内部也引入 Serialize 和 Deserialize
    use miniserde::json;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
    #[test]
    fn test_bgm() {
        let bgm = Bgm::new();
        let bgm = bgm.get("孤独摇滚");
        println!("{:?}",bgm);
    }
    #[test]
    fn test_ep() {
        let ep = Ep::get("388190"); // 移除第二个参数
        println!("{:?}",ep);
    }
    #[test]
    fn test_file_sort() {
        let mut v = vec![
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 11 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 01 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 02 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 03 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 04 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 05 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 06 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 07 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 08 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 09 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 10 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
            PathBuf::from("[SweetSub&LoliHouse] Bocchi the Rock! - 12 [WebRip 1080p HEVC-10bit AAC ASSx2].mkv"),
        ];
        let v = file_sort(&mut v);
        println!("{:?}",v);
    }

    // 添加新的测试结构体和测试函数
    #[derive(Serialize, Deserialize, PartialEq, Debug)] // 添加 PartialEq 和 Debug 用于比较和打印
    struct MiniserdeTestStruct {
        name: String,
        value: i32,
        enabled: bool,
    }


    #[test]
    fn test_serde_json_serde() {
        let original_data = MiniserdeTestStruct {
            name: "test".to_string(),
            value: 123,
            enabled: true,
        };

        // 序列化
        let json_string = json::to_string(&original_data); // miniserde::json::to_string is infallible
        println!("Serialized JSON: {}", json_string);

        // 反序列化
        let deserialized_data: MiniserdeTestStruct = json::from_str(&json_string).unwrap();
        println!("Deserialized data: {:?}", deserialized_data);

        // 验证
        assert_eq!(original_data, deserialized_data);
    }
}
