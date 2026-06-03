<script lang="ts">
  let { done, total }: { done: number; total: number } = $props();
  const pct = $derived(total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 0);
</script>

<div class="track" title="{done}/{total}">
  <div class="fill" style="width:{total > 0 ? pct : 100}%" class:indeterminate={total === 0}></div>
</div>

<style>
  .track {
    height: 4px;
    width: 100%;
    background: var(--bg-sunken);
    border-radius: 4px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: 4px;
    transition: width 0.25s var(--ease);
  }
  .indeterminate {
    width: 40% !important;
    animation: slide 1.1s ease-in-out infinite;
  }
  @keyframes slide {
    0% {
      margin-left: -40%;
    }
    100% {
      margin-left: 100%;
    }
  }
</style>
