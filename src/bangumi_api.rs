// src/bangumi_api.rs

use miniserde::{Deserialize, Serialize, json};
use minreq;
use urlencoding;
use std::time::Duration;

// --- 配置常量 ---
const BGM_API_BASE: &str = "https://api.bgm.tv";
const BGM_API_V0_BASE: &str = "https://api.bgm.tv/v0";
const DEFAULT_USER_AGENT: &str = "uuzp/bgm_rename_cuby";
const MAX_RETRIES: u32 = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

// --- 自定义错误类型 ---
#[derive(Debug)]
pub enum BangumiError {
    NetworkError(String),
    ParseError(String),
    ApiError {
        title: String,
        error_type: String,
        detail: String,
    },
    NotFound,
    RateLimited,
}

impl std::fmt::Display for BangumiError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BangumiError::NetworkError(msg) => write!(f, "网络请求失败: {}", msg),
            BangumiError::ParseError(msg) => write!(f, "响应解析失败: {}", msg),
            BangumiError::ApiError { title, error_type, detail } => {
                write!(f, "BGM API 错误: {}. 类型: {}. 详情: {}", title, error_type, detail)
            }
            BangumiError::NotFound => write!(f, "未找到请求的资源"),
            BangumiError::RateLimited => write!(f, "请求频率限制"),
        }
    }
}

impl std::error::Error for BangumiError {}

// --- 统一的获取接口 ---
pub fn get<T, P>(param: P) -> Result<T, BangumiError> 
where
    T: Fetcher<P>,
{
    T::fetch(param)
}

// --- 核心特征 ---
pub trait Fetcher<P> {
    fn fetch(param: P) -> Result<Self, BangumiError>
    where
        Self: Sized;
}

// --- 通用网络请求辅助函数 ---
fn fetch_json<T: Deserialize>(url: &str, user_agent: Option<&str>) -> Result<T, BangumiError> {
    fetch_json_with_retry(url, user_agent, MAX_RETRIES)
}

fn fetch_json_with_retry<T: Deserialize>(
    url: &str, 
    user_agent: Option<&str>, 
    retries: u32
) -> Result<T, BangumiError> {
    let mut last_error = None;
    
    for attempt in 0..=retries {
        match perform_request(url, user_agent) {
            Ok(response_text) => {
                return parse_response(&response_text);
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < retries {
                    std::thread::sleep(Duration::from_millis(500 * (attempt + 1) as u64));
                }
            }
        }
    }
    
    Err(last_error.unwrap_or(BangumiError::NetworkError("未知错误".to_string())))
}

fn perform_request(url: &str, user_agent: Option<&str>) -> Result<String, BangumiError> {
    let mut request = minreq::get(url)
        .with_timeout(REQUEST_TIMEOUT.as_secs());
    
    if let Some(ua) = user_agent {
        request = request.with_header("User-Agent", ua);
    }

    let response = request.send()
        .map_err(|e| BangumiError::NetworkError(format!("请求失败: {}", e)))?;
    
    // 检查HTTP状态码
    match response.status_code {
        200 => {},
        404 => return Err(BangumiError::NotFound),
        429 => return Err(BangumiError::RateLimited),
        code => return Err(BangumiError::NetworkError(format!("HTTP错误: {}", code))),
    }
    
    response.as_str()
        .map(|s| s.to_string())
        .map_err(|e| BangumiError::ParseError(format!("响应文本编码错误: {}", e)))
}

fn parse_response<T: Deserialize>(response_text: &str) -> Result<T, BangumiError> {
    match json::from_str(response_text) {
        Ok(res) => Ok(res),
        Err(e) => {
            // 尝试解析API错误响应
            if let Ok(err_resp) = json::from_str::<ApiErrorResponse>(response_text) {
                Err(BangumiError::ApiError {
                    title: err_resp.title.unwrap_or_default(),
                    error_type: err_resp.error_type.unwrap_or_default(),
                    detail: err_resp.detail.unwrap_or_default(),
                })
            } else {
                Err(BangumiError::ParseError(format!(
                    "JSON解析失败: {}. 响应内容: {}", 
                    e, 
                    response_text.chars().take(200).collect::<String>()
                )))
            }
        }
    }
}

