<script lang="ts">
  import { store } from "$lib/store.svelte";
  import { api } from "$lib/ipc";
  import Select from "$lib/components/Select.svelte";

  const methods = $derived(store.compressionMethods);
  const available = $derived(methods.filter((m) => m.available));
  const methodOptions = $derived([
    { value: "off", label: "Aucune (pas de compression)" },
    ...available.map((m) => ({ value: m.id, label: m.name })),
  ]);

  let refreshing = $state(false);
  async function refresh() {
    if (refreshing) return;
    refreshing = true;
    await store.loadCompressionMethods();
    refreshing = false;
  }

  let local = $state<{ method: string; level: number } | null>(null);
  $effect(() => {
    if (store.settings && !local) {
      local = {
        method: store.settings.defaultCompressionMethod ?? "off",
        level: store.settings.defaultCompressionLevel ?? 0,
      };
    }
  });
  $effect(() => {
    if (!local || local.method === "off" || methods.length === 0) return;
    const cur = local.method;
    if (!methods.some((m) => m.id === cur)) {
      local.method = "deflate";
    }
  });

  const selectedMeta = $derived(methods.find((m) => m.id === local?.method));

  async function persistDefault() {
    if (!store.settings || !local) return;
    try {
      await api.updateSettings({
        ...store.settings,
        defaultCompressionMethod: local.method,
        defaultCompressionLevel: local.level,
      });
      await store.refresh();
    } catch (e) {
      store.toast("error", String(e));
    }
  }
  function onMethodChange() {
    if (!local) return;
    const method = local.method;
    const meta = methods.find((m) => m.id === method);
    local.level = meta ? meta.defaultLevel : 0;
    persistDefault();
  }
  function install(url: string) {
    if (url) api.openUrl(url).catch((e) => store.toast("error", String(e)));
  }
</script>

<div class="page-scroll">
  <header class="page-head">
    <h1>Formats de compression</h1>
    <div class="spacer"></div>
    <button class="btn refresh-btn" onclick={refresh} disabled={refreshing}>
      {#if refreshing}<span class="rfill"></span>{/if}
      <span class="rlbl">Rafraîchir</span>
    </button>
  </header>

  <section class="card def">
    <h2>Méthode par défaut</h2>
    <p class="muted small">
      Pré-sélectionnée pour les nouvelles paires (modifiable ensuite dans « Modifier la paire »).
    </p>
    {#if local}
      <div class="row ctrls">
        <div class="method-sel">
          <Select bind:value={local.method} options={methodOptions} onchange={onMethodChange} width="240px" />
        </div>
        <label class="lvl" class:hidden={(selectedMeta?.maxLevel ?? 0) === 0}>
          <span>Niveau {local.level} / {selectedMeta?.maxLevel ?? 9}</span>
          <input
            type="range"
            min="1"
            max={selectedMeta?.maxLevel ?? 9}
            bind:value={local.level}
            onchange={persistDefault}
            disabled={(selectedMeta?.maxLevel ?? 0) === 0}
          />
        </label>
      </div>
    {/if}
  </section>

  {#if methods.length === 0}
    <p class="muted">Chargement…</p>
  {:else}
    <p class="muted small">
      Les codecs <strong>intégrés</strong> (ZIP : Deflate / Bzip2 / Zstandard) ne nécessitent rien et
      se mettent à jour de façon incrémentale. Les formats <strong>externes</strong> (7-Zip, tar.*)
      demandent l'outil installé et reconstruisent l'archive entière à chaque synchro.
    </p>
    <table class="grid">
      <thead>
        <tr>
          <th>Format</th>
          <th>Extension</th>
          <th>Compression</th>
          <th>Mot de passe</th>
          <th>Niveaux</th>
          <th>État</th>
        </tr>
      </thead>
      <tbody>
        {#each methods as m (m.id)}
          <tr class:off={!m.available}>
            <td class="name">{m.name}</td>
            <td><code>{m.extension}</code></td>
            <td><span class="ratio r-{m.id}">{m.ratio}</span></td>
            <td>
              {#if m.supportsPassword}
                <span class="ok">✓ Oui</span>
              {:else}
                <span class="muted no-pwd">— Non</span>
              {/if}
            </td>
            <td>{m.maxLevel > 0 ? `1 – ${m.maxLevel}` : "—"}</td>
            <td>
              {#if m.builtin}
                <span class="ok">✓ Intégré</span>
              {:else if m.available}
                <span class="ok">✓ Installé</span>
              {:else}
                <button class="install" onclick={() => install(m.downloadUrl)}>Installer →</button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .page-head {
    display: flex;
    align-items: center;
    margin-bottom: var(--s4);
  }
  .def {
    padding: var(--s3) var(--s4);
    margin-bottom: var(--s3);
  }
  .def h2 {
    margin-bottom: 4px;
    font-size: 14px;
  }
  .small {
    font-size: 12px;
    margin-bottom: var(--s3);
  }
  .ctrls {
    gap: var(--s4);
    flex-wrap: wrap;
  }
  .method-sel {
    min-width: 240px;
  }
  .refresh-btn {
    position: relative;
    overflow: hidden;
  }
  .refresh-btn .rfill {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 42%;
    border-radius: inherit;
    background: color-mix(in srgb, var(--accent) 24%, transparent);
    animation: rindet 1.1s ease-in-out infinite;
  }
  .refresh-btn .rlbl {
    position: relative;
    z-index: 1;
  }
  @keyframes rindet {
    0% {
      transform: translateX(-120%);
    }
    100% {
      transform: translateX(300%);
    }
  }
  .lvl {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    flex: 1;
    min-width: 200px;
  }
  .lvl.hidden {
    visibility: hidden;
  }
  .lvl input {
    width: 100%;
  }
  .grid {
    width: 100%;
    border-collapse: collapse;
    background: var(--bg-elev);
    border: 1px solid var(--hairline);
    border-radius: var(--r-card);
    overflow: hidden;
  }
  .grid th,
  .grid td {
    text-align: left;
    padding: 5px 12px;
    border-bottom: 1px solid var(--hairline);
    font-size: 13px;
  }
  .grid th {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
    background: var(--bg-sunken);
  }
  .grid tbody tr:last-child td {
    border-bottom: none;
  }
  .name {
    font-weight: 600;
  }
  tr.off {
    opacity: 0.6;
  }
  .ok {
    color: var(--green);
    font-weight: 600;
    font-size: 12px;
  }
  .no-pwd {
    font-size: 12px;
  }
  .install {
    border: none;
    background: none;
    color: var(--accent);
    font-weight: 600;
    font-size: 12px;
    cursor: pointer;
    padding: 0;
  }
  .install:hover {
    text-decoration: underline;
  }
  .ratio {
    font-weight: 600;
    font-size: 12px;
  }
  .r-store {
    color: var(--text-2);
  }
  .r-deflate {
    color: var(--accent);
  }
  .r-bzip2 {
    color: var(--orange);
  }
  .r-zstd {
    color: var(--green);
  }
  code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    background: var(--hover);
    padding: 1px 6px;
    border-radius: 4px;
  }
</style>
