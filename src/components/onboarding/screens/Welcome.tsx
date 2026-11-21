import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../ui/Button";
import { Mascot } from "../../shared/Mascot";
import {
  LanguageOptions,
  LanguageSearchField,
  useLanguagePicker,
} from "../../shared/LanguagePicker";
import AiraFlowTextLogo from "../../icons/AiraFlowTextLogo";
import { OnboardingShell, ScreenHeader } from "../OnboardingShell";
import { AppLanguageMenu } from "../AppLanguageMenu";
import { useOnboardingStore } from "../onboardingStore";
import { CURATED_ASR, isWeakLanguage } from "@/lib/models/catalog";
import { modelMatchesLanguage } from "@handy-computer/models";
import { getSupportedLanguage } from "@/i18n";
import { languageList } from "@/lib/constants/languages";
import { useSettings } from "@/hooks/useSettings";

// The mascot waves on the first visit only.
let waved = false;

interface WelcomeProps {
  progress: { index: number; count: number };
  onContinue: () => void;
}

/** Screen 1: greeting and spoken languages. */
export const Welcome: React.FC<WelcomeProps> = ({ progress, onContinue }) => {
  const { t, i18n } = useTranslation();
  const { updateSetting } = useSettings();
  const { languages, modelId, preview, set } = useOnboardingStore();
  const picker = useLanguagePicker({
    languages,
    onChange: (next) => {
      // A model picked for other languages no longer counts as the pick.
      const picked = CURATED_ASR.find((m) => m.id === modelId);
      const stillFits =
        !picked || next.every((l) => modelMatchesLanguage(picked, l));
      set({
        languages: next,
        ...(stillFits ? {} : { modelId: null }),
      });
    },
  });
  const [wave] = useState(() => !waved);
  useEffect(() => {
    waved = true;
    const id = setTimeout(
      () => picker.inputRef.current?.focus({ preventScroll: true }),
      50,
    );
    return () => clearTimeout(id);
  }, []);

  const ui = i18n.language;
  const weak = languages.filter(isWeakLanguage);

  const hint = !languages.length
    ? t("onboarding.welcome.hint.pickOne")
    : weak.length
      ? t("onboarding.welcome.hint.weak", {
          count: weak.length,
          languages: languageList(weak, ui),
        })
      : "";

  // What the menus are in right now (a preview never writes the setting).
  const uiLanguage = getSupportedLanguage(ui) || "en";
  const changeUiLanguage = (code: string) => {
    i18n.changeLanguage(code);
    if (!preview) updateSetting("app_language", code);
  };

  return (
    <OnboardingShell
      progress={progress}
      pill={<AppLanguageMenu value={uiLanguage} onChange={changeUiLanguage} />}
      hint={hint}
      hintTone={weak.length ? "warning" : "quiet"}
      primary={
        <Button
          variant="primary"
          size="lg"
          disabled={!languages.length}
          onClick={onContinue}
        >
          {t("onboarding.shell.continue")}
        </Button>
      }
    >
      <div className="flex h-full min-h-0 flex-col items-center px-gutter pt-[6px] pb-[4px] text-center">
        <div className="mb-[10px] inline-flex items-end gap-[8px] leading-none">
          <Mascot size={54} animation={wave ? "wave" : null} />
          <AiraFlowTextLogo width={112} className="mb-[4px] block" />
        </div>
        <ScreenHeader
          title={t("onboarding.welcome.title")}
          sub={t("onboarding.welcome.subtitle")}
        />

        <div className="mt-lead flex min-h-0 w-full flex-1 flex-col overflow-hidden rounded-panel border-[1.5px] border-line-strong bg-background">
          <LanguageSearchField
            picker={picker}
            className="border-b border-line"
            inputClassName="min-w-[120px] text-body"
          />
          <div
            role="group"
            aria-label={t("onboarding.welcome.languagesLabel")}
            className="grid min-h-0 flex-1 grid-cols-3 content-start gap-[2px] overflow-y-auto p-[6px] pb-[14px] text-start [mask-image:linear-gradient(to_bottom,black_calc(100%-40px),transparent_calc(100%-6px))]"
          >
            <LanguageOptions
              picker={picker}
              noMatch={t("onboarding.welcome.noMatch", {
                query: picker.query,
              })}
              emptyClassName="p-[24px]"
            />
          </div>
        </div>
      </div>
    </OnboardingShell>
  );
};
