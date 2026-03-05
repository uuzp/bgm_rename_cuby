// src/bangumi_api.rs

use miniserde::Deserialize;
use std::borrow::Cow;
use std::time::Duration;

#[cfg(not(windows))]
use std::io::{Read, Write};

#[cfg(not(windows))]
use std::net::TcpStream;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Networking::WinHttp::{
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_LOCATION, WINHTTP_QUERY_STATUS_CODE,
};

#[cfg(target_os = "windows")]
const METHOD_GET_W: [u16; 4] = ['G' as u16, 'E' as u16, 'T' as u16, 0];

#[cfg(target_os = "windows")]
type HINTERNET = *mut core::ffi::c_void;

#[cfg(target_os = "windows")]
struct WinHttpHandle(HINTERNET);

#[cfg(target_os = "windows")]
impl WinHttpHandle {
    fn new(handle: HINTERNET) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self(handle))
        }
    }

    fn get(&self) -> HINTERNET {
        self.0
    }
}

#[cfg(target_os = "windows")]
impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }
}

#[cfg(not(windows))]
use native_tls::TlsConnector;

// 配置常量
const BGM_API_BASE: &str = "https://api.bgm.tv";
const DEFAULT_USER_AGENT: &str = "uuzp/bgm_rename_cuby";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

// 优化的错误类型 - 保持简洁但有区分度
#[derive(Debug)]
pub enum BangumiError {
    Network(Cow<'static, str>),
    NotFound,
    RateLimit,
    Parse(Cow<'static, str>),
    InvalidInput(Cow<'static, str>),
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
    pub fn display_name(&self) -> &str {
        if !self.name_cn.is_empty() {
            &self.name_cn
        } else {
            &self.name
        }
    }

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
    let response_text = https_get_text(url, DEFAULT_USER_AGENT, REQUEST_TIMEOUT, 5)?;
    miniserde::json::from_str(&response_text)
    .map_err(|e| BangumiError::Parse(format!("JSON解析失败: {}", e).into()))
}

#[inline(never)]
fn https_get_text(
    url: &str,
    user_agent: &str,
    timeout: Duration,
    max_redirects: usize,
) -> Result<String, BangumiError> {
    #[cfg(target_os = "windows")]
    {
        return winhttp_get_text(url, user_agent, timeout, max_redirects);
    }

    #[cfg(not(windows))]
    let mut current = url.to_string();
    #[cfg(not(windows))]
    for _ in 0..=max_redirects {
        #[cfg(not(windows))]
        let (host, port, path_and_query) = parse_https_url(&current)?;
        #[cfg(not(windows))]
        let bytes = https_get_bytes(&host, port, &path_and_query, user_agent, timeout)?;
        #[cfg(not(windows))]
        let response = parse_http_response(&bytes)?;

        #[cfg(not(windows))]
        match response.status_code {
            200 => {
                return String::from_utf8(response.body)
                    .map_err(|e| BangumiError::Parse(format!("响应文本编码错误: {}", e).into()));
            }
            301 | 302 | 303 | 307 | 308 => {
                let Some(location) = response.location() else {
                    return Err(BangumiError::Network("HTTP重定向但缺少Location头".into()));
                };
                current = resolve_redirect_url(&current, location);
                continue;
            }
            404 => return Err(BangumiError::NotFound),
            429 => return Err(BangumiError::RateLimit),
            code => return Err(BangumiError::Network(format!("HTTP错误: {}", code).into())),
        }
    }

    #[cfg(not(windows))]
    {
        Err(BangumiError::Network("重定向次数过多".into()))
    }
}

fn parse_https_url(url: &str) -> Result<(String, u16, String), BangumiError> {
    let Some(rest) = url.strip_prefix("https://") else {
        return Err(BangumiError::InvalidInput("仅支持 https:// URL".into()));
    };

    let (host_port, path_part) = match rest.split_once('/') {
        Some((h, p)) => (h, format!("/{}", p)),
        None => (rest, "/".to_string()),
    };

    let (host, port) = match host_port.split_once(':') {
        Some((h, p)) => {
            let port: u16 = p
                .parse()
                .map_err(|_| BangumiError::InvalidInput("URL端口非法".into()))?;
            (h.to_string(), port)
        }
        None => (host_port.to_string(), 443u16),
    };

    if host.is_empty() {
        return Err(BangumiError::InvalidInput("URL主机为空".into()));
    }

    Ok((host, port, path_part))
}

#[cfg(not(windows))]
fn https_get_bytes(
    host: &str,
    port: u16,
    path_and_query: &str,
    user_agent: &str,
    timeout: Duration,
) -> Result<Vec<u8>, BangumiError> {
    let addr = (host, port);
    let stream = TcpStream::connect(addr)
        .map_err(|e| BangumiError::Network(format!("连接失败: {}", e).into()))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| BangumiError::Network(format!("设置读取超时失败: {}", e).into()))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| BangumiError::Network(format!("设置写入超时失败: {}", e).into()))?;

    let connector = TlsConnector::new()
        .map_err(|e| BangumiError::Network(format!("TLS初始化失败: {}", e).into()))?;
    let mut tls = connector
        .connect(host, stream)
        .map_err(|e| BangumiError::Network(format!("TLS握手失败: {}", e).into()))?;

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: {ua}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        path = path_and_query,
        host = host,
        ua = user_agent
    );
    tls.write_all(request.as_bytes())
        .map_err(|e| BangumiError::Network(format!("写入请求失败: {}", e).into()))?;

