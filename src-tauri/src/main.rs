#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod api;
mod utils;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    
    log::info!("JD Invoice App starting...");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::browser::open_browser,
            commands::login_window::open_login_window_clean,
            commands::login_window::close_login_window_clean,
            commands::login_window::clear_login_window_cookies,
            commands::login_window::clear_login_window_cache,
            commands::login_window::is_login_window_clean_open,
            commands::login_window::has_login_succeeded,
            commands::login_window::extract_cookies_clean,
            commands::cookie::save_cookie,
            commands::cookie::get_cookie,
            commands::cookie::delete_cookie,
            commands::invoice::fetch_invoices,
            commands::invoice::fetch_invoice_detail,
            commands::exchange::fetch_batch_orders,
            commands::exchange::fetch_batch_order_amounts,
            commands::exchange::check_merge,
            commands::exchange::submit_merge_exchange,
            commands::download::download_invoice,
            commands::title::get_titles,
            commands::title::save_titles,
            commands::user_agent::get_user_agent,
            commands::user_agent::save_user_agent,
            commands::user_agent::reset_user_agent,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
