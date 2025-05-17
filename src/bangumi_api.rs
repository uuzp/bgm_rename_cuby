// src/bangumi_api.rs

use miniserde::{Deserialize, Serialize, json};
use minreq;
use urlencoding;

// --- 通用网络请求辅助函数 ---
pub fn fetch_json_from_api<T: Deserialize>(url: &str, user_agent: Option<&str>) -> Result<T, String> {
    let mut request = minreq::get(url);
    if let Some(ua) = user_agent {
        request = request.with_header("User-Agent", ua);
    }

    let response = request.send().map_err(|e| format!("网络请求失败: {}", e))?;
    let response_text = response.as_str().map_err(|e| format!("响应文本编码错误: {}", e))?;

    match json::from_str(&response_text) {
        Ok(res) => Ok(res),
        Err(e) => {
            #[derive(Deserialize)]
            struct ApiErrorResponse {
                title: Option<String>,
                #[serde(rename = "type")]
                error_type: Option<String>,
                detail: Option<String>,
            }
            if let Ok(err_resp) = json::from_str::<ApiErrorResponse>(&response_text) {
                Err(format!(
                    "BGM API 错误: {}. 类型: {}. 详情: {}",
                    err_resp.title.unwrap_or_default(),
                    err_resp.error_type.unwrap_or_default(),
                    err_resp.detail.unwrap_or_default()
                ))
            } else {
                // 如果尝试解析为 ApiErrorResponse 也失败，则很可能响应不是 JSON 错误。
                // 返回一个更通用的消息，不包含原始响应体，以避免显示 HTML。
                Err(format!("API响应解析失败: 无法识别的错误格式。初始JSON解析错误: {}", e))
            }
        }
    }
}

// --- 数据模型 (直接用于反序列化和内部使用) ---
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BangumiSubject { // 重命名自 Subject
    pub id: u64,
    pub name: String,
    pub name_cn: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BangumiEpisode { // 重命名自 Episode
    pub airdate: String,
    pub sort: u64,
    pub name: String,
    pub name_cn: String,
}

// --- API 响应的顶层结构 ---
#[derive(Deserialize, Serialize)]
struct SubjectListPayload { // 重命名自 SearchResult
    list: Vec<BangumiSubject>,
}

#[derive(Deserialize, Serialize)]
struct EpisodeListPayload { // 重命名自 EpisodesResult
    data: Vec<BangumiEpisode>,
}

// --- 资源获取器特征 ---
pub trait ResourceFetcher<Param> where Self: Sized {
    fn fetch(param: Param) -> Result<Self, String>;
}

// --- 实现特征: 搜索番剧主题 ---
// 返回 Vec<BangumiSubject>
impl ResourceFetcher<&str> for Vec<BangumiSubject> {
    fn fetch(keywords: &str) -> Result<Self, String> {
        let encoded_keywords = urlencoding::encode(keywords);
        let url = format!(
            "https://api.bgm.tv/search/subject/{}?type=2&responseGroup=small",
            encoded_keywords
        );
        let payload: SubjectListPayload = fetch_json_from_api(&url, None)?;
        Ok(payload.list)
    }
}

// --- 剧集集合结构体 ---
#[derive(Debug, Clone)]
pub struct EpisodeCollection { // 重命名自 Ep
    pub episodes: Vec<BangumiEpisode>,
    pub year: i32,
}

impl EpisodeCollection {
    pub fn new_empty() -> Self { // 用于创建空实例
        Self { episodes: Vec::new(), year: 1970 }
    }

    /// 获取格式化后的剧集名称列表 (原始名称，不替换特殊字符)
    pub fn get_formatted_names(&self) -> Vec<String> {
        if self.episodes.is_empty() {
            return Vec::new();
        }
        let mut ep_display_names = Vec::new();
        let mut ep_sort_numbers = Vec::new();

        for episode in &self.episodes {
            ep_sort_numbers.push(episode.sort);
            let name_to_use = if !episode.name_cn.is_empty() {
                &episode.name_cn
            } else {
                &episode.name
            };
            ep_display_names.push(name_to_use.to_string()); // 返回原始名称
        }

        let mut formatted_names = vec![];
        let max_ep_num = ep_sort_numbers.iter().max().cloned().unwrap_or(0);
        let num_digits = if max_ep_num == 0 { 1 } else { (max_ep_num as f64).log10() as usize + 1 };

        for i in 0..ep_display_names.len() {
            let ep_num_str = format!("{:0width$}", ep_sort_numbers[i], width = num_digits);
            let formatted_ep_name = format!("ep{} - {}", ep_num_str, ep_display_names[i]);
            formatted_names.push(formatted_ep_name);
        }
        formatted_names
    }
}

// --- 实现特征: 获取剧集 ---
// 参数为 u64 类型的 subject_id
impl ResourceFetcher<u64> for EpisodeCollection {
    fn fetch(subject_id: u64) -> Result<Self, String> {
        let url = format!(
            "https://api.bgm.tv/v0/episodes?subject_id={}&type=0&limit=100&offset=0",
            subject_id
        );
        let payload: EpisodeListPayload = fetch_json_from_api(&url, Some("uuzp/bgm_rename_cuby"))?;

        let mut year = 1970;
        if let Some(first_episode) = payload.data.first() {
            year = first_episode.airdate.get(0..4).unwrap_or("").parse().unwrap_or(1970);
        }

        Ok(EpisodeCollection { episodes: payload.data, year })
    }
}
