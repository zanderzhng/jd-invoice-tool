use std::fs;
use std::path::PathBuf;

pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.3 Mobile/15E148 Safari/604.1";

fn get_app_data_dir() -> Result<PathBuf, String> {
    let app_data = dirs::data_local_dir()
        .ok_or("Cannot find app data directory")?
        .join("jd-invoice-app");

    if !app_data.exists() {
        fs::create_dir_all(&app_data).map_err(|e| format!("Failed to create data dir: {}", e))?;
    }

    Ok(app_data)
}

fn get_cookie_file_path() -> Result<PathBuf, String> {
    Ok(get_app_data_dir()?.join("cookie.txt"))
}

fn get_titles_file_path() -> Result<PathBuf, String> {
    Ok(get_app_data_dir()?.join("titles.json"))
}

fn get_user_agent_file_path() -> Result<PathBuf, String> {
    Ok(get_app_data_dir()?.join("user_agent.txt"))
}

pub fn read_cookie_file() -> Result<Option<String>, String> {
    let path = get_cookie_file_path()?;
    if path.exists() {
        let content =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read cookie file: {}", e))?;
        Ok(Some(content.trim().to_string()))
    } else {
        Ok(None)
    }
}

pub fn write_cookie_file(cookie: &str) -> Result<(), String> {
    let path = get_cookie_file_path()?;
    fs::write(&path, cookie).map_err(|e| format!("Failed to write cookie file: {}", e))
}

pub fn delete_cookie_file() -> Result<(), String> {
    let path = get_cookie_file_path()?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete cookie file: {}", e))?;
    }
    Ok(())
}

pub fn read_titles_file() -> Result<String, String> {
    let path = get_titles_file_path()?;
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("Failed to read titles file: {}", e))
    } else {
        Ok("[]".to_string())
    }
}

pub fn write_titles_file(content: &str) -> Result<(), String> {
    let path = get_titles_file_path()?;
    fs::write(&path, content).map_err(|e| format!("Failed to write titles file: {}", e))
}

pub fn read_user_agent_file() -> Result<Option<String>, String> {
    let path = get_user_agent_file_path()?;
    if path.exists() {
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read user agent file: {}", e))?;
        let user_agent = content.trim().to_string();
        if user_agent.is_empty() {
            Ok(None)
        } else {
            Ok(Some(user_agent))
        }
    } else {
        Ok(None)
    }
}

pub fn read_user_agent_or_default() -> Result<String, String> {
    Ok(read_user_agent_file()?.unwrap_or_else(|| DEFAULT_USER_AGENT.to_string()))
}

pub fn write_user_agent_file(user_agent: &str) -> Result<(), String> {
    let path = get_user_agent_file_path()?;
    fs::write(&path, user_agent.trim())
        .map_err(|e| format!("Failed to write user agent file: {}", e))
}

pub fn delete_user_agent_file() -> Result<(), String> {
    let path = get_user_agent_file_path()?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete user agent file: {}", e))?;
    }
    Ok(())
}
