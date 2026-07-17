import { describe, expect, it } from "vitest";
import {
  ADMINISTRATIVE_REGION_LABEL_BANDS,
  administrativeRegionContext,
  administrativeRegionFromMapFeature,
  administrativeRegionLabelBandForZoom,
} from "./regions";

describe("Atlas administrative regions", () => {
  it("translates a sourced map feature into an inspector view", () => {
    const region = administrativeRegionFromMapFeature({
      id: "natural-earth:1159315805",
      properties: {
        region_id: "natural-earth:1159315805",
        level: "ADM1",
        name: "Western Australia",
        name_local: null,
        local_type: "State",
        country_name: "Australia",
        country_code: "AUS",
        subdivision_code: "AU-WA",
        source: "Natural Earth",
        source_version: "5.1.2",
      },
    });

    expect(region).toEqual({
      id: "natural-earth:1159315805",
      feature_id: "natural-earth:1159315805",
      level: "ADM1",
      name: "Western Australia",
      name_local: undefined,
      local_type: "State",
      country_name: "Australia",
      country_code: "AUS",
      subdivision_code: "AU-WA",
      source: "Natural Earth",
      source_version: "5.1.2",
    });
    expect(administrativeRegionContext(region!)).toBe("State · Australia");
  });

  it("rejects incomplete features instead of inventing regional facts", () => {
    expect(
      administrativeRegionFromMapFeature({
        id: "unknown",
        properties: {
          region_id: "unknown",
          level: "ADM1",
          source: "Natural Earth",
          source_version: "5.1.2",
        },
      }),
    ).toBeUndefined();
  });

  it("supports the future ADM2 tier without calling every region a county", () => {
    const region = administrativeRegionFromMapFeature({
      id: "example:district",
      properties: {
        region_id: "example:district",
        level: "ADM2",
        name: "Example district",
        local_type: "District",
        source: "Example source",
        source_version: "1",
      },
    });

    expect(region?.level).toBe("ADM2");
    expect(region?.local_type).toBe("District");
  });

  it("progressively reveals sourced labels without a zoom gap", () => {
    expect(administrativeRegionLabelBandForZoom(2.49)).toBeUndefined();
    expect(administrativeRegionLabelBandForZoom(2.5)?.id).toBe("overview");
    expect(administrativeRegionLabelBandForZoom(3.25)?.id).toBe("continental");
    expect(administrativeRegionLabelBandForZoom(5.5)?.id).toBe("regional");
    expect(administrativeRegionLabelBandForZoom(9)?.id).toBe("detailed");
    expect(administrativeRegionLabelBandForZoom(18)?.id).toBe("survey");
    expect(administrativeRegionLabelBandForZoom(24)).toBeUndefined();

    for (
      let index = 1;
      index < ADMINISTRATIVE_REGION_LABEL_BANDS.length;
      index++
    ) {
      const previous = ADMINISTRATIVE_REGION_LABEL_BANDS[index - 1];
      const current = ADMINISTRATIVE_REGION_LABEL_BANDS[index];
      expect(current.min_zoom).toBe(previous.max_zoom);
      expect(current.maximum_source_min_zoom).toBeGreaterThanOrEqual(
        previous.maximum_source_min_zoom,
      );
      expect(current.text_padding).toBeLessThanOrEqual(previous.text_padding);
    }
  });
});