    let mut buf = Vec::new();
    tls.read_to_end(&mut buf)
        .map_err(|e| BangumiError::Network(format!("读取响应失败: {}", e).into()))?;
    Ok(buf)
}

#[cfg(target_os = "windows")]
#[inline(never)]
fn winhttp_get_text(
    url: &str,
    user_agent: &str,
    timeout: Duration,
    max_redirects: usize,
) -> Result<String, BangumiError> {
    let mut current = url.to_string();
    for _ in 0..=max_redirects {
        let (host, port, path_and_query) = parse_https_url(&current)?;

        let host_w = wide_null(&host);
        let path_w = wide_null(&path_and_query);
        let user_agent_w = wide_null(user_agent);

        let session = WinHttpHandle::new(unsafe {
            WinHttpOpen(
                user_agent_w.as_ptr(),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                std::ptr::null(),
                std::ptr::null(),
                0,
            )
        })
        .ok_or_else(|| BangumiError::Network("WinHttpOpen失败".into()))?;

        let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as i32;
        let _ = unsafe {
            WinHttpSetTimeouts(
                session.get(),
                timeout_ms,
                timeout_ms,
                timeout_ms,
                timeout_ms,
            )
        };

        let connect = WinHttpHandle::new(unsafe { WinHttpConnect(session.get(), host_w.as_ptr(), port, 0) })
            .ok_or_else(|| BangumiError::Network("WinHttpConnect失败".into()))?;

        let request = WinHttpHandle::new(unsafe {
            WinHttpOpenRequest(
                connect.get(),
                METHOD_GET_W.as_ptr(),
                path_w.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                WINHTTP_FLAG_SECURE,
            )
        })
        .ok_or_else(|| BangumiError::Network("WinHttpOpenRequest失败".into()))?;

        let ok: i32 = unsafe {
            WinHttpSendRequest(
                request.get(),
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
                0,
                0,
            )
        };
        if ok == 0 {
            return Err(BangumiError::Network("WinHttpSendRequest失败".into()));
        }

        let ok: i32 = unsafe { WinHttpReceiveResponse(request.get(), std::ptr::null_mut()) };
        if ok == 0 {
            return Err(BangumiError::Network("WinHttpReceiveResponse失败".into()));
        }

        let status_code = unsafe { query_status_code(request.get()) }
            .ok_or_else(|| BangumiError::Network("读取HTTP状态码失败".into()))?;

        if matches!(status_code, 301 | 302 | 303 | 307 | 308) {
            if let Some(location) = unsafe { query_header_string(request.get(), WINHTTP_QUERY_LOCATION) } {
                current = resolve_redirect_url(&current, &location);
                continue;
            }

            return Err(BangumiError::Network("HTTP重定向但缺少Location头".into()));
        }

        if status_code == 404 {
            return Err(BangumiError::NotFound);
        }
        if status_code == 429 {
            return Err(BangumiError::RateLimit);
        }
        if status_code != 200 {
            return Err(BangumiError::Network(format!("HTTP错误: {}", status_code).into()));
        }

        let mut body = Vec::new();
        let mut chunk = Vec::new();
        loop {
            let mut available: u32 = 0;
            let ok: i32 = unsafe { WinHttpQueryDataAvailable(request.get(), &mut available) };
            if ok == 0 {
                return Err(BangumiError::Network("WinHttpQueryDataAvailable失败".into()));
            }
            if available == 0 {
                break;
            }

            let available_usize = available as usize;
            if chunk.len() < available_usize {
                chunk.resize(available_usize, 0);
            }

            let mut read: u32 = 0;
            let ok: i32 = unsafe {
                WinHttpReadData(
                    request.get(),
                    chunk.as_mut_ptr() as *mut _,
                    available,
                    &mut read,
                )
            };
            if ok == 0 {
                return Err(BangumiError::Network("WinHttpReadData失败".into()));
            }
            body.extend_from_slice(&chunk[..read as usize]);
        }

        return String::from_utf8(body)
            .map_err(|e| BangumiError::Parse(format!("响应文本编码错误: {}", e).into()));
    }

    Err(BangumiError::Network("重定向次数过多".into()))
}

