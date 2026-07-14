export type TextDirection = "left_to_right" | "right_to_left";

export type LanguagePackManifest = {
  schema_version: number;
  id: string;
  locale: string;
  name: string;
  author?: string;
  source_locale: string;
  source_catalog_version: number;
  direction: TextDirection;
  messages: Record<string, string>;
};

export type LanguagePackTrust = "built_in" | "reviewed" | "community";

export type AvailableLanguagePack = {
  manifest: LanguagePackManifest;
  trust: LanguagePackTrust;
  translated_messages: number;
  eligible_messages: number;
};

export type LanguageStatus = {
  selected_language_pack_id: string;
  active_pack: LanguagePackManifest;
  packs: AvailableLanguagePack[];
};
