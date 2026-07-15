import { describe, expect, it } from "vitest";
import {
  closedDialogNavigation,
  enterDialogSurface,
  leaveDialogSurface,
  openDialogNavigation,
} from "./dialogStack";

type Surface = "settings" | "security" | "privacy";

describe("dialog navigation stack", () => {
  it("returns to the immediate parent", () => {
    const settings = openDialogNavigation<Surface>("settings");
    const security = enterDialogSurface(settings, "security");
    const privacy = enterDialogSurface(security, "privacy");

    expect(leaveDialogSurface(privacy)).toEqual(security);
    expect(leaveDialogSurface(security)).toEqual(settings);
  });

  it("closes a root dialog and discards stale history", () => {
    const root = openDialogNavigation<Surface>("settings");
    expect(leaveDialogSurface(root)).toEqual(closedDialogNavigation<Surface>());
  });

  it("opens an independent root without inheriting an earlier path", () => {
    const settings = enterDialogSurface(
      openDialogNavigation<Surface>("settings"),
      "security",
    );
    expect(openDialogNavigation<Surface>("privacy")).toEqual({
      current: "privacy",
      history: [],
    });
    expect(settings.history).toEqual(["settings"]);
  });
});
