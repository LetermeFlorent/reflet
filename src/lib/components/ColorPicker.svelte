<script lang="ts">
  let { value = $bindable("") }: { value: string } = $props();

  const COLORS = [
    { id: "", name: "Aucune" },
    { id: "bleu", name: "Bleu" },
    { id: "vert", name: "Vert" },
    { id: "orange", name: "Orange" },
    { id: "rose", name: "Rose" },
    { id: "violet", name: "Violet" },
    { id: "teal", name: "Teal" },
    { id: "jaune", name: "Jaune" },
    { id: "rouge", name: "Rouge" },
  ];
</script>

<div class="color-picker">
  {#each COLORS as c}
    <button
      class="color-oval"
      class:selected={value === c.id}
      style={c.id ? `background:var(--color-${c.id}-bg)` : ""}
      title={c.name}
      onclick={() => (value = c.id)}
    >
      {#if !c.id}
        <span class="color-none">∅</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .color-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .color-oval {
    width: 32px;
    height: 22px;
    border-radius: 11px;
    border: 2px solid var(--border);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-elev);
    transition: border-color 0.15s, transform 0.1s;
  }
  .color-oval:hover {
    transform: scale(1.1);
  }
  .color-oval.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .color-none {
    font-size: 12px;
    color: var(--text-2);
    line-height: 1;
  }
</style>
