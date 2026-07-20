import { describe, expect, it } from "vitest";
import { downloadThemeFile, type ThemeFileDownloadEnvironment } from "./file";

describe("theme file downloads", () => {
  it("clicks an attached browser download before revoking its object URL", () => {
    const events: string[] = [];
    let deferred: (() => void) | undefined;
    const environment: ThemeFileDownloadEnvironment = {
      createObjectUrl: (content) => {
        events.push(`create:${content}`);
        return "blob:theme";
      },
      clickDownload: (filename, objectUrl) => {
        events.push(`click:${filename}:${objectUrl}`);
      },
      revokeObjectUrl: (objectUrl) => {
        events.push(`revoke:${objectUrl}`);
      },
      defer: (action) => {
        events.push("defer");
        deferred = action;
      },
    };

    downloadThemeFile("my-theme.json", "{}\n", environment);

    expect(events).toEqual([
      "create:{}\n",
      "click:my-theme.json:blob:theme",
      "defer",
    ]);
    deferred?.();
    expect(events.at(-1)).toBe("revoke:blob:theme");
  });
});
