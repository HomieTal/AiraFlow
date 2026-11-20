import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Mic, Loader2 } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { commands, type TtsEngine, type TtsVoiceInfo } from "@/bindings";
import { Dropdown } from "../ui/Dropdown";
import { Input } from "../ui/Input";
import { Slider } from "../ui/Slider";
import { Button } from "../ui/Button";
import { SettingsGroup } from "../ui/SettingsGroup";
import { useSettings } from "../../hooks/useSettings";
import { SpeakAnswers } from "./post-processing/SpeakAnswers";

const OPENAI_VOICES = ["alloy", "echo", "fable", "onyx", "nova", "shimmer"];
const OPENAI_MODELS = ["gpt-4o-mini-tts", "gpt-4o-tts", "tts-1", "tts-1-hd"];
const ELEVENLABS_MODELS = [
  "eleven_multilingual_v2",
  "eleven_turbo_v2_5",
  "eleven_flash_v2_5",
];

/** Voice answers: engine picker, per-engine voice settings, and a test. */
export const VoiceSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const engine = (getSetting("tts_engine") as TtsEngine) || "windows";

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.voice.speakAnswersGroup.title")}>
        <SpeakAnswers grouped={true} />
      </SettingsGroup>

      <SettingsGroup title={t("settings.voice.engine.title")}>
        <Dropdown
          options={[
            {
              value: "windows",
              label: t("settings.voice.engine.windows"),
            },
            {
              value: "open_ai",
              label: t("settings.voice.engine.openai"),
            },
            {
              value: "eleven_labs",
              label: t("settings.voice.engine.elevenlabs"),
            },
          ]}
          selectedValue={engine}
          onSelect={(value) => updateSetting("tts_engine", value as TtsEngine)}
        />
      </SettingsGroup>

      {engine === "windows" && <WindowsVoiceSettings />}
      {engine === "open_ai" && <OpenAiVoiceSettings />}
      {engine === "eleven_labs" && <ElevenLabsVoiceSettings />}

      <SettingsGroup title={t("settings.voice.test.title")}>
        <TestVoiceButton />
      </SettingsGroup>
    </div>
  );
};

/** Windows SAPI: voice picker, rate, volume, and the pitch-matching flow. */
const WindowsVoiceSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [voices, setVoices] = useState<TtsVoiceInfo[] | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [analysisNote, setAnalysisNote] = useState<string | null>(null);

  const refreshVoices = useCallback(async () => {
    setVoices(null);
    try {
      setVoices(await commands.listTtsSapiVoices());
    } catch (error) {
      console.error("Failed to list SAPI voices:", error);
      setVoices([]);
    }
  }, []);

  useEffect(() => {
    void refreshVoices();
  }, [refreshVoices]);

  const selectedVoiceId = (getSetting("tts_sapi_voice_id") as string) || "";

  const matchMyVoice = async () => {
    setAnalysisNote(null);
    const picked = await open({
      multiple: false,
      filters: [
        { name: t("settings.voice.windows.sampleFilter"), extensions: ["wav", "mp3", "m4a", "flac", "ogg", "opus"] },
      ],
    });
    if (!picked) return;
    setAnalyzing(true);
    try {
      const result = await commands.analyzeVoiceSampleSetting(picked);
      if (result.status === "error") throw new Error(result.error);
      const analysis = result.data;
      if (!analysis.detected_f0 || !analysis.suggested_voice_id) {
        setAnalysisNote(t("settings.voice.windows.matchNoVoice"));
        return;
      }
      await updateSetting("tts_sapi_voice_id", analysis.suggested_voice_id);
      await updateSetting("tts_sapi_pitch", analysis.suggested_pitch);
      setAnalysisNote(
        t("settings.voice.windows.matchApplied", {
          f0: Math.round(analysis.detected_f0),
        }),
      );
    } catch (error) {
      console.error("Voice sample analysis failed:", error);
      setAnalysisNote(t("settings.voice.windows.matchFailed"));
    } finally {
      setAnalyzing(false);
    }
  };

  const voiceOptions =
    voices?.map((v) => ({ value: v.id, label: v.name })) ?? [];
  const voiceName = voices?.find((v) => v.id === selectedVoiceId)?.name;

  return (
    <SettingsGroup title={t("settings.voice.windows.title")}>
      <Dropdown
        options={voiceOptions}
        selectedValue={selectedVoiceId || null}
        onSelect={(value) => updateSetting("tts_sapi_voice_id", value)}
        placeholder={voiceName || t("settings.voice.windows.voicePlaceholder")}
        onOpen={() => {
          if (!voices) void refreshVoices();
        }}
      />
      <Slider
        value={(getSetting("tts_sapi_rate") as number) || 0}
        onChange={(value) => updateSetting("tts_sapi_rate", value)}
        min={-10}
        max={10}
        step={1}
        label={t("settings.voice.windows.rate.label")}
        description={t("settings.voice.windows.rate.description")}
        grouped={true}
        formatValue={(v) => String(v)}
      />
      <Slider
        value={(getSetting("tts_sapi_volume") as number) || 100}
        onChange={(value) => updateSetting("tts_sapi_volume", value)}
        min={0}
        max={100}
        step={5}
        label={t("settings.voice.windows.volume.label")}
        description={t("settings.voice.windows.volume.description")}
        grouped={true}
        formatValue={(v) => `${v}%`}
      />
      <SettingRow
        label={t("settings.voice.windows.match.label")}
        description={t("settings.voice.windows.match.description")}
      >
        <Button variant="secondary" size="md" onClick={matchMyVoice} disabled={analyzing}>
          {analyzing ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <Mic className="w-4 h-4" />
          )}
          {t("settings.voice.windows.match.button")}
        </Button>
      </SettingRow>
      {analysisNote && <p className="text-sm text-ink-2 px-1">{analysisNote}</p>}
    </SettingsGroup>
  );
};

