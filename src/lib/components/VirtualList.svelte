<script lang="ts" generics="T">
  import type { Snippet } from "svelte";

  let {
    items,
    itemHeight,
    overscan = 8,
    row,
  }: {
    items: T[];
    itemHeight: number;
    overscan?: number;
    row: Snippet<[T, number]>;
  } = $props();

  let scrollTop = $state(0);
  let vh = $state(600);
  let vp: HTMLElement | undefined = $state();

  const total = $derived(items.length);
  const start = $derived(Math.max(0, Math.floor(scrollTop / itemHeight) - overscan));
  const visible = $derived(Math.ceil(vh / itemHeight) + overscan * 2);
  const end = $derived(Math.min(total, start + visible));
  const slice = $derived(items.slice(start, end));

  // Listener scroll passif + coalescé par frame : évite de bloquer le scroll et de
  // mettre à jour l'état plus d'une fois par frame sur les longues listes (journaux).
  $effect(() => {
    const el = vp;
    if (!el) return;
    let raf = 0;
    const handler = () => {
      if (raf) return;
      raf = requestAnimationFrame(() => {
        raf = 0;
        scrollTop = el.scrollTop;
      });
    };
    el.addEventListener("scroll", handler, { passive: true });
    return () => {
      el.removeEventListener("scroll", handler);
      if (raf) cancelAnimationFrame(raf);
    };
  });
</script>

<div class="vlist" bind:this={vp} bind:clientHeight={vh}>
  <div class="vspace" style="height:{total * itemHeight}px">
    <div class="vwin" style="transform:translateY({start * itemHeight}px)">
      {#each slice as item, i (start + i)}
        <div class="vrow" style="height:{itemHeight}px">
          {@render row(item, start + i)}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .vlist {
    height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
    position: relative;
  }
  .vspace {
    position: relative;
    width: 100%;
  }
  .vwin {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    will-change: transform;
  }
  .vrow {
    box-sizing: border-box;
  }
</style>