#[cfg(target_os = "windows")]
fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
unsafe fn query_status_code(request: HINTERNET) -> Option<u32> {
    let mut status: u32 = 0;
    let mut len: u32 = std::mem::size_of::<u32>() as u32;
    let ok: i32 = unsafe {
        WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            std::ptr::null(),
            &mut status as *mut _ as *mut _,
            &mut len,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        None
    } else {
        Some(status)
    }
}

#[cfg(target_os = "windows")]
unsafe fn query_header_string(request: HINTERNET, query: u32) -> Option<String> {
    let mut len: u32 = 0;
    let ok: i32 = unsafe {
        WinHttpQueryHeaders(
            request,
            query,
            std::ptr::null(),
            std::ptr::null_mut(),
            &mut len,
            std::ptr::null_mut(),
        )
    };
    if ok != 0 {
        return None;
    }
    if len == 0 {
        return None;
    }

    let mut buf: Vec<u16> = vec![0u16; (len as usize + 1) / 2];
    let ok: i32 = unsafe {
        WinHttpQueryHeaders(
            request,
            query,
            std::ptr::null(),
            buf.as_mut_ptr() as *mut _,
            &mut len,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return None;
    }

    if let Some(end) = buf.iter().position(|&c| c == 0) {
        buf.truncate(end);
    }
    Some(String::from_utf16_lossy(&buf))
}

#[cfg(not(windows))]
struct HttpResponse {
    status_code: u16,
    location: Option<String>,
    body: Vec<u8>,
}

#[cfg(not(windows))]
impl HttpResponse {
    fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }
}

