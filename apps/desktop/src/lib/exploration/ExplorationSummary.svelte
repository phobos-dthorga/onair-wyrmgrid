<script lang="ts">
  let {
    shown,
    total,
    label,
    activeFilters = 0,
    clearLabel = "Clear filters",
    onclear,
  }: {
    shown: number;
    total: number;
    label: string;
    activeFilters?: number;
    clearLabel?: string;
    onclear: () => void;
  } = $props();
</script>

<div class="exploration-summary">
  <span role="status" aria-live="polite">
    <strong>{shown.toLocaleString()}</strong> of {total.toLocaleString()} {label}
    shown
  </span>
  <button type="button" disabled={activeFilters === 0} onclick={onclear}>
    {clearLabel}{activeFilters > 0 ? ` (${activeFilters})` : ""}
  </button>
</div>
