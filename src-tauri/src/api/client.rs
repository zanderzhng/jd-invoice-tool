use crate::utils::file::read_user_agent_or_default;
use reqwest::Client;

pub fn build_client(cookie: &str) -> Result<Client, String> {
    let user_agent = read_user_agent_or_default()?;

    Client::builder()
        .user_agent(user_agent)
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "Accept",
                "application/json, text/plain, */*".parse().unwrap(),
            );
            headers.insert("Accept-Language", "zh-CN,zh-Hans;q=0.9".parse().unwrap());
            headers.insert("Referer", "https://invoice-m.jd.com/".parse().unwrap());
            headers.insert("Origin", "https://invoice-m.jd.com".parse().unwrap());
            headers.insert("Sec-Fetch-Site", "same-site".parse().unwrap());
            headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
            headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
            headers.insert("x-source", "2".parse().unwrap());
            headers.insert("x-source-id", "0".parse().unwrap());
            headers.insert("Cookie", cookie.parse().unwrap());
            headers
        })
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}
