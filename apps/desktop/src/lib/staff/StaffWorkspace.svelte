<script lang="ts">
  import "./staff.css";
  import type {
    AirportSummary,
    StaffMemberSummary,
    StaffSnapshotView,
  } from "$lib/atlas/types";
  import { responsiveSurface } from "$lib/accessibility/responsiveSurface";
  import ExplorationSummary from "$lib/exploration/ExplorationSummary.svelte";
  import ExplorationTabs from "$lib/exploration/ExplorationTabs.svelte";
  import { selectedOrFirst } from "$lib/exploration/collection";
  import {
    formatLocalDateTime,
    mediumDateShortTime,
  } from "$lib/presentation/dateTime";
  import {
    activeStaffFilterCount,
    defaultStaffFilters,
    filterAndSortStaff,
    providerCodeLabel,
    staffFilterOptions,
    type StaffFilters,
  } from "$lib/staff/presentation";

  let {
    view,
    busy = false,
    errorMessage = "",
    responsiveSurfaces = true,
    onsynchronize,
  }: {
    view: StaffSnapshotView | null;
    busy?: boolean;
    errorMessage?: string;
    responsiveSurfaces?: boolean;
    onsynchronize: () => void;
  } = $props();

  let selectedMemberId = $state<string | null>(null);
  let dossierSection = $state("overview");
  let filters = $state<StaffFilters>({ ...defaultStaffFilters });
  const members = $derived(view?.snapshot.value.staff ?? []);
  const options = $derived(staffFilterOptions(members));
  const filteredMembers = $derived(filterAndSortStaff(members, filters));
  const activeFilterCount = $derived(activeStaffFilterCount(filters));
  const selectedMember = $derived(
    selectedOrFirst(filteredMembers, selectedMemberId, (member) => member.id),
  );
  const dossierTabs = [
    { id: "overview", label: "Overview" },
    { id: "qualifications", label: "Qualifications" },
    { id: "evidence", label: "Source evidence" },
  ] as const;

  function memberName(member: StaffMemberSummary): string {
    return member.display_name ?? "Unnamed staff member";
  }

  function airportLabel(airport: AirportSummary | undefined): string {
    if (!airport) return "Not reported";
    return airport.icao ?? airport.name ?? "Airport details unavailable";
  }

  function formatDate(value: string | undefined): string {
    return formatLocalDateTime(value, "Not reported", mediumDateShortTime);
  }

  function onlineLabel(value: boolean | undefined): string {
    return value === undefined ? "Not reported" : value ? "Online" : "Offline";
  }

  function observationLabel(value: string | undefined): string {
    const observed = formatLocalDateTime(
      value,
      "Observation time unavailable",
    );
    return observed === "Observation time unavailable"
      ? observed
      : `Observed ${observed}`;
  }

  function resetFilters(): void {
    filters = { ...defaultStaffFilters };
  }

  function selectNumberFilter(
    key: "categoryCode" | "statusCode",
    value: string,
  ): void {
    filters[key] = value === "" ? null : Number(value);
  }
</script>

