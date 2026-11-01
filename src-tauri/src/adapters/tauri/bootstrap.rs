//! Tauri runtime bootstrap and command registration.
use crate::{
    adapters::tauri::commands, application::research_workspace::ResearchWorkspace,
    infrastructure::mt5::bridge::Mt5Bridge, runtime::trading_core::TradingCore,
};
use std::path::PathBuf;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir: PathBuf = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let core = TradingCore::open(app_data_dir.join("agentic-trading-os.sqlite3"))
                .map_err(std::io::Error::other)?;
            let research = ResearchWorkspace::open(app_data_dir.join("research-workspace.sqlite3"))
                .map_err(std::io::Error::other)?;
            app.manage(core);
            app.manage(research);
            app.manage(Mt5Bridge::start(app_data_dir.clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_state,
            commands::get_mt5_bridge_status,
            commands::get_mt5_bridge_setup,
            commands::get_mt5_market_data_status,
            commands::query_mt5_market_data_series,
            commands::get_mt5_instrument_specification,
            commands::get_mt5_session_calendar,
            commands::calculate_mt5_risk_sizing,
            commands::get_market_data_provider_catalog,
            commands::query_unified_market_data,
            commands::get_candle_series,
            commands::place_paper_order,
            commands::run_backtest,
            commands::set_agent_running,
            commands::get_ai_agent_runtime_status,
            commands::save_ai_provider_key,
            commands::remove_ai_provider_key,
            commands::test_ai_provider_connection,
            commands::list_imported_ai_models,
            commands::import_local_ai_model,
            commands::remove_imported_ai_model,
            commands::verify_imported_ai_model,
            commands::verify_ai_agent_audit_chain,
            commands::run_ai_research_agent,
            commands::set_kill_switch,
            commands::reset_paper_account,
            commands::update_risk_limits,
            commands::get_audit_history,
            commands::refresh_public_market_data,
            commands::refresh_public_screener,
            commands::get_meme_coin_hunter_security_status,
            commands::save_meme_coin_hunter_security_token,
            commands::remove_meme_coin_hunter_security_token,
            commands::run_meme_coin_hunter,
            commands::get_latest_meme_coin_hunter_report,
            commands::create_strict_analysis_work_plan,
            commands::evaluate_strict_analysis_completion,
            commands::verify_strict_analysis_audit_chain,
            commands::create_research_hypothesis,
            commands::list_research_hypotheses,
            commands::invalidate_research_hypothesis,
            commands::execute_research_workflow,
            commands::list_research_run_cards,
            commands::get_research_run_card,
            commands::verify_research_workspace
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agentic Trading OS");
}
