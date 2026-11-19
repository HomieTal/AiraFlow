//! Windows SAPI text-to-speech for spoken answers.
//!
//! When "Speak answers aloud" is enabled and LLM post-processing produced an
//! answer, [`speak_text`] renders it through the default SAPI voice on a
//! background thread, so the dictation pipeline never blocks on audio.
//!
//! Best-effort by design: any failure is logged and swallowed — a missing or
//! misconfigured voice must never break transcription or paste.

#[cfg(windows)]
use log::{debug, info, warn};
#[cfg(windows)]
use regex::Regex;

/// Speak `text` aloud asynchronously on a background thread.
///
/// Fire-and-forget: the caller continues immediately; overlapping utterances
/// are possible if the user dictates again mid-speech (each call owns its own
/// SAPI voice instance on its own STA thread).
#[cfg(windows)]
pub fn speak_text(text: &str) {
    let text = strip_markdown_for_speech(text);
    if text.trim().is_empty() {
        return;
    }
    std::thread::Builder::new()
        .name("tts".into())
        .spawn(move || speak_blocking(&text))
        .map_err(|e| warn!("Failed to spawn TTS thread: {e}"))
        .ok();
}

/// Non-Windows stub: the speak-answers setting only has an effect on Windows.
#[cfg(not(windows))]
pub fn speak_text(_text: &str) {}

#[cfg(windows)]
fn speak_blocking(text: &str) {
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
        // SPF_DEFAULT (0). Note: on some SAPI configurations the
        // SPF_IS_NOT_XML (4) path fails outright with INET_E_UNKNOWN_PROTOCOL
        // (0x800C000D) even for plain ASCII text, so we keep XML parsing on
        // and escape the XML-sensitive characters ourselves instead.
        let safe = escape_for_sapi_xml(text);
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

    if let Err(err) = spoken {
        warn!("Failed to speak answer aloud: {err}");
    } else {
        info!("Spoken answer finished ({} chars)", text.chars().count());
    }

    if owns_com {
        unsafe { CoUninitialize() };
    }
}

/// Escape the characters SAPI's XML parser treats as markup so plain LLM
/// prose (code snippets, comparisons like `a < b`, ampersands) is spoken
/// literally instead of being parsed or rejected as SSML.
#[cfg(windows)]
fn escape_for_sapi_xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
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