/** OpenAI TTS: key, voice, model, speed. */
const OpenAiVoiceSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [apiKey, setApiKey] = useState(
    t("settings.voice.openai.apiKeyPlaceholder"),
  );

  React.useEffect(() => {
    void commands.getAppSettings().then((s) => {
      const key =
        (s as { tts_openai_api_key?: Record<string, string> }).tts_openai_api_key?.[
          "openai"
        ] ?? "";
      setApiKey(key);
    });
  }, []);

  return (
    <SettingsGroup title={t("settings.voice.openai.title")}>
      <SettingRow
        label={t("settings.voice.openai.apiKey.label")}
        description={t("settings.voice.openai.apiKey.description")}
      >
        <Input
          type="password"
          value={apiKey}
          onChange={(event) => setApiKey(event.target.value)}
          onBlur={() => void commands.setTtsOpenaiApiKeySetting(apiKey)}
          variant="compact"
          className="flex-1 min-w-[320px]"
        />
      </SettingRow>
      <Dropdown
        options={OPENAI_VOICES.map((v) => ({ value: v, label: v }))}
        selectedValue={(getSetting("tts_openai_voice") as string) || "alloy"}
        onSelect={(value) => updateSetting("tts_openai_voice", value)}
      />
      <Dropdown
        options={OPENAI_MODELS.map((m) => ({ value: m, label: m }))}
        selectedValue={
          (getSetting("tts_openai_model") as string) || "gpt-4o-mini-tts"
        }
        onSelect={(value) => updateSetting("tts_openai_model", value)}
      />
      <Slider
        value={(getSetting("tts_openai_speed") as number) || 1.0}
        onChange={(value) => updateSetting("tts_openai_speed", value)}
        min={0.25}
        max={4}
        step={0.05}
        label={t("settings.voice.openai.speed.label")}
        description={t("settings.voice.openai.speed.description")}
        grouped={true}
        formatValue={(v) => `${v.toFixed(2)}x`}
      />
    </SettingsGroup>
  );
};

/** ElevenLabs: key, voice id (stock or a cloned voice), model. */
const ElevenLabsVoiceSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [apiKey, setApiKey] = useState("");

  React.useEffect(() => {
    void commands.getAppSettings().then((s) => {
      const key =
        (s as { tts_elevenlabs_api_key?: Record<string, string> })
          .tts_elevenlabs_api_key?.["elevenlabs"] ?? "";
      setApiKey(key);
    });
  }, []);

  return (
    <SettingsGroup title={t("settings.voice.elevenlabs.title")}>
      <SettingRow
        label={t("settings.voice.elevenlabs.apiKey.label")}
        description={t("settings.voice.elevenlabs.apiKey.description")}
      >
        <Input
          type="password"
          value={apiKey}
          onChange={(event) => setApiKey(event.target.value)}
          onBlur={() => void commands.setTtsElevenlabsApiKeySetting(apiKey)}
          variant="compact"
          className="flex-1 min-w-[320px]"
        />
      </SettingRow>
      <SettingRow
        label={t("settings.voice.elevenlabs.voiceId.label")}
        description={t("settings.voice.elevenlabs.voiceId.description")}
      >
        <Input
          value={(getSetting("tts_elevenlabs_voice_id") as string) || ""}
          onChange={(event) =>
            updateSetting("tts_elevenlabs_voice_id", event.target.value)
          }
          variant="compact"
          className="flex-1 min-w-[320px]"
        />
      </SettingRow>
      <Dropdown
        options={ELEVENLABS_MODELS.map((m) => ({ value: m, label: m }))}
        selectedValue={
          (getSetting("tts_elevenlabs_model") as string) ||
          "eleven_multilingual_v2"
        }
        onSelect={(value) => updateSetting("tts_elevenlabs_model", value)}
      />
    </SettingsGroup>
  );
};

/** Speak the sample sentence with the current voice settings. */
const TestVoiceButton: React.FC = () => {
  const { t } = useTranslation();
  const [testing, setTesting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const runTest = async () => {
    setTesting(true);
    setError(null);
    try {
      await commands.testTtsVoice();
    } catch (err) {
      setError(String(err));
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-2">
        <Button variant="secondary" size="md" onClick={runTest} disabled={testing}>
          {testing && <Loader2 className="w-4 h-4 animate-spin" />}
          {t("settings.voice.test.button")}
        </Button>
        {testing && (
          <span className="text-sm text-ink-2">
            {t("settings.voice.test.running")}
          </span>
        )}
      </div>
      {error && (
        <p className="text-sm text-error px-1 break-all">{error}</p>
      )}
    </div>
  );
};

/** Minimal labelled row (label/description on the left, control on the right). */
const SettingRow: React.FC<{
  label: string;
  description?: string;
  children: React.ReactNode;
}> = ({ label, description, children }) => (
  <div className="flex items-center justify-between gap-4 py-3">
    <div className="flex flex-col">
      <span className="text-sm font-medium">{label}</span>
      {description && (
        <span className="text-xs text-ink-2">{description}</span>
      )}
    </div>
    <div className="flex items-center">{children}</div>
  </div>
);
