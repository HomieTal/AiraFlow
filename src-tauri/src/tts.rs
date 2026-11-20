//! Text-to-speech engines for spoken answers.
//!
//! Three engines: the built-in Windows SAPI voice (voice/rate/pitch/volume
//! configurable, with a "match my voice" pitch approximation), OpenAI's TTS
//! HTTP API, and ElevenLabs. All engines speak on a background thread and are
//! best-effort: failures are logged, never surfaced into the dictation flow —
//! except `test_voice`, used by the settings screen to report errors back.

use log::{info, warn};
#[cfg(windows)]
use regex::Regex;
#[cfg(windows)]
use windows::core::Interface;

use crate::settings::{AppSettings, TtsEngine};

/// NUL-terminated UTF-16 for SAPI string parameters.
#[cfg(windows)]
fn wide0(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}


/// The resolved TTS configuration for one utterance.
#[derive(Clone, Debug)]
pub struct TtsConfig {
    pub engine: TtsEngine,
    pub sapi_voice_id: Option<String>,
    pub sapi_rate: i32,
    pub sapi_pitch: i32,
    pub sapi_volume: u32,
    pub openai_api_key: String,
    pub openai_voice: String,
    pub openai_model: String,
    pub openai_speed: f64,
    pub elevenlabs_api_key: String,
    pub elevenlabs_voice_id: String,
    pub elevenlabs_model: String,
}

impl TtsConfig {
    pub fn from_settings(s: &AppSettings) -> Self {
        Self {
            engine: s.tts_engine,
            sapi_voice_id: s.tts_sapi_voice_id.clone(),
            sapi_rate: s.tts_sapi_rate,
            sapi_pitch: s.tts_sapi_pitch,
            sapi_volume: s.tts_sapi_volume,
            openai_api_key: s
                .tts_openai_api_key
                .get("openai")
                .cloned()
                .unwrap_or_default(),
            openai_voice: s.tts_openai_voice.clone(),
            openai_model: s.tts_openai_model.clone(),
            openai_speed: s.tts_openai_speed,
            elevenlabs_api_key: s
                .tts_elevenlabs_api_key
                .get("elevenlabs")
                .cloned()
                .unwrap_or_default(),
            elevenlabs_voice_id: s.tts_elevenlabs_voice_id.clone(),
            elevenlabs_model: s.tts_elevenlabs_model.clone(),
        }
    }
}

/// One installed Windows SAPI voice.
#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct TtsVoiceInfo {
    pub id: String,
    pub name: String,
    pub gender: Option<String>,
}

/// Result of analyzing a user's voice sample for the "Match my voice" flow.
#[derive(Clone, Debug, serde::Serialize, specta::Type)]
pub struct VoiceSampleAnalysis {
    /// Median fundamental frequency of the sample (Hz), when enough voiced
    /// frames were detected.
    pub detected_f0: Option<f64>,
    /// The SAPI voice token id that best matches the detected voice.
    pub suggested_voice_id: Option<String>,
    /// SAPI pitch delta to approximate the detected voice.
    pub suggested_pitch: i32,
}

/// Speak `text` asynchronously on a background thread, honoring `cfg`.
/// Fire-and-forget: the caller continues immediately; failures are logged.
pub fn speak_text(text: &str, cfg: &TtsConfig) {
    let text = strip_markdown_for_speech(text);
    if text.trim().is_empty() {
        return;
    }
    let cfg = cfg.clone();
    std::thread::Builder::new()
        .name("tts".into())
        .spawn(move || {
            let result = dispatch(&text, &cfg);
            match result {
                Ok(()) => info!(
                    "Spoken answer finished ({} chars, engine {:?})",
                    text.chars().count(),
                    cfg.engine
                ),
                Err(err) => warn!("Failed to speak answer aloud: {err}"),
            }
        })
        .map_err(|e| warn!("Failed to spawn TTS thread: {e}"))
        .ok();
}

/// Speak `text` synchronously and report failures to the caller. Used by the
/// settings screen's test button, where the error must reach the UI.
pub fn test_voice_blocking(text: &str, cfg: &TtsConfig) -> Result<(), String> {
    let text = strip_markdown_for_speech(text);
    if text.trim().is_empty() {
        return Ok(());
    }
    dispatch(&text, cfg).map_err(|e| format!("{e:#}"))
}

