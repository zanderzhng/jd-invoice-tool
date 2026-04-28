use tauri::command;
use std::fs;
use std::path::PathBuf;
use crate::utils::file::{read_cookie_file, read_user_agent_or_default};

#[command]
pub async fn download_invoice(url: String, filename: String) -> Result<String, String> {
    let cookie = read_cookie_file()?.ok_or("Cookie not found, please login first")?;
    let user_agent = read_user_agent_or_default()?;

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let res = client
        .get(&url)
        .header("Cookie", &cookie)
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let download_dir = dirs::download_dir().unwrap_or_else(|| PathBuf::from("."));
    let file_path = download_dir.join(&filename);

    fs::write(&file_path, &bytes)
        .map_err(|e| format!("Failed to save file: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}
