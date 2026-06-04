<script lang="ts">
  import { onMount } from "svelte";

  let bootEl: HTMLDivElement;
  let pctEl: HTMLSpanElement;

  onMount(() => {
    if (!pctEl || !bootEl) return;

    const start = performance.now();
    const dur = 3000;
    let raf = 0;

    const tick = (t: number) => {
      const v = Math.min(100, Math.round(((t - start) / dur) * 100));
      pctEl.textContent = v + "%";
      if (v < 100) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });
</script>

<div class="boot" bind:this={bootEl}>
  <span class="brand">◑</span>
  <svg class="track" viewBox="0 0 200 6" preserveAspectRatio="none" aria-hidden="true">
    <rect class="bg" x="0" y="0" width="200" height="6" rx="3" ry="3" />
    <rect class="fill" x="0" y="0" width="0" height="6" rx="3" ry="3">
      <animate attributeName="width" from="0" to="200" dur="3s" fill="freeze" begin="0s" />
    </rect>
  </svg>
  <span class="pct" bind:this={pctEl}>0%</span>
</div>

<style>
  .boot {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: var(--bg);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--s3);
  }
  .brand {
    font-size: 38px;
    line-height: 1;
    color: var(--accent);
    opacity: 0.7;
  }
  .track {
    width: 200px;
    height: 6px;
    display: block;
  }
  .track .bg {
    fill: var(--hover);
  }
  .track .fill {
    fill: var(--accent);
  }
  .pct {
    font-size: 12px;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
</style>