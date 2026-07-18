<script lang="ts">
  import "./jobs.css";
  import type { JobSnapshotView, JobSummary } from "$lib/atlas/types";
  import ExplorationSummary from "$lib/exploration/ExplorationSummary.svelte";
  import ExplorationTabs from "$lib/exploration/ExplorationTabs.svelte";
  import { selectedOrFirst } from "$lib/exploration/collection";
  import { translation } from "$lib/i18n/runtime";
  import {
    formatLocalDateTime,
    mediumDateShortTime,
  } from "$lib/presentation/dateTime";
  import {
    activeJobFilterCount,
    defaultJobFilters,
    filterAndSortJobs,
    jobFilterOptions,
    jobRoute,
    type JobFilters,
  } from "./presentation";

  let {
    view,
    busy = false,
    errorMessage = "",
    routeContext = null,
    onsynchronize,
    ondispatch,
    onreturn,
  }: {
    view: JobSnapshotView | null;
    busy?: boolean;
    errorMessage?: string;
    routeContext?: string | null;
    onsynchronize: () => void;
    ondispatch: (jobId: string) => void;
    onreturn: () => void;
  } = $props();

  let selectedJobId = $state<string | null>(null);
  let detailSection = $state("overview");
  let filters = $state<JobFilters>({ ...defaultJobFilters });
  let appliedRouteContext = $state<string | null>(null);
  const jobs = $derived(view?.snapshot.value.jobs ?? []);
  const options = $derived(jobFilterOptions(jobs));
  const routeOptions = $derived(
    filters.route && !options.routes.includes(filters.route)
      ? [filters.route, ...options.routes]
      : options.routes,
  );
  const visibleJobs = $derived(filterAndSortJobs(jobs, filters));
  const activeFilterCount = $derived(activeJobFilterCount(filters));
  const planRouteApplied = $derived(
    Boolean(routeContext && filters.route === routeContext),
  );
  const selectedJob = $derived(
    selectedOrFirst(visibleJobs, selectedJobId, (job) => job.id),
  );
  const detailTabs = [
    { id: "overview", label: "Overview" },
    { id: "route", label: "Route" },
    { id: "payload", label: "Cargo & passengers" },
    { id: "evidence", label: "Source evidence" },
  ] as const;

  $effect(() => {
    if (routeContext && routeContext !== appliedRouteContext) {
      filters = { ...filters, route: routeContext };
      selectedJobId = null;
      appliedRouteContext = routeContext;
      return;
    }
    if (!routeContext && appliedRouteContext) {
      if (filters.route === appliedRouteContext) {
        filters = { ...filters, route: null };
        selectedJobId = null;
      }
      appliedRouteContext = null;
    }
  });

  function route(job: JobSummary): string {
    return jobRoute(job) ?? $translation("jobs-route-unavailable");
  }

  function cargo(job: JobSummary): number {
    return job.legs.reduce((sum, leg) => sum + (leg.cargo_weight_lb ?? 0), 0);
  }

  function passengers(job: JobSummary): number {
    return job.legs.reduce((sum, leg) => sum + (leg.passengers ?? 0), 0);
  }

  function formatMoney(value: number | undefined): string {
    return value === undefined
      ? $translation("jobs-not-reported")
      : new Intl.NumberFormat(undefined, {
          style: "currency",
          currency: "USD",
          maximumFractionDigits: 0,
        }).format(value);
  }

  function formatDate(value: string | undefined): string {
    return formatLocalDateTime(
      value,
      $translation("jobs-not-reported"),
      mediumDateShortTime,
    );
  }

  function resetFilters(): void {
    filters = { ...defaultJobFilters };
  }

  function restorePlanRoute(): void {
    if (!routeContext) return;
    filters = { ...filters, route: routeContext };
    selectedJobId = null;
  }
</script>

<section
  class="jobs-workspace"
  aria-label={$translation("jobs-workspace-label")}
