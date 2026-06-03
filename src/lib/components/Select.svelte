<script lang="ts">
  // Select custom : bouton + liste déroulante stylée. Taille alignée sur les boutons.
  let {
    value = $bindable<string>(""),
    options,
    width = "auto",
    onchange,
  }: {
    value?: string;
    options: { value: string; label: string }[];
    width?: string;
    onchange?: (value: string) => void;
  } = $props();

  let open = $state(false);
  let root: HTMLElement | undefined = $state();

  const current = $derived(options.find((o) => o.value === value)?.label ?? value);

  function pick(v: string) {
    value = v;
    open = false;
    onchange?.(v);
  }

  $effect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (root && !root.contains(e.target as Node)) open = false;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") open = false;
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  });
</script>

<div class="sel-root" bind:this={root} style="width:{width}">
  <button type="button" class="btn btn-sm sel-btn" class:open onclick={() => (open = !open)}>
    <span class="sel-val">{current}</span>
    <svg class="chev" width="10" height="10" viewBox="0 0 12 12" fill="none">
      <path d="M2.5 4.5L6 8l3.5-3.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </button>

  {#if open}
    <div class="sel-pop">
      {#each options as o}
        <button type="button" class="sel-opt" class:active={o.value === value} onclick={() => pick(o.value)}>
          <span>{o.label}</span>
          {#if o.value === value}
            <svg width="12" height="12" viewBox="0 0 14 14" fill="none">
              <path d="M2.5 7.5L6 11l5.5-7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sel-root {
    position: relative;
  }
  .sel-btn {
    width: 100%;
    justify-content: space-between;
    gap: 8px;
  }
  .chev {
    opacity: 0.6;
    transition: transform 0.15s var(--ease);
    flex: 0 0 auto;
  }
  .sel-btn.open .chev {
    transform: rotate(180deg);
  }
  .sel-val {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sel-pop {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    min-width: 100%;
    width: max-content;
    max-width: 280px;
    max-height: 240px;
    overflow-y: auto;
    overscroll-behavior: contain;
    background: var(--bg-elev);
    border: 1px solid var(--hairline);
    border-radius: var(--r-control);
    box-shadow: var(--shadow-modal);
    padding: 4px;
    z-index: 40;
    animation: pop 0.12s var(--ease);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  .sel-opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    color: var(--text);
    padding: 7px 9px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
  }
  .sel-opt:hover {
    background: var(--hover);
  }
  .sel-opt.active {
    color: var(--accent);
    font-weight: 600;
  }
</style>
