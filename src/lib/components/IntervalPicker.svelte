<script lang="ts">
  import Select from "./Select.svelte";
  let {
    seconds = $bindable<number | null>(null),
    allowDefault = false,
    minSec = 5,
  }: { seconds?: number | null; allowDefault?: boolean; minSec?: number } = $props();

  const UNITS = [
    { k: "s", label: "secondes", f: 1 },
    { k: "min", label: "minutes", f: 60 },
    { k: "h", label: "heures", f: 3600 },
    { k: "j", label: "jours", f: 86400 },
  ];
  const unitOptions = UNITS.map((u) => ({ value: u.k, label: u.label }));

  function fromSeconds(sec: number) {
    for (let i = UNITS.length - 1; i >= 0; i--) {
      if (sec % UNITS[i].f === 0) return { unit: UNITS[i].k, value: sec / UNITS[i].f };
    }
    return { unit: "s", value: sec };
  }

  const initial = seconds != null ? fromSeconds(seconds) : { unit: "min", value: allowDefault ? "" : 15 };
  let isDefault = $state(allowDefault && seconds == null);
  let unit = $state(initial.unit);
  let value = $state(String(initial.value));

  function recompute() {
    if (allowDefault && isDefault) {
      seconds = null;
      return;
    }
    const f = UNITS.find((u) => u.k === unit)!.f;
    const n = parseInt(value);
    if (isNaN(n) || n <= 0) {
      seconds = allowDefault ? null : minSec;
      return;
    }
    seconds = Math.max(minSec, n * f);
  }
</script>

<div class="ip">
  {#if allowDefault}
    <label class="chk">
      <input type="checkbox" bind:checked={isDefault} onchange={recompute} />
      Défaut global
    </label>
  {/if}
  {#if !(allowDefault && isDefault)}
    <input class="input num" type="number" min="1" bind:value oninput={recompute} />
    <Select bind:value={unit} options={unitOptions} onchange={recompute} width="130px" />
  {/if}
</div>

<style>
  .ip {
    display: flex;
    align-items: center;
    gap: var(--s2);
    flex-wrap: wrap;
  }
  .num {
    width: 90px;
  }
  .chk {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    white-space: nowrap;
  }
</style>