>
  <aside class="jobs-list-panel">
    <div class="jobs-heading">
      <span class="eyebrow">{$translation("jobs-eyebrow")}</span>
      <h2>{$translation("jobs-title")}</h2>
      <p>{$translation("jobs-introduction")}</p>
    </div>

    <button
      class="jobs-sync"
      type="button"
      disabled={busy}
      onclick={onsynchronize}
    >
      {busy
        ? $translation("jobs-synchronizing")
        : $translation("jobs-synchronize")}
    </button>

    <label class="jobs-search">
      <span>Find work</span>
      <input
        type="search"
        placeholder="Mission, airport, route, or description"
        bind:value={filters.query}
      />
    </label>

    {#if routeContext}
      <div class="jobs-plan-scope" class:broadened={!planRouteApplied}>
        <div>
          <span>SimBrief plan context</span>
          <strong>{routeContext}</strong>
          <small>
            {planRouteApplied
              ? "Exact route filter applied. You can refine it further below."
              : "Plan context retained while broader presentation filters are shown."}
          </small>
        </div>
        <div class="jobs-plan-actions">
          {#if !planRouteApplied}
            <button type="button" onclick={restorePlanRoute}
              >Restore route</button
            >
          {/if}
          <button type="button" onclick={onreturn}>Return to Dispatch</button>
        </div>
      </div>
    {/if}

    <details class="jobs-filter-panel">
      <summary>
        <span>Filter and sort</span>
        {#if activeFilterCount > 0}<strong>{activeFilterCount} active</strong
          >{/if}
      </summary>
      <div class="jobs-filter-grid">
        <label>
          <span>Route</span>
          <select
            value={filters.route ?? ""}
            onchange={(event) =>
              (filters.route = event.currentTarget.value || null)}
          >
            <option value="">All reported routes</option>
            {#each routeOptions as routeOption}
              <option value={routeOption}>{routeOption}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>Mission type</span>
          <select
            value={filters.missionType ?? ""}
            onchange={(event) =>
              (filters.missionType = event.currentTarget.value || null)}
          >
            <option value="">All reported mission types</option>
            {#each options.missionTypes as missionType}
              <option value={missionType}>{missionType}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>Payload kind</span>
          <select bind:value={filters.payload}>
            <option value="all">Any reported payload</option>
            {#each options.payloadKinds as payloadKind}
              <option value={payloadKind}
                >{payloadKind === "mixed"
                  ? "Cargo and passengers"
                  : payloadKind === "cargo"
                    ? "Cargo"
                    : "Passengers"}</option
              >
            {/each}
          </select>
        </label>
        <label>
          <span>Expiry field</span>
          <select bind:value={filters.expiry}>
            <option value="all">Either state</option>
            <option value="reported">Reported by OnAir</option>
            <option value="unreported">Not reported</option>
          </select>
        </label>
        <label>
          <span>Order work by</span>
          <select bind:value={filters.sort}>
            <option value="mission">Mission type</option>
            <option value="route">Route</option>
            <option value="pay">Reported pay</option>
            <option value="expiry">Expiry</option>
            <option value="legs">Leg count</option>
          </select>
        </label>
      </div>
    </details>

    <ExplorationSummary
      shown={visibleJobs.length}
      total={jobs.length}
      label="jobs"
      activeFilters={activeFilterCount}
      onclear={resetFilters}
    />

    {#if errorMessage}<p class="jobs-error" role="alert">{errorMessage}</p>{/if}

    <div class="jobs-list" aria-label={$translation("jobs-list-label")}>
      {#each visibleJobs as job (job.id)}
        <button
          class="responsive-surface"
          class:active={selectedJob?.id === job.id}
          type="button"
          onclick={() => (selectedJobId = job.id)}
        >
          <span>{job.mission_type ?? $translation("jobs-unnamed")}</span>
          <strong>{route(job)}</strong>
          <small>{formatMoney(job.reported_pay)}</small>
        </button>
      {:else}
        <div class="jobs-empty-list">
          <strong>
            {jobs.length
              ? planRouteApplied
                ? `No pending jobs match ${routeContext}`
                : "No jobs match the current filters"
              : $translation("jobs-empty-title")}
          </strong>
          <span>
            {jobs.length
              ? planRouteApplied
                ? "WyrmGrid has not substituted unrelated OnAir work."
                : "Adjust the filters or return to the imported plan."
              : $translation("jobs-empty-detail")}
          </span>
          {#if jobs.length}
            <div class="jobs-empty-actions">
              <button type="button" onclick={resetFilters}
                >Show all pending jobs</button
              >
              {#if routeContext}
                <button type="button" onclick={onreturn}
                  >Return to Dispatch</button
                >
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </aside>

  <main class="jobs-stage">
    {#if selectedJob}
      <article class="job-brief">
        <header>
          <div>
            <span class="eyebrow">{$translation("jobs-selected-eyebrow")}</span>
            <h2>{selectedJob.mission_type ?? $translation("jobs-unnamed")}</h2>
            <p>
              {selectedJob.description ?? $translation("jobs-no-description")}
            </p>
          </div>
          <span class="jobs-source">{$translation("jobs-onair-fact")}</span>
        </header>

        <div class="job-detail-tabs">
          <ExplorationTabs
            tabs={detailTabs}
            bind:selected={detailSection}
            label="Job detail sections"
            idPrefix="job"
          />
        </div>

        {#if detailSection === "overview"}
          <section id="job-panel-overview" class="job-metrics" role="tabpanel">
            {#each [[$translation("jobs-route"), route(selectedJob)], [$translation("jobs-pay"), formatMoney(selectedJob.reported_pay)], [$translation("jobs-cargo"), `${cargo(selectedJob).toLocaleString()} lb`], [$translation("jobs-passengers"), passengers(selectedJob).toLocaleString()], [$translation("jobs-expires"), formatDate(selectedJob.expires_at)], [$translation("jobs-legs"), selectedJob.legs.length.toLocaleString()]] as [label, value]}
              <article class="responsive-surface">
                <span>{label}</span><strong>{value}</strong>
              </article>
            {/each}
          </section>
        {:else if detailSection === "route"}
          <section id="job-panel-route" class="job-route-list" role="tabpanel">
            {#each selectedJob.legs as leg, index (leg.id)}
              <article class="responsive-surface">
                <span class="job-leg-index">{index + 1}</span>
                <div>
                  <strong
                    >{leg.departure?.icao ?? "—"} → {leg.destination?.icao ??
                      "—"}</strong
                  >
                  <small>{leg.description ?? "Description not reported"}</small>
                </div>
                <span
                  >{leg.distance_nm
                    ? `${Math.round(leg.distance_nm)} nm`
                    : "—"}</span
                >
              </article>
            {/each}
          </section>
        {:else if detailSection === "payload"}
          <section
            id="job-panel-payload"
            class="job-payload-list"
            role="tabpanel"
          >
            {#each selectedJob.legs as leg, index (leg.id)}
              <article class="responsive-surface">
                <span class="job-leg-index">{index + 1}</span>
                <div>
                  <strong
                    >{leg.kind === "cargo" ? "Cargo" : "Passengers"}</strong
                  >
                  <small
                    >{leg.departure?.icao ?? "—"} → {leg.destination?.icao ??
                      "—"}</small
                  >
                </div>
                <span
                  >{leg.kind === "cargo"
                    ? leg.cargo_weight_lb === undefined
                      ? "Weight not reported"
                      : `${leg.cargo_weight_lb.toLocaleString()} lb`
                    : leg.passengers === undefined
                      ? "Count not reported"
                      : `${leg.passengers.toLocaleString()} passengers`}</span
                >
              </article>
            {/each}
          </section>
        {:else}
          <section id="job-panel-evidence" class="job-evidence" role="tabpanel">
            <article class="responsive-surface">
              <span>Snapshot provenance</span><strong
                >{view?.snapshot.provenance.kind ?? "Unavailable"}</strong
              >
            </article>
            <article class="responsive-surface">
              <span>Source</span><strong
                >{view?.snapshot.provenance.source ?? "Unavailable"}</strong
              >
            </article>
            <article class="responsive-surface">
              <span>Created</span><strong
                >{formatDate(selectedJob.created_at)}</strong
              >
            </article>
            <article class="responsive-surface">
              <span>Taken</span><strong
                >{formatDate(selectedJob.taken_at)}</strong
              >
            </article>
          </section>
        {/if}

        <footer>
          <p>{$translation("jobs-read-only-note")}</p>
          <button
            type="button"
            disabled={busy}
            onclick={() => ondispatch(selectedJob.id)}
          >
            {busy ? "Adding to Dispatch…" : $translation("jobs-open-dispatch")}
          </button>
        </footer>
      </article>
    {:else}
      <div class="jobs-empty-stage">
        <span aria-hidden="true">◇</span>
        <h2>{$translation("jobs-awaiting-title")}</h2>
        <p>{$translation("jobs-awaiting-detail")}</p>
      </div>
    {/if}
  </main>
</section>
