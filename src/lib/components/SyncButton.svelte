<script lang="ts">
  let { isSyncing, pct, indeterminate, onclick }: {
    isSyncing: boolean;
    pct: number;
    indeterminate: boolean;
    onclick: () => void;
  } = $props();
</script>

<button class="btn btn-sm sync-btn" class:syncing={isSyncing} {onclick} disabled={isSyncing}>
  {#if isSyncing}
    <span class="fill" class:indet={indeterminate} style="width:{pct}%"></span>
  {/if}
  <span class="lbl">{isSyncing ? (indeterminate ? "…" : pct + "%") : "Synchroniser"}</span>
</button>

<style>
  .sync-btn {
    position: relative;
    overflow: hidden;
    min-width: 112px;
  }
  .sync-btn.syncing {
    border-color: var(--card-glow, var(--accent));
    color: var(--card-glow, var(--accent));
  }
  .sync-btn .fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 0;
    background: color-mix(in srgb, var(--card-glow, var(--accent)) 38%, transparent);
    transition: width 0.25s var(--ease);
  }
  .sync-btn .fill.indet {
    width: 45% !important;
    animation: indet 1.1s ease-in-out infinite;
  }
  @keyframes indet {
    0% {
      transform: translateX(-110%);
    }
    100% {
      transform: translateX(260%);
    }
  }
  .sync-btn .lbl {
    position: relative;
    z-index: 1;
  }
</style>
