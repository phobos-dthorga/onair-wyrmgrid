export type ThemeFileDownloadEnvironment = {
  createObjectUrl: (content: string) => string;
  clickDownload: (filename: string, objectUrl: string) => void;
  revokeObjectUrl: (objectUrl: string) => void;
  defer: (action: () => void) => void;
};

export function downloadThemeFile(
  filename: string,
  content: string,
  environment: ThemeFileDownloadEnvironment = browserDownloadEnvironment(),
): void {
  const objectUrl = environment.createObjectUrl(content);
  environment.clickDownload(filename, objectUrl);
  environment.defer(() => environment.revokeObjectUrl(objectUrl));
}

function browserDownloadEnvironment(): ThemeFileDownloadEnvironment {
  return {
    createObjectUrl: (content) =>
      URL.createObjectURL(new Blob([content], { type: "application/json" })),
    clickDownload: (filename, objectUrl) => {
      const link = document.createElement("a");
      link.href = objectUrl;
      link.download = filename;
      link.hidden = true;
      document.body.append(link);
      link.click();
      link.remove();
    },
    revokeObjectUrl: (objectUrl) => URL.revokeObjectURL(objectUrl),
    defer: (action) => void globalThis.setTimeout(action, 0),
  };
}
