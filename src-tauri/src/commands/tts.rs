//! Settings commands for the voice (TTS) engines.

use crate::settings::{get_settings, write_settings, AppSettings, TtsEngine};
use crate::tts::{self, TtsConfig, TtsVoiceInfo};
use tauri::AppHandle;

fn update_tts_settings(app: &AppHandle, apply: impl FnOnce(&mut AppSettings)) -> Result<(), String> {
    let mut settings = get_settings(app);
    apply(&mut settings);
    write_settings(app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_tts_engine_setting(app: AppHandle, engine: TtsEngine) -> Result<(), String> {
    update_tts_settings(&app, |s| s.tts_engine = engine)
}

#[tauri::command]
#[specta::specta]
pub fn change_tts_sapi_voice_setting(app: AppHandle, voice_id: Option<String>) -> Result<(), String> {
    update_tts_settings(&app, |s| s.tts_sapi_voice_id = voice_id)
}

#[tauri::command]
#[specta::specta]
pub fn change_tts_sapi_rate_setting(app: AppHandle, rate: i32) -> Result<(), String> {
    update_tts_settings(&app, |s| s.tts_sapi_rate = rate.clamp(-10, 10))
}

#[tauri::command]
#[specta::specta]
pub fn change_tts_sapi_pitch_setting(app: AppHandle, pitch: i32) -> Result<(), String> {
    update_tts_settings(&app, |s| s.tts_sapi_pitch = pitch.clamp(-10, 10))
}

#[tauri::command]
#[specta::specta]
pub fn change_tts_sapi_volume_setting(app: AppHandle, volume: u32) -> Result<(), String> {
    update_tts_settings(&app, |s| s.tts_sapi_volume = volume.clamp(0, 100))
}

#[tauri::command]
#[specta::specta]
pub fn set_tts_openai_api_key_setting(app: AppHandle, api_key: String) -> Result<(), String> {
    update_tts_settings(&app, |s| {
        s.tts_openai_api_key.insert("openai".to_string(), api_key);
    })
}

#[tauri::command]
#[specta::specta]
pub fn change_tts_openai_voice_setting(app: AppHandle, voice: String) -> Result<(), String> {
    update_tts_settings(&app, |s| s.tts_openai_voice = voice)
}

#[tauri::command]
#[specta::specta]
pub fn change_tts_openai_model_setting(app: AppHandle, model: String) -> Result<(), String> {
    update_tts_settings(&app, |s| s.tts_openai_model = model)
}

#[tauri::command]
#[specta::specta]
pub fn change_tts_openai_speed_setting(app: AppHandle, speed: f64) -> Result<(), String> {
    update_tts_settings(&app, |s| s.tts_openai_speed = speed.clamp(0.25, 4.0))
}

#[tauri::command]
#[specta::specta]
pub fn set_tts_elevenlabs_api_key_setting(app: AppHandle, api_key: String) -> Result<(), String> {
    update_tts_settings(&app, |s| {
        s.tts_elevenlabs_api_key
            .insert("elevenlabs".to_string(), api_key);
    })
}

#[tauri::command]
#[specta::specta]
pub fn change_tts_elevenlabs_voice_id_setting(app: AppHandle, voice_id: String) -> Result<(), String> {
    update_tts_settings(&app, |s| s.tts_elevenlabs_voice_id = voice_id)
}

#[tauri::command]
#[specta::specta]
pub fn change_tts_elevenlabs_model_setting(app: AppHandle, model: String) -> Result<(), String> {
    update_tts_settings(&app, |s| s.tts_elevenlabs_model = model)
}

/// The installed Windows SAPI voices for the engine's voice dropdown.
#[tauri::command]
#[specta::specta]
pub fn list_tts_sapi_voices() -> Vec<TtsVoiceInfo> {
    tts::list_sapi_voices()
}

/// Speak the sample sentence with the current settings and return any error,
/// so the settings screen can surface engine problems (missing keys, bad
/// voice ids, network failures) directly to the user.
#[tauri::command]
#[specta::specta]
pub async fn test_tts_voice(app: AppHandle) -> Result<(), String> {
    let settings = get_settings(&app);
    let cfg = TtsConfig::from_settings(&settings);
    tauri::async_runtime::spawn_blocking(move || {
        tts::test_voice_blocking("This is Aira Flow speaking with the selected voice.", &cfg)
    })
    .await
    .map_err(|e| format!("TTS test failed: {e}"))?
}

/// Analyze a voice-sample audio file (WAV/MP3/M4A) and suggest the closest
/// built-in Windows voice + pitch offset for the "Match my voice" flow.
#[tauri::command]
#[specta::specta]
pub async fn analyze_voice_sample_setting(path: String) -> Result<tts::VoiceSampleAnalysis, String> {
    tauri::async_runtime::spawn_blocking(move || tts::analyze_voice_sample(&std::path::PathBuf::from(path)))
        .await
        .map_err(|e| format!("Voice sample analysis failed: {e}"))?
}

