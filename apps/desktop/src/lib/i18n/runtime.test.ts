import { get } from "svelte/store";
import { afterEach, describe, expect, it } from "vitest";
import { sourceLanguagePack } from "./client";
import { applyLanguage, translation } from "./runtime";
import type { LanguagePackManifest } from "./types";

afterEach(() => applyLanguage(sourceLanguagePack));

describe("localization runtime", () => {
  it("layers a partial community pack over the canonical English catalogue", () => {
    const communityPack: LanguagePackManifest = {
      ...sourceLanguagePack,
      id: "community-fr-test",
      locale: "fr",
      name: "Français test",
      author: "Fixture",
      messages: {
        "nav-fleet": "Flotte",
        "language-coverage":
          "{ $translated } messages communautaires sur { $total }",
      },
    };

    applyLanguage(communityPack);
    const t = get(translation);
    expect(t("nav-fleet")).toBe("Flotte");
    expect(t("nav-atlas")).toBe("Atlas");
    expect(
      t("language-coverage", { translated: 2, total: 40 }).replaceAll(
        /[\u2068\u2069]/g,
        "",
      ),
    ).toContain("2 messages communautaires sur 40");
  });

  it("returns an explicit caller fallback for an unknown message", () => {
    expect(get(translation)("missing-key", {}, "Readable fallback")).toBe(
      "Readable fallback",
    );
  });
});
