import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { useSettings } from "../../../hooks/useSettings";

interface SpeakAnswersProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/** Speak successful LLM post-processing answers aloud (Windows TTS). */
export const SpeakAnswers: React.FC<SpeakAnswersProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const enabled = getSetting("speak_answers_enabled") || false;

    return (
      <div className="flex flex-col">
        <ToggleSwitch
          checked={enabled}
          onChange={(value) => updateSetting("speak_answers_enabled", value)}
          isUpdating={isUpdating("speak_answers_enabled")}
          label={t("settings.postProcessing.speakAnswers.label")}
          description={t("settings.postProcessing.speakAnswers.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />
      </div>
    );
  },
);
