<script lang="ts">
  import "./jobs.css";
  import { translation } from "$lib/i18n/runtime";
  import type { JobSnapshotView, JobSummary } from "$lib/atlas/types";

  let {
    view,
    busy = false,
    errorMessage = "",
    onsynchronize,
    ondispatch,
  }: {
    view: JobSnapshotView | null;
    busy?: boolean;
    errorMessage?: string;
    onsynchronize: () => void;
    ondispatch: (jobId: string) => void;
  } = $props();

  let selectedJobId = $state<string | null>(null);
  const jobs = $derived(view?.snapshot.value.jobs ?? []);
  const selectedJob = $derived(
    jobs.find((job) => job.id === selectedJobId) ?? jobs[0] ?? null,
  );

  function route(job: JobSummary): string {
    const first = job.legs[0]?.departure?.icao;
    const last = job.legs.at(-1)?.destination?.icao;
    return first && last
      ? `${first} → ${last}`
      : $translation("jobs-route-unavailable");
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
    if (!value) return $translation("jobs-not-reported");
    const date = new Date(value);
    return Number.isNaN(date.getTime())
      ? $translation("jobs-not-reported")
      : date.toLocaleString([], { dateStyle: "medium", timeStyle: "short" });
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

    {#if errorMessage}<p class="jobs-error">{errorMessage}</p>{/if}

    <div class="jobs-list" aria-label={$translation("jobs-list-label")}>
      {#each jobs as job (job.id)}
        <button
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
          <strong>{$translation("jobs-empty-title")}</strong>
          <span>{$translation("jobs-empty-detail")}</span>
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

        <div class="job-metrics">
          <article>
            <span>{$translation("jobs-route")}</span><strong
              >{route(selectedJob)}</strong
            >
          </article>
          <article>
            <span>{$translation("jobs-pay")}</span><strong
              >{formatMoney(selectedJob.reported_pay)}</strong
            >
          </article>
          <article>
            <span>{$translation("jobs-cargo")}</span><strong
              >{cargo(selectedJob).toLocaleString()} lb</strong
            >
          </article>
          <article>
            <span>{$translation("jobs-passengers")}</span><strong
              >{passengers(selectedJob)}</strong
            >
          </article>
          <article>
            <span>{$translation("jobs-expires")}</span><strong
              >{formatDate(selectedJob.expires_at)}</strong
            >
          </article>
          <article>
            <span>{$translation("jobs-legs")}</span><strong
              >{selectedJob.legs.length}</strong
            >
          </article>
        </div>

        <div class="job-route-list">
          {#each selectedJob.legs as leg, index (leg.id)}
            <article>
              <span class="job-leg-index">{index + 1}</span>
              <div>
                <strong
                  >{leg.departure?.icao ?? "—"} → {leg.destination?.icao ??
                    "—"}</strong
                >
                <small
                  >{leg.description ??
                    (leg.kind === "cargo"
                      ? $translation("jobs-cargo-leg")
                      : $translation("jobs-passenger-leg"))}</small
                >
              </div>
              <span
                >{leg.distance_nm
                  ? `${Math.round(leg.distance_nm)} nm`
                  : "—"}</span
              >
            </article>
          {/each}
        </div>

        <footer>
          <p>{$translation("jobs-read-only-note")}</p>
          <button type="button" onclick={() => ondispatch(selectedJob.id)}>
            {$translation("jobs-open-dispatch")}
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
