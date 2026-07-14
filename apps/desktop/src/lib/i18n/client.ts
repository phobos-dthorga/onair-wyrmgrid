import { invokeDesktop, isDesktopRuntime } from "$lib/desktop/client";
import { sourceLanguagePack } from "./catalog";
import type { LanguageStatus } from "./types";

export { sourceLanguagePack } from "./catalog";

const protectedPrefixes = [
  "legal-",
  "privacy-",
  "credential-",
  "telemetry-",
  "plugin-permission-",
  "security-",
  "destructive-",
  "error-",
];

const eligibleMessages = Object.keys(sourceLanguagePack.messages).filter(
  (key) => !protectedPrefixes.some((prefix) => key.startsWith(prefix)),
).length;

export const browserLanguageStatus: LanguageStatus = {
  selected_language_pack_id: sourceLanguagePack.id,
  active_pack: sourceLanguagePack,
  packs: [
    {
      manifest: sourceLanguagePack,
      trust: "built_in",
      translated_messages: eligibleMessages,
      eligible_messages: eligibleMessages,
    },
  ],
};

export async function loadLanguageStatus(): Promise<LanguageStatus> {
  return isDesktopRuntime()
    ? invokeDesktop<LanguageStatus>("language_status")
    : browserLanguageStatus;
}

export async function selectLanguagePack(
  packId: string,
): Promise<LanguageStatus> {
  return invokeDesktop<LanguageStatus>("select_language_pack", { packId });
}

export async function importLanguagePack(
  manifestJson: string,
): Promise<LanguageStatus> {
  return invokeDesktop<LanguageStatus>("import_language_pack", {
    manifestJson,
  });
}
