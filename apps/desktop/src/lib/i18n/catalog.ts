import sourceManifest from "$locales/en-AU.json";
import type { LanguagePackManifest } from "./types";

export type TranslationKey = keyof typeof sourceManifest.messages;

export const sourceLanguagePack = sourceManifest as LanguagePackManifest;
