<script lang="ts" generics="T">
  import type { Snippet } from "svelte";

  // Liste virtualisée : ne rend que les éléments visibles (+ overscan).
  // itemHeight = hauteur fixe d'une ligne en px.
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

  function onScroll() {
    if (vp) scrollTop = vp.scrollTop;
  }
</script>

<div class="vlist" bind:this={vp} bind:clientHeight={vh} onscroll={onScroll}>
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