fn dispatch(text: &str, cfg: &TtsConfig) -> anyhow::Result<()> {
    match cfg.engine {
        TtsEngine::Windows => speak_sapi_blocking(text, cfg),
        TtsEngine::OpenAi => speak_openai_blocking(text, cfg),
        TtsEngine::ElevenLabs => speak_elevenlabs_blocking(text, cfg),
    }
}

// ---------------------------------------------------------------- Windows SAPI

#[cfg(windows)]
fn speak_sapi_blocking(text: &str, cfg: &TtsConfig) -> anyhow::Result<()> {
    use windows::core::PCWSTR;
    use windows::Win32::Media::Speech::{ISpVoice, SpVoice};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };

    // SAPI wants an STA thread. If some earlier library already initialized
    // this thread as MTA, CoInitializeEx reports RPC_E_CHANGED_MODE — that is
    // fine to continue on, but we must not CoUninitialize in that case.
    let com_init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let owns_com = com_init.is_ok();

    let spoken = (|| -> anyhow::Result<()> {
        let voice: ISpVoice = unsafe { CoCreateInstance(&SpVoice, None, CLSCTX_ALL) }
            .map_err(|e| anyhow::anyhow!("CoCreateInstance(SpVoice): {e}"))?;

        unsafe {
            voice
                .SetVolume(cfg.sapi_volume as u16)
                .map_err(|e| anyhow::anyhow!("ISpVoice::SetVolume: {e}"))?;
            voice
                .SetRate(cfg.sapi_rate)
                .map_err(|e| anyhow::anyhow!("ISpVoice::SetRate: {e}"))?;
            if let Some(voice_id) = &cfg.sapi_voice_id {
                let token = sapi_token_by_id(voice_id)
                    .map_err(|e| anyhow::anyhow!("voice '{voice_id}' not found: {e}"))?;
                voice
                    .SetVoice(&token)
                    .map_err(|e| anyhow::anyhow!("ISpVoice::SetVoice: {e}"))?;
            }
        }

        // SAPI's XML parser is kept on (SPF_DEFAULT) because its
        // SPF_IS_NOT_XML path fails outright on some SAPI configurations with
        // INET_E_UNKNOWN_PROTOCOL (0x800C000D) — even for plain ASCII text —
        // so the XML-sensitive characters are escaped ourselves instead.
        // The <pitch> wrapper is applied after escaping so the delta is the
        // only markup: it tunes the voice toward the user's own pitch.
        let mut safe = escape_for_sapi_xml(text);
        if cfg.sapi_pitch != 0 {
            safe = format!(
                "<pitch middle=\"{}\">{}</pitch>",
                if cfg.sapi_pitch > 0 {
                    format!("+{}", cfg.sapi_pitch)
                } else {
                    cfg.sapi_pitch.to_string()
                },
                safe
            );
        }
        let wide: Vec<u16> = safe.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            voice
                .Speak(
                    PCWSTR::from_raw(wide.as_ptr()),
                    0u32,
                    Some(std::ptr::null_mut()),
                )
                .map_err(|e| anyhow::anyhow!("ISpVoice::Speak: {e}"))?;
        }
        Ok(())
    })();

    if owns_com {
        unsafe { CoUninitialize() };
    }
    spoken
}

#[cfg(not(windows))]
fn speak_sapi_blocking(_text: &str, _cfg: &TtsConfig) -> anyhow::Result<()> {
    anyhow::bail!("The Windows voice engine is only available on Windows")
}

/// Enumerate the installed SAPI voices with their display names and genders.
#[cfg(windows)]
pub fn list_sapi_voices() -> Vec<TtsVoiceInfo> {
    use windows::core::{BSTR, PCWSTR};
    use windows::Win32::Media::Speech::{ISpVoice, ISpeechVoice, SpVoice};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };

    let mut voices = Vec::new();
    let com_init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let owns_com = com_init.is_ok();
    let _ = (|| -> anyhow::Result<()> {
        // The native ISpVoice in windows 0.61 does not expose GetVoices; the
        // dual automation interface ISpeechVoice (same coclass) does, and its
        // automation tokens QI back to ISpObjectToken for SetVoice.
        let voice: ISpVoice = unsafe { CoCreateInstance(&SpVoice, None, CLSCTX_ALL) }
            .map_err(|e| anyhow::anyhow!("CoCreateInstance(SpVoice): {e}"))?;
        let speech_voice: ISpeechVoice = voice
            .cast::<ISpeechVoice>()
            .map_err(|e| anyhow::anyhow!("cast to ISpeechVoice: {e}"))?;
        let tokens = unsafe { speech_voice.GetVoices(&BSTR::new(), &BSTR::new()) }
            .map_err(|e| anyhow::anyhow!("GetVoices: {e}"))?;
        let count = unsafe { tokens.Count() }.map_err(|e| anyhow::anyhow!("Count: {e}"))?;
        for i in 0..count.max(0) {
            let token = unsafe { tokens.Item(i) }
                .map_err(|e| anyhow::anyhow!("voice item {i}: {e}"))?;
            let id = unsafe { token.Id() }
                .map(|b| b.to_string())
                .unwrap_or_default();
            let name = unsafe {
                token.GetAttribute(&BSTR::from("Name"))
            }
            .map(|b| b.to_string())
            .unwrap_or_else(|_| id.clone());
            let gender = unsafe {
                token.GetAttribute(&BSTR::from("Gender"))
            }
            .map(|b| b.to_string())
            .ok();
            voices.push(TtsVoiceInfo {
                id,
                name,
                gender,
            });
        }
        Ok(())
    })();
    if owns_com {
        unsafe { CoUninitialize() };
    }
    voices
}

