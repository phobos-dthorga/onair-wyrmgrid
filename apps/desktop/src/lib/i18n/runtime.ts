import { FluentBundle, FluentResource } from "@fluent/bundle";
import { writable } from "svelte/store";
import { sourceLanguagePack } from "./catalog";
import type { TranslationKey } from "./catalog";
import type { LanguagePackManifest } from "./types";

export type TranslationArguments = Record<string, string | number | Date>;
export type Translator = (
  messageId: TranslationKey,
  arguments_?: TranslationArguments,
  fallback?: string,
) => string;

const sourceBundle = buildBundle(sourceLanguagePack);
let activeBundle = sourceBundle;

export const translation = writable<Translator>(translate);
export const activeLocale = writable(sourceLanguagePack.locale);

export function applyLanguage(manifest: LanguagePackManifest): void {
  activeBundle =
    manifest.id === sourceLanguagePack.id
      ? sourceBundle
      : buildBundle(manifest);
  activeLocale.set(manifest.locale);
  translation.set(translate);

  if (typeof document !== "undefined") {
    document.documentElement.lang = manifest.locale;
    document.documentElement.dir =
      manifest.direction === "right_to_left" ? "rtl" : "ltr";
    document.documentElement.dataset.languagePack = manifest.id;
  }
}

export function translate(
  messageId: string,
  arguments_: TranslationArguments = {},
  fallback = messageId,
): string {
  return (
    formatMessage(activeBundle, messageId, arguments_) ??
    formatMessage(sourceBundle, messageId, arguments_) ??
    fallback
  );
}

function buildBundle(manifest: LanguagePackManifest): FluentBundle {
  const bundle = new FluentBundle(manifest.locale, { useIsolating: true });
  for (const [messageId, pattern] of Object.entries(manifest.messages)) {
    const resource = new FluentResource(messageResource(messageId, pattern));
    bundle.addResource(resource);
  }
  return bundle;
}

function messageResource(messageId: string, pattern: string): string {
  const [first = "", ...remaining] = pattern.split("\n");
  return `${messageId} = ${first}${remaining.map((line) => `\n    ${line}`).join("")}`;
}

function formatMessage(
  bundle: FluentBundle,
  messageId: string,
  arguments_: TranslationArguments,
): string | null {
  const message = bundle.getMessage(messageId);
  if (!message?.value) return null;
  const errors: Error[] = [];
  const formatted = bundle.formatPattern(message.value, arguments_, errors);
  return errors.length === 0 ? formatted : null;
}
