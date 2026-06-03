<script lang="ts">
  import { onMount } from "svelte";
  import { api, listen, type UnlistenFn } from "$lib/ipc";
  import { store } from "$lib/store.svelte";
  import { confirmCtl } from "$lib/confirm.svelte";
  import type { LogEntry } from "$lib/types";
  import { formatDate } from "$lib/format";
  import Select from "$lib/components/Select.svelte";
  import VirtualList from "$lib/components/VirtualList.svelte";

  let logs = $state<LogEntry[]>([]);
  let filter = $state("all");

  async function reload() {
    try {
      logs = (await api.getLogs()).slice().reverse();
    } catch (e) {
      store.toast("error", String(e));
    }
  }

  const filtered = $derived(filter === "all" ? logs : logs.filter((l) => l.level === filter));

  onMount(() => {
    reload();
    let un: UnlistenFn | undefined;
    listen("sync:finished", () => reload()).then((fn) => (un = fn));
    return () => un?.();
  });

  async function clear() {
    const ok = await confirmCtl.ask("Vider le journal ?", {
      title: "Vider le journal",
      confirmLabel: "Vider",
      danger: true,
    });
    if (!ok) return;
    await api.clearLogs();
    reload();
  }

  const actionColor: Record<string, string> = {
    copy: "var(--green)",
    update: "var(--accent)",
    delete: "var(--red)",
    skip: "var(--orange)",
    error: "var(--red)",
    info: "var(--text-2)",
  };

  const filterOptions = [
    { value: "all", label: "Tout" },
    { value: "info", label: "Infos" },
    { value: "warn", label: "Avertissements" },
    { value: "error", label: "Erreurs" },
  ];
</script>

<div class="dash">
  <header class="page-head">
    <h1>Journal</h1>
    <div class="spacer"></div>
    <Select bind:value={filter} options={filterOptions} width="170px" />
    <button class="btn btn-sm" onclick={reload}>Rafraîchir</button>
    <button class="btn btn-sm btn-danger" onclick={clear}>Vider</button>
  </header>

  {#if filtered.length === 0}
    <p class="muted">Aucune entrée.</p>
  {:else}
    <div class="card logs-area">
      <VirtualList items={filtered} itemHeight={30}>
        {#snippet row(l: LogEntry, i: number)}
          <div
            class="logrow"
            class:err={l.level === "error"}
            class:warn={l.level === "warn"}
            class:odd={i % 2 === 1}
          >
            <span class="time mono">{formatDate(l.at)}</span>
            <span class="action" style="color:{actionColor[l.action] ?? 'var(--text-2)'}">{l.action}</span>
            <span class="pair muted">{store.pairName(l.pairId)}</span>
            <span class="msg">
              {l.message}{#if l.path}<span class="mono pathtxt"> · {l.path}</span>{/if}
            </span>
          </div>
        {/snippet}
      </VirtualList>
    </div>
  {/if}
</div>

<style>
  .dash {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: var(--s5) var(--s6) var(--s5);
    animation: pageIn 0.25s var(--ease);
  }
  .page-head {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin-bottom: var(--s4);
  }
  .logs-area {
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
    padding: 4px;
  }
  .logrow {
    display: grid;
    grid-template-columns: 110px 70px 130px 1fr;
    gap: var(--s3);
    padding: 0 10px;
    height: 100%;
    align-items: center;
    border-radius: 6px;
    font-size: 13px;
  }
  .logrow.odd {
    background: var(--hover);
  }
  .logrow.err {
    background: color-mix(in srgb, var(--red) 10%, transparent);
  }
  .logrow.warn {
    background: color-mix(in srgb, var(--orange) 10%, transparent);
  }
  .time {
    color: var(--text-2);
  }
  .action {
    font-weight: 600;
    text-transform: capitalize;
  }
  .pair,
  .msg {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pathtxt {
    color: var(--text-2);
  }
</style>
