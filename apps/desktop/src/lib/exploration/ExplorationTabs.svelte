<script lang="ts">
  export type ExplorationTab = {
    id: string;
    label: string;
  };

  let {
    tabs,
    selected = $bindable(),
    label,
    idPrefix,
  }: {
    tabs: readonly ExplorationTab[];
    selected: string;
    label: string;
    idPrefix: string;
  } = $props();
</script>

<div class="exploration-tabs" role="tablist" aria-label={label}>
  {#each tabs as tab (tab.id)}
    <button
      id={`${idPrefix}-tab-${tab.id}`}
      type="button"
      role="tab"
      aria-selected={selected === tab.id}
      aria-controls={`${idPrefix}-panel-${tab.id}`}
      onclick={() => (selected = tab.id)}
    >
      {tab.label}
    </button>
  {/each}
</div>
