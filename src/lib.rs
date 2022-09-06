use std::{path::{PathBuf, Path}, fs, ffi::OsStr};

use fltk::dialog;
use reqwest::header::USER_AGENT;
use anitomy::{Anitomy, ElementCategory};
use rust_embed::{RustEmbed, EmbeddedFile};

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
        let json = reqwest::blocking::get(url).unwrap().json().unwrap();
    

        // 提取番剧name和id

        let id = collect(&json, &"id");
        let name = collect(&json, &"name");
        let name_cn = collect(&json, &"name_cn");

        // 存储番剧数据到Bgm结构体
        for i in 0..id.len() {
            self.id.push(id[i].to_owned());
            self.name.push(
                match name_cn[i].is_empty() {
                    true => String::from(name[i].to_owned()),
                    false => String::from(name_cn[i].to_owned()),
                }    
            );
        }
        self
    }
}

pub struct Ep {
    pub name: Vec<String>,
    pub year: i32,
}

impl Ep {
    pub fn new() -> Self{
        Self { name: Vec::new(), year: 1970 }
    } 
    fn get(id:&str) -> Self{
        let client = reqwest::blocking::Client::new();
        let url = format!(
            "https://api.bgm.tv/v0/episodes?subject_id={}&type=0&limit=100&offset=0",
            id
        );
        let json: serde_json::Value = client
            .get(url)
            .header(USER_AGENT, "bgm_rename_cuby")
            .send().unwrap()
            .json().unwrap();
            let mut ep_list = Vec::new();
            let year = jsonpath_lib::select(&json, "$.data.*.airdate").unwrap();
            let year:i32 = year[0].as_str().unwrap()[0..4].parse().unwrap();
            // let total = jsonpath::select(&json_body, "$.total").unwrap();
            // let total = total[0].as_i64().unwrap();
            // let total = total as usize;
    
            let mut epn = Vec::new();
            for s in jsonpath_lib::selector(&json)("$.data.*.sort").unwrap() {
                epn.push(s.as_i64().unwrap());
            }
            for s in jsonpath_lib::selector(&json)("$.data.*.name_cn").unwrap() {
                let s = s.as_str().unwrap();
                let s = s.replace("&lt;", "＜");
                let s = s.replace("&gt;", "＞");
                ep_list.push(s);
            }
            let mut name = vec![];
            let len = epn.len();
            let len = epn[len - 1].to_string().len();
            
            for i in 0..ep_list.len() {
               let ep = match len {
                    3  => match epn[i] > 99  {
                        true => format!("ep{} - ",epn[i]),
                        false => match epn[i] > 9 {
                            true => format!("ep0{} - ",epn[i]),
                            false => format!("ep00{} - ",epn[i]),
                        }   
                    }

                    2 => match  epn[i] > 9 {
                        true => format!("ep{} - ",epn[i]),
                        false => format!("ep0{} - ",epn[i]), 
                    }
                    1 => format!("ep0{} - ",epn[i]),
                    _ => "".to_string(),
                };
                let ep = format!("{}{}",ep,ep_list[i]);
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

fn collect(json:&serde_json::Value,s:&str) -> Vec<String> {
    jsonpath_lib::selector(json)(&format!("$.list.*.{}",s))
        .unwrap()
        .iter()
        .map(|x| 
            match x.as_str() {
                Some(m) =>  m.to_string(),
                None => x.as_i64().unwrap().to_string(),
            }
        )
        .collect()
}

pub fn replace2(s:&str) -> String {
    let s = s.replace("/", "／");
    let s = s.replace("\\", "＼");
    let s = s.replace("&lt;", "＜");
    let s = s.replace("&gt;", "＞");
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
pub fn out_ep_path(p:PathBuf,v:Vec<(String,String)>) -> (Vec<Ep>,Vec<PathBuf>) {
    let ep:Vec<Ep> =  v.iter().map(|(x,_y)| {
         Ep::get(&x)
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


pub struct Link {
    pub inpaths: Vec<PathBuf>,
    pub outpaths: Vec<PathBuf>,
}

impl Link {
    pub fn new() -> Self{
        Self { inpaths: Vec::new(), outpaths: Vec::new() }
    }
    pub fn set(v_inpaths:Vec<PathBuf>,v_outpaths:Vec<PathBuf>) -> Self {
        Self { inpaths: v_inpaths, outpaths: v_outpaths }
    }
}

#[derive(RustEmbed)]
#[folder = "images/"]
#[include = "ice-cubes.png"]
struct Asset;

pub fn get_png() -> EmbeddedFile{
    let png = Asset::get("ice-cubes.png").unwrap();
    println!("{:?}", std::str::from_utf8( png.data.as_ref()));
   png
}



#[cfg(test)]
mod tests {
   // use std::path::PathBuf;
    
 //   use crate::get_png;
  //  use crate::file_sort;
    use crate::Ep;


    #[test]
    fn it_works() {
        // get_png();
        // let mut f1 = PathBuf::new();
        // f1.push("[LavaAnime & MingY] Machikado Mazoku S2 [010][1080p][CHS&JPN].mp4");
        // let mut f2 = PathBuf::new();
        // f2. push("[LavaAnime & MingY] Machikado Mazoku S2 [11v2][1080p][CHS&JPN] .mp4");
        // let mut f3 = PathBuf::new();
        // f3. push("[LavaAnime & MingY] Machikado Mazoku S2 [12][1080p][CHS&JPN] .mp4");
        // let mut f4 = PathBuf::new();
        // f4. push("[MingY] Machikado Mazoku S2 [7][1080p][CHS].mp4");
        // let mut f5 = PathBuf::new();
        // f5.push("[MingY] Machikado Mazoku S2 [08v2][1080p][CHS].mp4");
        // let mut f6 = PathBuf::new();
        // f6. push("[MingY] Machikado Mazoku S2 [9v2][1080p][CHS&JPN].mp4");
        // let mut f7 = PathBuf::new();
        // f7. push("D:\\1\\2\\3\\video1.sc.ass");
        // let mut f8 = PathBuf::new();
        // f8. push("D:\\1\\2\\3\\video1.tc.ass");
        // let mut v = vec![f1,f2,f3,f4,f5,f6];
        // println!("原始数据：{:?}",&v);
        // file_sort(&mut v);
        // println!("排序数据：{:?}",&v);
        // let files = name_extension(v);
        
        let ep = Ep::get("299673");
        println!("{:?}\n{:?}",ep.name,ep.year);

    }

}

