import { describe, expect, it } from "vitest";
import type { StaffMemberSummary } from "$lib/atlas/types";
import {
  activeStaffFilterCount,
  defaultStaffFilters,
  filterAndSortStaff,
  matchesStaffSearch,
  providerCodeLabel,
  staffFilterOptions,
  type StaffFilters,
} from "$lib/staff/presentation";

const member: StaffMemberSummary = {
  id: "11111111-1111-4111-8111-111111111111",
  display_name: "Example Aviator",
  category_code: 2,
  status_code: 7,
  is_online: true,
  busy_until: "2026-07-17T12:00:00Z",
  current_airport: {
    id: "22222222-2222-4222-8222-222222222222",
    icao: "YTEST",
    name: "Synthetic Test Airport",
    location: null,
  },
  class_qualifications: [
    {
      id: "33333333-3333-4333-8333-333333333333",
      aircraft_class_id: "44444444-4444-4444-8444-444444444444",
      short_name: "TP",
      name: "Synthetic Turboprop",
    },
  ],
};

const secondMember: StaffMemberSummary = {
  id: "55555555-5555-4555-8555-555555555555",
  display_name: "Another Crew Member",
  category_code: 1,
  status_code: 0,
  is_online: false,
  class_qualifications: [],
};

describe("staff presentation", () => {
  it("keeps unknown provider enums as explicit codes", () => {
    expect(providerCodeLabel("category", 2)).toBe("OnAir category code 2");
    expect(providerCodeLabel("status", undefined)).toBe("Not reported");
  });

  it("searches only facts present in the bounded roster", () => {
    expect(matchesStaffSearch(member, "ytest")).toBe(true);
    expect(matchesStaffSearch(member, "turboprop")).toBe(true);
    expect(matchesStaffSearch(member, "invented qualification")).toBe(false);
  });

  it("derives filter choices only from reported roster facts", () => {
    expect(staffFilterOptions([member, secondMember])).toEqual({
      categoryCodes: [1, 2],
      statusCodes: [0, 7],
      qualifications: [
        {
          id: "44444444-4444-4444-8444-444444444444",
          label: "TP",
        },
      ],
    });
  });

  it("combines explicit filters without inferring availability", () => {
    const filters: StaffFilters = {
      ...defaultStaffFilters,
      categoryCode: 2,
      presence: "online",
      busy: "reported",
      qualificationId: "44444444-4444-4444-8444-444444444444",
    };
    expect(filterAndSortStaff([secondMember, member], filters)).toEqual([
      member,
    ]);
    expect(activeStaffFilterCount(filters)).toBe(4);
  });

  it("sorts unavailable airport facts after reported airports", () => {
    const filters: StaffFilters = {
      ...defaultStaffFilters,
      sort: "current_airport",
    };
    expect(
      filterAndSortStaff([secondMember, member], filters).map(
        (candidate) => candidate.id,
      ),
    ).toEqual([member.id, secondMember.id]);
    expect(activeStaffFilterCount(filters)).toBe(1);
  });
});
