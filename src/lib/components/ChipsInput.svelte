<script lang="ts">
  let {
    label,
    placeholder = "",
    items = $bindable([]),
    browseLabel,
    onBrowse,
  }: {
    label: string;
    placeholder?: string;
    items: string[];
    browseLabel: string;
    onBrowse: () => void;
  } = $props();

  let input = $state("");

  function add() {
    const v = input.trim();
    // Réassignation (pas .push) : garantit la propagation au parent via $bindable (Svelte 5).
    if (v && !items.includes(v)) items = [...items, v];
    input = "";
  }
</script>

<div class="field">
  <span class="label">{label}</span>
  <div class="row">
    <input class="input" bind:value={input} {placeholder} onkeydown={(e) => e.key === "Enter" && add()} />
    <button class="btn" onclick={add}>Ajouter</button>
    <button class="btn" onclick={onBrowse}>{browseLabel}</button>
  </div>
</div>

{#if items.length}
  <div class="chips">
    {#each items as it, i (it)}
      <span class="chip">{it}<button class="x" onclick={() => (items = items.filter((_, idx) => idx !== i))} aria-label="retirer">×</button></span>
    {/each}
  </div>
{/if}

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: var(--s3);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--hover);
    border-radius: var(--r-control);
    padding: 3px 4px 3px 10px;
    font-size: 12px;
    font-family: ui-monospace, Menlo, Consolas, monospace;
  }
  .chip .x {
    border: none;
    background: none;
    cursor: pointer;
    color: var(--text-2);
    font-size: 15px;
    line-height: 1;
    padding: 0 4px;
  }
</style>
