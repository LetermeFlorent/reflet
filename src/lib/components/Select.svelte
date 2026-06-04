<script lang="ts">
  let {
    value = $bindable<string>(""),
    options,
    width = "auto",
    onchange,
  }: {
    value?: string;
    options: { value: string; label: string; disabled?: boolean }[];
    width?: string;
    onchange?: (value: string) => void;
  } = $props();

  let open = $state(false);
  let root: HTMLElement | undefined = $state();
  let btnEl: HTMLElement | undefined = $state();
  let popEl: HTMLElement | undefined = $state();
  let popStyle = $state("position:fixed;top:0;left:0;visibility:hidden");

  const current = $derived(options.find((o) => o.value === value)?.label ?? value);

  function pick(v: string) {
    const opt = options.find(o => o.value === v);
    if (opt?.disabled) return;
    value = v;
    open = false;
    onchange?.(v);
  }

  function reposition() {
    if (!btnEl || !popEl) return;
    const rect = btnEl.getBoundingClientRect();
    const popRect = popEl.getBoundingClientRect();

    let top = rect.bottom + 4;
    let left = rect.left;

    // Flip above if not enough space below
    if (top + popRect.height > window.innerHeight - 8) {
      top = rect.top - popRect.height - 4;
    }
    // Clamp left
    if (left + popRect.width > window.innerWidth - 8) {
      left = window.innerWidth - popRect.width - 8;
    }
    if (left < 8) left = 8;

    popStyle = `position:fixed;top:${top}px;left:${left}px;min-width:${rect.width}px;`;
  }

  // Reposition when opened
  $effect(() => {
    if (!open) return;
    // Wait two frames for the popup to render and get layout
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        reposition();
      });
    });
    window.addEventListener("scroll", reposition, true);
    window.addEventListener("resize", reposition);
    return () => {
      window.removeEventListener("scroll", reposition, true);
      window.removeEventListener("resize", reposition);
    };
  });

  // Close on outside click / escape
  $effect(() => {
    if (!open) return;

    const onDown = (e: MouseEvent) => {
      if (root && !root.contains(e.target as Node)) {
        open = false;
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") open = false;
    };

    // Delay to avoid catching the click that opened it. On capture le rAF pour
    // l'annuler si l'effet est nettoyé avant qu'il ne s'exécute (sinon les listeners
    // seraient ajoutés après le cleanup → fuite à chaque réouverture).
    const raf = requestAnimationFrame(() => {
      window.addEventListener("mousedown", onDown);
      window.addEventListener("keydown", onKey);
    });

    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  });
</script>

<div class="sel-root" bind:this={root} style="width:{width}">
  <button type="button" bind:this={btnEl} class="btn btn-sm sel-btn" class:open onclick={() => (open = !open)}>
    <span class="sel-val">{current}</span>
    <svg class="chev" width="10" height="10" viewBox="0 0 12 12" fill="none">
      <path d="M2.5 4.5L6 8l3.5-3.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </button>

  {#if open}
    <div bind:this={popEl} class="sel-pop" style={popStyle}>
      {#each options as o}
        <button type="button" class="sel-opt" class:active={o.value === value} class:disabled={o.disabled} onclick={() => pick(o.value)} disabled={o.disabled}>
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
    width: max-content;
    max-width: 280px;
    max-height: 240px;
    overflow-y: auto;
    overscroll-behavior: contain;
    background: var(--bg-elev);
    border: 1px solid var(--hairline);
    border-radius: var(--r-control);
    box-shadow: var(--shadow-modal);
    padding: 3px;
    z-index: 100;
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
    padding: 4px 9px;
    border-radius: 5px;
    cursor: pointer;
    font-size: 12px;
  }
  .sel-opt:hover:not(.disabled) {
    background: var(--hover);
  }
  .sel-opt.active {
    color: var(--accent);
    font-weight: 600;
  }
  .sel-opt.disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