// --- 数据模型 ---
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Subject {
    pub id: u64,
    pub name: String,
    pub name_cn: String,
    #[serde(rename = "type")]
    pub subject_type: Option<u8>,
    pub air_date: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Episode {
    pub airdate: String,
    pub sort: u64,
    pub name: String,
    pub name_cn: String,
    pub duration: Option<String>,
    pub desc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Episodes {
    pub items: Vec<Episode>,
    pub year: i32,
    pub subject_id: u64,
}

impl Episodes {
    pub fn empty() -> Self {
        Self { 
            items: Vec::new(), 
            year: 1970,
            subject_id: 0,
        }
    }

    pub fn new(items: Vec<Episode>, subject_id: u64) -> Self {
        let year = items
            .first()
            .and_then(|ep| ep.airdate.get(0..4))
            .and_then(|year_str| year_str.parse().ok())
            .unwrap_or(1970);
        
        Self { items, year, subject_id }
    }

    pub fn formatted_names(&self) -> Vec<String> {
        self.formatted_names_with_options(true, true)
    }

    pub fn formatted_names_with_options(&self, use_cn_name: bool, zero_pad: bool) -> Vec<String> {
        if self.items.is_empty() {
            return Vec::new();
        }

        let max_ep_num = self.items.iter().map(|ep| ep.sort).max().unwrap_or(0);
        let num_digits = if zero_pad && max_ep_num > 0 {
            (max_ep_num as f64).log10() as usize + 1
        } else {
            1
        };

        self.items
            .iter()
            .map(|episode| {
                let name = if use_cn_name && !episode.name_cn.is_empty() {
                    &episode.name_cn
                } else {
                    &episode.name
                };
                
                if zero_pad {
                    format!("ep{:0width$} - {}", episode.sort, name, width = num_digits)
                } else {
                    format!("ep{} - {}", episode.sort, name)
                }
            })
            .collect()
    }

    pub fn get_episode_by_sort(&self, sort: u64) -> Option<&Episode> {
        self.items.iter().find(|ep| ep.sort == sort)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// --- API 响应结构 ---
#[derive(Deserialize)]
struct SubjectSearchResponse {
    list: Vec<Subject>,
}

#[derive(Deserialize)]
struct EpisodeListResponse {
    data: Vec<Episode>,
}

#[derive(Deserialize)]
struct ApiErrorResponse {
    title: Option<String>,
    #[serde(rename = "type")]
    error_type: Option<String>,
    detail: Option<String>,
}

// --- 特征实现: 字符串参数搜索番剧 ---
impl Fetcher<&str> for Vec<Subject> {
    fn fetch(keywords: &str) -> Result<Self, BangumiError> {
        if keywords.trim().is_empty() {
            return Ok(Vec::new());
        }
        
        let encoded_keywords = urlencoding::encode(keywords.trim());
        let url = format!(
            "{}/search/subject/{}?type=2&responseGroup=small&limit=25",
            BGM_API_BASE,
            encoded_keywords
        );
        
        let response: SubjectSearchResponse = fetch_json(&url, None)?;
        Ok(response.list)
    }
}

// --- 特征实现: 整数参数获取剧集 ---
impl Fetcher<u64> for Episodes {
    fn fetch(subject_id: u64) -> Result<Self, BangumiError> {
        if subject_id == 0 {
            return Ok(Episodes::empty());
        }
        
        let url = format!(
            "{}/episodes?subject_id={}&type=0&limit=100&offset=0",
            BGM_API_V0_BASE,
            subject_id
        );
        
        let response: EpisodeListResponse = fetch_json(&url, Some(DEFAULT_USER_AGENT))?;
        Ok(Episodes::new(response.data, subject_id))
    }
}

// --- 辅助函数 ---
pub fn search_subjects(keywords: &str) -> Result<Vec<Subject>, BangumiError> {
    get::<Vec<Subject>, &str>(keywords)
}

pub fn get_episodes(subject_id: u64) -> Result<Episodes, BangumiError> {
    get::<Episodes, u64>(subject_id)
}

// --- 测试模块 ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episodes_formatting() {
        let episodes = vec![
            Episode {
                airdate: "2023-01-01".to_string(),
                sort: 1,
                name: "Episode 1".to_string(),
                name_cn: "第一集".to_string(),
                duration: None,
                desc: None,
            },
            Episode {
                airdate: "2023-01-08".to_string(),
                sort: 12,
                name: "Episode 12".to_string(),
                name_cn: "第十二集".to_string(),
                duration: None,
                desc: None,
            },
        ];
        
        let eps = Episodes::new(episodes, 123);
        let formatted = eps.formatted_names();
        
        assert_eq!(formatted[0], "ep01 - 第一集");
        assert_eq!(formatted[1], "ep12 - 第十二集");
    }

    #[test]
    fn test_empty_search() {
        let result = search_subjects("");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