/// Non-Windows stub: no SAPI voices to enumerate.
#[cfg(not(windows))]
pub fn list_sapi_voices() -> Vec<TtsVoiceInfo> {
    Vec::new()
}

/// Find a SAPI voice token by its registry-style id (e.g.
/// `TTS_MS_EN-US_DAVID_11.0`).
#[cfg(windows)]
fn sapi_token_by_id(
    voice_id: &str,
) -> anyhow::Result<windows::Win32::Media::Speech::ISpObjectToken> {
    use windows::core::{BSTR, PCWSTR};
    use windows::Win32::Media::Speech::{ISpObjectToken, ISpVoice, ISpeechVoice, SpVoice};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    let voice: ISpVoice = unsafe { CoCreateInstance(&SpVoice, None, CLSCTX_ALL) }
        .map_err(|e| anyhow::anyhow!("CoCreateInstance(SpVoice): {e}"))?;
    let speech_voice: ISpeechVoice = voice
        .cast::<ISpeechVoice>()
        .map_err(|e| anyhow::anyhow!("cast to ISpeechVoice: {e}"))?;
    let tokens = unsafe { speech_voice.GetVoices(&BSTR::new(), &BSTR::new()) }
        .map_err(|e| anyhow::anyhow!("GetVoices: {e}"))?;
    let count = unsafe { tokens.Count() }.map_err(|e| anyhow::anyhow!("Count: {e}"))?;
    for i in 0..count.max(0) {
        let token = unsafe { tokens.Item(i) }
            .map_err(|e| anyhow::anyhow!("voice item {i}: {e}"))?;
        let id = unsafe { token.Id() }
            .map(|b| b.to_string())
            .unwrap_or_default();
        if id == voice_id {
            return token
                .cast::<ISpObjectToken>()
                .map_err(|e| anyhow::anyhow!("token cast to ISpObjectToken: {e}"));
        }
    }
    anyhow::bail!("voice id {voice_id} is not installed")
}

#[cfg(windows)]
fn pcwstr_to_string(p: windows::core::PCWSTR) -> String {
    if p.0.is_null() {
        return String::new();
    }
    unsafe {
        let mut len = 0usize;
        while *p.0.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p.0, len))
    }
}

// ------------------------------------------------------------- cloud engines

fn speak_openai_blocking(text: &str, cfg: &TtsConfig) -> anyhow::Result<()> {
    if cfg.openai_api_key.is_empty() {
        anyhow::bail!("No OpenAI API key configured for the OpenAI voice engine");
    }
    let body = serde_json::json!({
        "model": cfg.openai_model,
        "voice": cfg.openai_voice,
        "input": text,
        "speed": cfg.openai_speed,
        "response_format": "mp3",
    });
    let bytes = tts_http_json(
        "https://api.openai.com/v1/audio/speech",
        &body,
        Some(("Authorization", format!("Bearer {}", cfg.openai_api_key))),
    )?;
    play_mp3(&bytes)
}

