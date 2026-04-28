use crate::utils::file::{
    delete_user_agent_file, read_user_agent_or_default, write_user_agent_file, DEFAULT_USER_AGENT,
};
use reqwest::header::HeaderValue;
use tauri::command;

#[command]
pub async fn get_user_agent() -> Result<String, String> {
    read_user_agent_or_default()
}

#[command]
pub async fn save_user_agent(user_agent: String) -> Result<String, String> {
    let trimmed = user_agent.trim();
    if trimmed.is_empty() {
        return Err("User-Agent 不能为空".to_string());
    }

    HeaderValue::from_str(trimmed).map_err(|_| "User-Agent 包含无效字符".to_string())?;
    write_user_agent_file(trimmed)?;
    Ok(trimmed.to_string())
}

#[command]
pub async fn reset_user_agent() -> Result<String, String> {
    delete_user_agent_file()?;
    Ok(DEFAULT_USER_AGENT.to_string())
}
