// src/bangumi_api.rs

use miniserde::Deserialize;
use std::time::Duration;

// 配置常量
const BGM_API_BASE: &str = "https://api.bgm.tv";
const DEFAULT_USER_AGENT: &str = "uuzp/bgm_rename_cuby";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

// 优化的错误类型 - 保持简洁但有区分度
#[derive(Debug)]
pub enum BangumiError {
    Network(String),
    NotFound,
    RateLimit,
    Parse(String),
    InvalidInput(String),
}

impl std::fmt::Display for BangumiError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BangumiError::Network(msg) => write!(f, "网络错误: {}", msg),
            BangumiError::NotFound => write!(f, "未找到请求的资源"),
            BangumiError::RateLimit => write!(f, "请求频率限制，请稍后重试"),
            BangumiError::Parse(msg) => write!(f, "数据解析错误: {}", msg),
            BangumiError::InvalidInput(msg) => write!(f, "输入错误: {}", msg),
        }
    }
}

impl std::error::Error for BangumiError {}

// 简化的 Subject 结构体 - 只保留实际使用的字段
#[derive(Deserialize, Debug, Clone)]
pub struct Subject {
    pub id: u64,
    pub name: String,
    pub name_cn: String,
    pub air_date: Option<String>,
}

// 简化的 Episode 结构体 - 移除 airdate，年份通过 Subject 获取
#[derive(Deserialize, Debug, Clone)]
pub struct Episode {
    pub sort: u64,
    pub name: String,
    pub name_cn: String,
}

// 简化的 Episodes 结构体 - 移除 subject_id
#[derive(Debug, Clone)]
pub struct Episodes {
    pub items: Vec<Episode>,
    pub year: i32,
}

impl Subject {
    pub fn year(&self) -> Option<i32> {
        self.air_date
            .as_ref()
            .and_then(|date| date.get(0..4))
            .and_then(|year_str| year_str.parse().ok())
    }
}

impl Episode {
    pub fn display_name(&self) -> &str {
        if !self.name_cn.is_empty() { 
            &self.name_cn 
        } else { 
            &self.name 
        }
    }
}

impl Episodes {
    pub fn new(items: Vec<Episode>, subject: &Subject) -> Self {
        Self {
            items,
            year: subject.year().unwrap_or(1970),
        }
    }

    pub fn formatted_names(&self) -> Vec<String> {
        // 保持现有格式化逻辑，只是代码更简洁
        if self.items.is_empty() {
            return Vec::new();
        }

        let max_ep_num = self.items.iter().map(|ep| ep.sort).max().unwrap_or(0);
        let num_digits = if max_ep_num > 0 {
            (max_ep_num as f64).log10() as usize + 1
        } else {
            1
        };

        self.items
            .iter()
            .map(|episode| {
                let name = episode.display_name();
                format!("ep{:0width$} - {}", episode.sort, name, width = num_digits)
            })
            .collect()
    }
}

// 通用的API响应结构体 - 简化内部使用
#[derive(Deserialize)]
struct SearchResponse {
    list: Vec<Subject>,
}

#[derive(Deserialize)]
struct EpisodeResponse {
    data: Vec<Episode>,
}

// 简化的网络请求函数
fn fetch_json<T: Deserialize>(url: &str) -> Result<T, BangumiError> {
    let response = minreq::get(url)
        .with_timeout(REQUEST_TIMEOUT.as_secs())
        .with_header("User-Agent", DEFAULT_USER_AGENT)
        .send()
        .map_err(|e| BangumiError::Network(format!("网络请求失败: {}", e)))?;
    
    // 检查HTTP状态码
    match response.status_code {
        200 => {},
        404 => return Err(BangumiError::NotFound),
        429 => return Err(BangumiError::RateLimit),
        code => return Err(BangumiError::Network(format!("HTTP错误: {}", code))),
    }
      let response_text = response.as_str()
        .map_err(|e| BangumiError::Parse(format!("响应文本编码错误: {}", e)))?;
    
    miniserde::json::from_str(response_text)
        .map_err(|e| BangumiError::Parse(format!("JSON解析失败: {}", e)))
}

// 直接且明确的API函数
pub fn search_subjects(keywords: &str) -> Result<Vec<Subject>, BangumiError> {
    let trimmed = keywords.trim();
    if trimmed.is_empty() {
        return Err(BangumiError::InvalidInput("搜索关键词不能为空".to_string()));
    }
    if trimmed.len() > 100 {
        return Err(BangumiError::InvalidInput("搜索关键词过长".to_string()));
    }
    
    let encoded_keywords = urlencoding::encode(trimmed);    let url = format!(
        "{}/search/subject/{}?type=2&responseGroup=large&limit=25",
        BGM_API_BASE,
        encoded_keywords
    );
    
    let response: SearchResponse = fetch_json(&url)?;
    Ok(response.list)
}

// 修改为接受 Subject 引用，自动处理年份
pub fn get_episodes(subject: &Subject) -> Result<Episodes, BangumiError> {
    let url = format!(
        "{}/v0/episodes?subject_id={}&type=0&limit=100&offset=0",
        BGM_API_BASE,
        subject.id
    );
    
    let response: EpisodeResponse = fetch_json(&url)?;
    
    Ok(Episodes::new(response.data, subject))
}

// --- 测试模块 ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episodes_formatting() {
        let episodes = vec![
            Episode {
                sort: 1,
                name: "Episode 1".to_string(),
                name_cn: "第一集".to_string(),
            },
            Episode {
                sort: 12,
                name: "Episode 12".to_string(),
                name_cn: "第十二集".to_string(),
            },
        ];
        
        let subject = Subject {
            id: 123,
            name: "Test Anime".to_string(),
            name_cn: "测试动画".to_string(),
            air_date: Some("2023-01-01".to_string()),
        };
        
        let eps = Episodes::new(episodes, &subject);
        let formatted = eps.formatted_names();
        
        assert_eq!(formatted[0], "ep01 - 第一集");
        assert_eq!(formatted[1], "ep12 - 第十二集");
    }

    #[test]
    fn test_empty_search() {
        let result = search_subjects("");
        assert!(result.is_err());
        match result {
            Err(BangumiError::InvalidInput(_)) => {},
            _ => panic!("Expected InvalidInput error"),
        }
    }
}
