<script lang="ts">
  import { onMount } from "svelte";

  // Remplissage piloté en JS (requestAnimationFrame) plutôt qu'en animation CSS :
  // visible même si "réduire les animations" est activé côté OS, et progresse vraiment.
  let pct = $state(0);

  onMount(() => {
    const start = performance.now();
    const dur = 3000;
    let raf = 0;
    const tick = (t: number) => {
      pct = Math.min(100, Math.round(((t - start) / dur) * 100));
      if (pct < 100) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });
</script>

<div class="boot">
  <span class="brand">◑</span>
  <div class="bar"><span style="width:{pct}%"></span></div>
  <span class="pct">{pct}%</span>
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
  .bar {
    width: 200px;
    height: 6px;
    border-radius: var(--r-control);
    background: var(--hover);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: var(--r-control);
    background: var(--accent);
  }
  .pct {
    font-size: 12px;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
</style>