<section class="staff-workspace" aria-label="Read-only company staff roster">
  <aside class="staff-roster-panel">
    <div class="staff-heading">
      <span class="eyebrow">WyrmGrid Staff</span>
      <h2>Company roster</h2>
      <p>
        Review bounded staff facts received from OnAir. WyrmGrid does not alter
        assignments or invent missing roles and qualifications.
      </p>
    </div>

    <button
      class="staff-sync"
      type="button"
      disabled={busy}
      onclick={onsynchronize}
    >
      {busy ? "Synchronizing…" : "Synchronize OnAir"}
    </button>

    <label class="staff-search">
      <span>Find staff</span>
      <input
        type="search"
        placeholder="Name, airport, or class qualification"
        bind:value={filters.query}
      />
    </label>

    <details class="staff-filter-panel">
      <summary>
        <span>Filter and sort</span>
        {#if activeFilterCount > 0}<strong>{activeFilterCount} active</strong>{/if}
      </summary>
      <div class="staff-filter-grid">
        <label>
          <span>Provider category</span>
          <select
            value={filters.categoryCode ?? ""}
            onchange={(event) =>
              selectNumberFilter("categoryCode", event.currentTarget.value)}
          >
            <option value="">All reported categories</option>
            {#each options.categoryCodes as code}
              <option value={code}>{providerCodeLabel("category", code)}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>Provider status</span>
          <select
            value={filters.statusCode ?? ""}
            onchange={(event) =>
              selectNumberFilter("statusCode", event.currentTarget.value)}
          >
            <option value="">All reported statuses</option>
            {#each options.statusCodes as code}
              <option value={code}>{providerCodeLabel("status", code)}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>Provider presence</span>
          <select bind:value={filters.presence}>
            <option value="all">Any presence value</option>
            <option value="online">Online</option>
            <option value="offline">Offline</option>
            <option value="unreported">Not reported</option>
          </select>
        </label>
        <label>
          <span>Busy-until field</span>
          <select bind:value={filters.busy}>
            <option value="all">Either state</option>
            <option value="reported">Reported by OnAir</option>
            <option value="unreported">Not reported</option>
          </select>
        </label>
        <label>
          <span>Aircraft class</span>
          <select
            value={filters.qualificationId ?? ""}
            onchange={(event) =>
              (filters.qualificationId = event.currentTarget.value || null)}
          >
            <option value="">Any reported class</option>
            {#each options.qualifications as qualification}
              <option value={qualification.id}>{qualification.label}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>Order roster by</span>
          <select bind:value={filters.sort}>
            <option value="name">Name</option>
            <option value="current_airport">Current airport</option>
            <option value="provider_status">Provider status</option>
            <option value="qualification_count">Qualification count</option>
          </select>
        </label>
      </div>
    </details>

    <ExplorationSummary
      shown={filteredMembers.length}
      total={members.length}
      label="staff"
      activeFilters={activeFilterCount}
      onclear={resetFilters}
    />

    {#if errorMessage}<p class="staff-error" role="alert">{errorMessage}</p>{/if}

    <div class="staff-roster" aria-label="Staff roster">
      {#each filteredMembers as member (member.id)}
        <button
          class="responsive-surface"
          class:active={selectedMember?.id === member.id}
          use:responsiveSurface={{ enabled: responsiveSurfaces }}
          type="button"
          onclick={() => (selectedMemberId = member.id)}
        >
          <span>{memberName(member)}</span>
          <strong>{airportLabel(member.current_airport)}</strong>
          <small>{providerCodeLabel("status", member.status_code)}</small>
        </button>
      {:else}
        <div class="staff-empty-list">
          <strong
            >{members.length ? "No roster matches" : "Awaiting roster"}</strong
          >
          <span>
            {members.length
              ? "Adjust or clear the current filters."
              : "Connect and synchronize OnAir to receive staff facts."}
          </span>
        </div>
      {/each}
    </div>
  </aside>

  <main class="staff-stage">
    {#if selectedMember}
      <article class="staff-dossier">
        <header>
          <div>
            <span class="eyebrow">Read-only staff fact</span>
            <h2>{memberName(selectedMember)}</h2>
            <p>{view?.company.name ?? "Company unavailable"}</p>
          </div>
          <span class="staff-source"
            >{view?.availability === "preview"
              ? "Synthetic preview"
              : "OnAir fact"}</span
          >
        </header>

        <div class="staff-dossier-tabs">
          <ExplorationTabs
            tabs={dossierTabs}
            bind:selected={dossierSection}
            label="Staff dossier sections"
            idPrefix="staff"
          />
        </div>

        {#if dossierSection === "overview"}
          <section
            id="staff-panel-overview"
            class="staff-metrics"
            role="tabpanel"
          >
            <article
              class="responsive-surface"
              use:responsiveSurface={{ enabled: responsiveSurfaces }}
            >
              <span>Provider category</span>
              <strong
                >{providerCodeLabel(
                  "category",
                  selectedMember.category_code,
                )}</strong
              >
              <small>Numeric provider code; WyrmGrid does not guess its label.</small>
            </article>
            <article
              class="responsive-surface"
              use:responsiveSurface={{ enabled: responsiveSurfaces }}
            >
              <span>Provider status</span>
              <strong
                >{providerCodeLabel("status", selectedMember.status_code)}</strong
              >
              <small>Numeric provider code; WyrmGrid does not guess its label.</small>
            </article>
            <article
              class="responsive-surface"
              use:responsiveSurface={{ enabled: responsiveSurfaces }}
            >
              <span>Current airport</span>
              <strong>{airportLabel(selectedMember.current_airport)}</strong>
              {#if selectedMember.current_airport?.name}
                <small>{selectedMember.current_airport.name}</small>
              {/if}
            </article>
            <article
              class="responsive-surface"
              use:responsiveSurface={{ enabled: responsiveSurfaces }}
            >
              <span>Home airport</span>
              <strong>{airportLabel(selectedMember.home_airport)}</strong>
              {#if selectedMember.home_airport?.name}
                <small>{selectedMember.home_airport.name}</small>
              {/if}
            </article>
            <article
              class="responsive-surface"
              use:responsiveSurface={{ enabled: responsiveSurfaces }}
            >
              <span>Busy until</span>
              <strong>{formatDate(selectedMember.busy_until)}</strong>
            </article>
            <article
              class="responsive-surface"
              use:responsiveSurface={{ enabled: responsiveSurfaces }}
            >
              <span>Provider presence</span>
              <strong>{onlineLabel(selectedMember.is_online)}</strong>
            </article>
          </section>
        {:else if dossierSection === "qualifications"}
          <section
            id="staff-panel-qualifications"
            class="staff-qualifications"
            role="tabpanel"
          >
            <div>
              <span class="eyebrow">Reported class qualifications</span>
              <h3>Aircraft classes</h3>
            </div>
            {#each selectedMember.class_qualifications as qualification (qualification.id)}
              <article
                class="responsive-surface"
                use:responsiveSurface={{ enabled: responsiveSurfaces }}
              >
                <strong
                  >{qualification.short_name ??
                    qualification.name ??
                    "Unnamed class"}</strong
                >
                <span>{qualification.name ?? "Full class name not reported"}</span>
                <small>
                  {qualification.last_validated_at
                    ? `Last validated ${formatDate(qualification.last_validated_at)}`
                    : "Validation date not reported"}
                </small>
              </article>
            {:else}
              <p class="staff-unavailable">
                Class qualifications were not reported for this staff member.
              </p>
            {/each}
          </section>
        {:else}
          <section
            id="staff-panel-evidence"
            class="staff-evidence"
            role="tabpanel"
          >
            <article
              class="responsive-surface"
              use:responsiveSurface={{ enabled: responsiveSurfaces }}
            >
              <span>Avatar artwork</span>
              <strong
                >{selectedMember.avatar_reference
                  ? "Provider reference reported"
                  : "Not reported"}</strong
              >
              <p>
                OnAir's public response does not provide a usable avatar URL.
                WyrmGrid will not construct one or substitute invented artwork.
              </p>
            </article>
            <article
              class="responsive-surface"
              use:responsiveSurface={{ enabled: responsiveSurfaces }}
            >
              <span>Snapshot provenance</span>
              <strong>{view?.snapshot.provenance.kind ?? "Unavailable"}</strong>
              <p>{view?.snapshot.provenance.source ?? "Source unavailable"}</p>
            </article>
            <article
              class="responsive-surface"
              use:responsiveSurface={{ enabled: responsiveSurfaces }}
            >
              <span>Unavailable staff facts</span>
              <strong>Preserved as unavailable</strong>
              <p>
                General certifications and human-readable provider enum labels
                are not present in the verified response and remain uninferred.
              </p>
            </article>
          </section>
        {/if}

        <footer>
          <p>
            General certifications are unavailable in the current verified
            response and are deliberately not inferred.
          </p>
          <div>
            <strong>{view?.availability ?? "unavailable"}</strong>
            <span
              >{view?.storage ?? "not stored"} · {observationLabel(
                view?.snapshot.provenance.observed_at,
              )}</span
            >
          </div>
        </footer>
      </article>
    {:else}
      <div class="staff-empty-stage">
        <span aria-hidden="true">◇</span>
        <h2>Staff roster awaiting facts</h2>
        <p>
          This area remains empty until OnAir reports staff data. Missing facts
          will remain visibly unavailable.
        </p>
      </div>
    {/if}
  </main>
</section>
