<script lang="ts">
  let { times = $bindable<string[]>([]) }: { times: string[] } = $props();

  const TIME_RE = /^([01]\d|2[0-3]):[0-5]\d$/;
  let input = $state("");
  const valid = $derived(TIME_RE.test(input.trim()));

  function add() {
    const t = input.trim();
    if (TIME_RE.test(t) && !times.includes(t)) times = [...times, t];
    input = "";
  }
  function remove(i: number) {
    times = times.filter((_, j) => j !== i);
  }
</script>

<div class="row">
  <input
    class="input"
    style="width:100px"
    placeholder="HH:MM"
    bind:value={input}
    onkeydown={(e) => e.key === "Enter" && add()}
  />
  <button class="btn btn-sm" onclick={add} disabled={!valid}>Ajouter</button>
</div>
{#if times.length > 0}
  <div class="chips">
    {#each times as t, i (t)}
      <span class="chip">{t}<button class="chip-x" onclick={() => remove(i)} aria-label="retirer">×</button></span>
    {/each}
  </div>
{/if}

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--bg-2);
    font-size: 12px;
  }
  .chip-x {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    color: var(--text-2);
    padding: 0 2px;
  }
  .chip-x:hover {
    color: var(--red);
  }
</style>