#[cfg(not(windows))]
fn parse_http_response(bytes: &[u8]) -> Result<HttpResponse, BangumiError> {
    let header_end = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| BangumiError::Parse("HTTP响应头不完整".into()))?;
    let (header_bytes, body_bytes) = bytes.split_at(header_end + 4);

    let header_text = std::str::from_utf8(header_bytes)
        .map_err(|e| BangumiError::Parse(format!("HTTP头编码错误: {}", e).into()))?;
    let mut lines = header_text.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| BangumiError::Parse("HTTP状态行缺失".into()))?;
    let mut status_parts = status_line.split_whitespace();
    let _http_version = status_parts
        .next()
        .ok_or_else(|| BangumiError::Parse("HTTP版本缺失".into()))?;
    let code_str = status_parts
        .next()
        .ok_or_else(|| BangumiError::Parse("HTTP状态码缺失".into()))?;
    let status_code: u16 = code_str
        .parse()
        .map_err(|_| BangumiError::Parse("HTTP状态码非法".into()))?;

    let mut location: Option<String> = None;
    let mut transfer_chunked = false;
    let mut content_length: Option<usize> = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            let value = v.trim();
            if key.eq_ignore_ascii_case("location") {
                location = Some(value.to_string());
                continue;
            }
            if key.eq_ignore_ascii_case("transfer-encoding") {
                transfer_chunked = value
                    .split(',')
                    .any(|part| part.trim().eq_ignore_ascii_case("chunked"));
                continue;
            }
            if key.eq_ignore_ascii_case("content-length") {
                content_length = value.parse::<usize>().ok();
                continue;
            }
        }
    }

    let body = if transfer_chunked {
        decode_chunked(body_bytes)?
    } else if let Some(len) = content_length {
        body_bytes.get(..len).unwrap_or(body_bytes).to_vec()
    } else {
        body_bytes.to_vec()
    };

    Ok(HttpResponse {
        status_code,
        location,
        body,
    })
}

#[cfg(not(windows))]
fn decode_chunked(mut input: &[u8]) -> Result<Vec<u8>, BangumiError> {
    let mut out = Vec::new();

    loop {
        let line_end = input
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or_else(|| BangumiError::Parse("chunked长度行不完整".into()))?;
        let (size_line, rest) = input.split_at(line_end);
        let rest = &rest[2..];

        let size_str = std::str::from_utf8(size_line)
            .map_err(|e| BangumiError::Parse(format!("chunked长度编码错误: {}", e).into()))?;
        let size_str = size_str.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_str, 16)
            .map_err(|_| BangumiError::Parse("chunked长度非法".into()))?;

        if size == 0 {
            break;
        }

        if rest.len() < size + 2 {
            return Err(BangumiError::Parse("chunked数据不完整".into()));
        }

        out.extend_from_slice(&rest[..size]);
        input = &rest[size + 2..];
    }

    Ok(out)
}

fn resolve_redirect_url(current: &str, location: &str) -> String {
    if location.starts_with("https://") {
        return location.to_string();
    }

    if location.starts_with('/') {
        if let Ok((host, port, _)) = parse_https_url(current) {
            if port == 443 {
                return format!("https://{}{}", host, location);
            }
            return format!("https://{}:{}{}", host, port, location);
        }
    }

    location.to_string()
}

fn percent_encode_path_segment(input: &str) -> String {
    // RFC 3986 unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"
    // Everything else is percent-encoded as UTF-8 bytes.
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~' => out.push(b as char),
            _ => {
                out.push('%');
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push(HEX[(b >> 4) as usize] as char);
                out.push(HEX[(b & 0x0F) as usize] as char);
            }
        }
    }
    out
}

// 直接且明确的API函数
pub fn search_subjects(keywords: &str) -> Result<Vec<Subject>, BangumiError> {
    let trimmed = keywords.trim();
    if trimmed.is_empty() {
        return Err(BangumiError::InvalidInput("搜索关键词不能为空".into()));
    }
    if trimmed.len() > 100 {
        return Err(BangumiError::InvalidInput("搜索关键词过长".into()));
    }
    
    let encoded_keywords = percent_encode_path_segment(trimmed);
    let url = format!(
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

    #[test]
    #[cfg(not(windows))]
    fn test_decode_chunked() {
        let chunked = b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n";
        let decoded = decode_chunked(chunked).expect("decode chunked");
        assert_eq!(decoded, b"Wikipedia");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_parse_http_response_content_length() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhelloEXTRA";
        let resp = parse_http_response(raw).expect("parse http");
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body, b"hello");
    }
}