fn speak_elevenlabs_blocking(text: &str, cfg: &TtsConfig) -> anyhow::Result<()> {
    if cfg.elevenlabs_api_key.is_empty() {
        anyhow::bail!("No ElevenLabs API key configured for the ElevenLabs voice engine");
    }
    if cfg.elevenlabs_voice_id.is_empty() {
        anyhow::bail!("No ElevenLabs voice id configured for the ElevenLabs voice engine");
    }
    let url = format!(
        "https://api.elevenlabs.io/v1/text-to-speech/{}?output_format=mp3_44100_128",
        cfg.elevenlabs_voice_id
    );
    let body = serde_json::json!({
        "text": text,
        "model_id": cfg.elevenlabs_model,
    });
    let bytes = tts_http_json(
        &url,
        &body,
        Some(("xi-api-key", cfg.elevenlabs_api_key.clone())),
    )?;
    play_mp3(&bytes)
}

/// POST a JSON body, expect MP3 bytes back, and play them on the default
/// output device through the app's rodio stack (honors the mp3 feature).
fn tts_http_json(
    url: &str,
    body: &serde_json::Value,
    auth_header: Option<(&str, String)>,
) -> anyhow::Result<Vec<u8>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("TTS runtime: {e}"))?;
    let bytes = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| anyhow::anyhow!("HTTP client: {e}"))?;
        let mut request = client.post(url).json(body);
        if let Some((header, value)) = auth_header {
            request = request.header(header, value);
        }
        let response = request
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("request failed: {e}"))?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("{}: {body}", status));
        }
        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| anyhow::anyhow!("audio download failed: {e}"))
    })?;
    if bytes.is_empty() {
        anyhow::bail!("the speech service returned no audio");
    }
    Ok(bytes)
}

