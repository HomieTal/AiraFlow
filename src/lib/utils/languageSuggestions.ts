import { languageIdentity } from "@handy-computer/models";
import { LANGUAGES } from "@/lib/constants/languages";

declare global {
  interface Window {
    /** Injected before scripts run (`system_languages.rs`); absent in a browser. */
    __AIRA_FLOW_SYSTEM_LANGUAGES__?: {
      languages: string[];
    };
  }
}

const system = () => window.__AIRA_FLOW_SYSTEM_LANGUAGES__;
const locale = () => navigator.language || "en-US";

/** Likely spoken languages (always with English), falling back to the browser locale. */
export const suggestedLanguages = (): string[] => {
  const guessed = system()?.languages;
  if (guessed?.length) return guessed;
  const code = languageIdentity(locale().split("_")[0]);
  return code !== "en" && LANGUAGES.some((l) => l.code === code)
    ? [code, "en"]
    : ["en"];
};
