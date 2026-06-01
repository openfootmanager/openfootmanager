use log::info;
use ofm_core::currency::{self, CurrencyDefinition};
use tauri::Manager as TauriManager;

const SETTINGS_LOAD_FAILED_ERROR: &str = "be.error.settings.loadFailed";
const SETTINGS_PARSE_FAILED_ERROR: &str = "be.error.settings.parseFailed";
const SETTINGS_SAVE_FAILED_ERROR: &str = "be.error.settings.saveFailed";
const SAVE_MANAGER_UNAVAILABLE_ERROR: &str = "be.error.saveManagerUnavailable";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub theme: String, // "dark" | "light" | "system"
    #[serde(default = "default_language")]
    pub language: String, // "en" | "es" | "pt" | "fr" | "de"
    pub currency: String, // "EUR" | "GBP" | "USD"
    pub default_match_mode: String, // "live" | "spectator" | "delegate"
    pub auto_save: bool,
    pub match_speed: String, // "slow" | "normal" | "fast"
    pub show_match_commentary: bool,
    pub confirm_advance: bool,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: String, // "small" | "normal" | "large" | "xlarge"
    #[serde(default)]
    pub high_contrast: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AppSettingsResponse {
    pub settings: AppSettings,
    pub currency: CurrencyDefinition,
    pub supported_currencies: Vec<CurrencyDefinition>,
}

fn default_language() -> String {
    "en".to_string()
}
fn default_ui_scale() -> String {
    "normal".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            language: "en".to_string(),
            currency: "EUR".to_string(),
            default_match_mode: "live".to_string(),
            auto_save: true,
            match_speed: "normal".to_string(),
            show_match_commentary: true,
            confirm_advance: false,
            ui_scale: "normal".to_string(),
            high_contrast: false,
        }
    }
}

fn response_for_settings(settings: AppSettings) -> AppSettingsResponse {
    let currency = currency::currency_definition(&settings.currency)
        .unwrap_or_else(|| currency::currency_definition(currency::DEFAULT_CURRENCY_CODE).unwrap());

    AppSettingsResponse {
        settings,
        currency,
        supported_currencies: currency::supported_currencies(),
    }
}

fn normalize_loaded_settings(mut settings: AppSettings) -> AppSettings {
    settings.currency = currency::normalize_currency_code(&settings.currency)
        .unwrap_or(currency::DEFAULT_CURRENCY_CODE)
        .to_string();
    settings
}

fn validate_settings(mut settings: AppSettings) -> Result<AppSettings, String> {
    settings.currency = currency::normalize_currency_code(&settings.currency)
        .ok_or("be.error.settings.invalidCurrency".to_string())?
        .to_string();
    Ok(settings)
}

fn settings_path(
    app_handle: &tauri::AppHandle,
    error_key: &str,
) -> Result<std::path::PathBuf, String> {
    let dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|_| error_key.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|_| error_key.to_string())?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub fn get_settings(app_handle: tauri::AppHandle) -> Result<AppSettingsResponse, String> {
    log::debug!("[cmd] get_settings");
    let path = settings_path(&app_handle, SETTINGS_LOAD_FAILED_ERROR)?;
    if !path.exists() {
        return Ok(response_for_settings(AppSettings::default()));
    }
    let json = std::fs::read_to_string(&path).map_err(|_| SETTINGS_LOAD_FAILED_ERROR.to_string())?;
    let settings = serde_json::from_str(&json).map_err(|_| SETTINGS_PARSE_FAILED_ERROR.to_string())?;
    Ok(response_for_settings(normalize_loaded_settings(settings)))
}

#[tauri::command]
pub fn save_settings(app_handle: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    let settings = validate_settings(settings)?;
    info!(
        "[cmd] save_settings: theme={}, lang={}",
        settings.theme, settings.language
    );
    let path = settings_path(&app_handle, SETTINGS_SAVE_FAILED_ERROR)?;
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|_| SETTINGS_SAVE_FAILED_ERROR.to_string())?;
    std::fs::write(&path, json).map_err(|_| SETTINGS_SAVE_FAILED_ERROR.to_string())
}

#[tauri::command]
pub fn clear_all_saves(sm_state: tauri::State<crate::SaveManagerState>) -> Result<(), String> {
    log::warn!("[cmd] clear_all_saves: deleting all save data!");
    let mut sm = sm_state
        .0
        .lock()
        .map_err(|_| SAVE_MANAGER_UNAVAILABLE_ERROR.to_string())?;
    let save_ids: Vec<String> = sm.load_saves()?.into_iter().map(|s| s.id).collect();
    for id in save_ids {
        sm.delete_save(&id)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, normalize_loaded_settings, response_for_settings, validate_settings};

    fn make_settings(currency: &str) -> AppSettings {
        AppSettings {
            currency: currency.to_string(),
            ..AppSettings::default()
        }
    }

    #[test]
    fn normalizes_loaded_settings_to_supported_currency() {
        let settings = normalize_loaded_settings(make_settings("gbp"));

        assert_eq!(settings.currency, "GBP");
    }

    #[test]
    fn falls_back_to_default_currency_when_loaded_settings_are_invalid() {
        let settings = normalize_loaded_settings(make_settings("CAD"));

        assert_eq!(settings.currency, "EUR");
    }

    #[test]
    fn rejects_invalid_currency_when_saving_settings() {
        let result = validate_settings(make_settings("CAD"));

        assert_eq!(result.unwrap_err(), "be.error.settings.invalidCurrency");
    }

    #[test]
    fn includes_supported_currency_metadata_in_settings_response() {
        let response = response_for_settings(make_settings("USD"));

        assert_eq!(response.currency.code, "USD");
        assert_eq!(response.currency.symbol, "$");
        assert_eq!(response.supported_currencies.len(), 3);
    }
}