/// Play MP3 bytes on the default output device, blocking until done (same
/// rodio stack the feedback sounds use, so the mp3 feature is already on).
fn play_mp3(bytes: &[u8]) -> anyhow::Result<()> {
    let tmp = std::env::temp_dir().join(format!("aira-flow-tts-{}.mp3", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    let result = (|| -> anyhow::Result<()> {
        use rodio::DeviceSinkBuilder;
        let device_sink = DeviceSinkBuilder::from_default_device()
            .map_err(|e| anyhow::anyhow!("audio output: {e}"))?
            .open_stream()
            .map_err(|e| anyhow::anyhow!("audio stream: {e}"))?;
        let mixer = device_sink.mixer();
        let file = std::fs::File::open(&tmp).map_err(|e| anyhow::anyhow!("open temp: {e}"))?;
        let player = rodio::play(mixer, std::io::BufReader::new(file))
            .map_err(|e| anyhow::anyhow!("decode mp3: {e}"))?;
        player.sleep_until_end();
        Ok(())
    })();
    let _ = std::fs::remove_file(&tmp);
    result
}

// ---------------------------------------------------------------- pitch match

/// Analyze a voice-sample audio file (WAV/MP3/M4A — symphonia decodes them
/// all) and suggest the closest built-in Windows voice + pitch offset.
#[cfg(windows)]
pub fn analyze_voice_sample(path: &std::path::Path) -> Result<VoiceSampleAnalysis, String> {
    let samples =
        crate::audio_toolkit::audio::decode::decode_audio_file(path).map_err(|e| e.to_string())?;
    let detected_f0 = estimate_pitch(&samples, 16_000);
    let voices = list_sapi_voices();
    let Some(f0) = detected_f0 else {
        return Ok(VoiceSampleAnalysis {
            detected_f0: None,
            suggested_voice_id: None,
            suggested_pitch: 0,
        });
    };

    // Human voices: male ≈ 85-180 Hz, female ≈ 160-300 Hz. The split sits at
    // 155 Hz. SAPI pitch deltas span ±10, each step ≈ a few percent, so the
    // delta maps the offset between the detected pitch and the voice's
    // natural centre onto the available range.
    let male = f0 < 155.0;
    let center = if male { 120.0 } else { 190.0 };
    let want_gender = if male { "male" } else { "female" };

    let suggested_voice_id = voices
        .iter()
        .find(|v| {
            v.gender
                .as_deref()
                .map(|g| g.eq_ignore_ascii_case(want_gender))
                .unwrap_or(false)
        })
        .map(|v| v.id.clone())
        .or_else(|| voices.first().map(|v| v.id.clone()));

    let delta = (((f0 - center) / center) * 25.0).round();
    let suggested_pitch = delta.clamp(-10.0, 10.0) as i32;

    Ok(VoiceSampleAnalysis {
        detected_f0: Some(f0),
        suggested_voice_id,
        suggested_pitch,
    })
}

/// Non-Windows stub: the "Match my voice" flow needs the SAPI engine.
#[cfg(not(windows))]
pub fn analyze_voice_sample(_path: &std::path::Path) -> Result<VoiceSampleAnalysis, String> {
    Err("The match-my-voice flow is only available on Windows".to_string())
}

/// Median fundamental frequency over voiced 40 ms frames via per-lag
/// normalized autocorrelation. `samples` are mono at `sample_rate` Hz.
#[cfg(windows)]
fn estimate_pitch(samples: &[f32], sample_rate: u32) -> Option<f64> {
    let frame = 1024usize;
    let hop = frame / 2;
    let min_lag = (sample_rate as usize / 400).max(2);
    let max_lag = (sample_rate as usize / 70).min(frame / 2);
    let mut f0s: Vec<f64> = Vec::new();

    let mut start = 0usize;
    while start + frame <= samples.len() {
        let window = &samples[start..start + frame];
        let rms = (window.iter().map(|s| s * s).sum::<f32>() / frame as f32).sqrt();
        if rms > 0.005 {
            let mut best_corr = 0f32;
            let mut best_lag = 0usize;
            for lag in min_lag..=max_lag {
                let mut corr = 0f32;
                for j in 0..frame - lag {
                    corr += window[j] * window[j + lag];
                }
                corr /= (frame - lag) as f32;
                if corr > best_corr {
                    best_corr = corr;
                    best_lag = lag;
                }
            }
            if best_lag >= min_lag && best_corr > 1e-4 {
                f0s.push(sample_rate as f64 / best_lag as f64);
            }
        }
        start += hop;
    }

    if f0s.len() < 5 {
        return None;
    }
    f0s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(f0s[f0s.len() / 2])
}

/// Render markdown answers as speakable prose: LLM output typically carries
/// headings, emphasis markers, fenced code blocks and link syntax that SAPI
/// would otherwise read out literally ("asterisk asterisk...").
#[cfg(windows)]
fn strip_markdown_for_speech(text: &str) -> String {
    // Fenced code blocks are implementation detail, not prose.
    let out = Regex::new(r"(?s)```[^\n]*\n.*?```")
        .and_then(|r| Ok(r.replace_all(text, " ")))
        .unwrap_or_else(|_| text.into());
    let out = Regex::new(r"`([^`]*)`")
        .and_then(|r| Ok(r.replace_all(&out, "$1")))
        .unwrap_or(out.clone());
    // Images: keep alt text; links: keep label, drop the URL.
    let out = Regex::new(r"!\[([^\]]*)\]\([^)]*\)")
        .and_then(|r| Ok(r.replace_all(&out, "$1")))
        .unwrap_or(out.clone());
    let out = Regex::new(r"\[([^\]]+)\]\([^)]*\)")
        .and_then(|r| Ok(r.replace_all(&out, "$1")))
        .unwrap_or(out.clone());
    let out = Regex::new(r"(?m)^#{1,6}\s+")
        .and_then(|r| Ok(r.replace_all(&out, "")))
        .unwrap_or(out.clone());
    let out = Regex::new(r"(?m)^\s*>\s?")
        .and_then(|r| Ok(r.replace_all(&out, "")))
        .unwrap_or(out.clone());
    let out = Regex::new(r"\*\*([^*]+)\*\*")
        .and_then(|r| Ok(r.replace_all(&out, "$1")))
        .unwrap_or(out.clone());
    let out = Regex::new(r"(?m)^\s*[-*+]\s+")
        .and_then(|r| Ok(r.replace_all(&out, "")))
        .unwrap_or(out.clone());
    let out = Regex::new(r"\*([^*\n]+)\*")
        .and_then(|r| Ok(r.replace_all(&out, "$1")))
        .unwrap_or(out.clone());
    let out = Regex::new(r"_([^_\n]+)_")
        .and_then(|r| Ok(r.replace_all(&out, "$1")))
        .unwrap_or(out.clone());
    // Collapse runs of blank lines so pauses between paragraphs stay natural.
    let out = Regex::new(r"\n{3,}")
        .and_then(|r| Ok(r.replace_all(&out, "\n\n")))
        .unwrap_or(out.clone());
    out.trim().to_string()
}

#[cfg(not(windows))]
fn strip_markdown_for_speech(text: &str) -> String {
    text.trim().to_string()
}

/// Escape the characters SAPI's XML parser treats as markup so plain LLM
/// prose (code snippets, comparisons like `a < b`, ampersands) is spoken
/// literally instead of being parsed or rejected as SSML.
#[cfg(windows)]
fn escape_for_sapi_xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
