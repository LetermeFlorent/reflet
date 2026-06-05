<script lang="ts">
  import { api } from "$lib/ipc";
  import { store } from "$lib/store.svelte";
  import ChipsInput from "./ChipsInput.svelte";

  let {
    initial = "",
    onClose,
    onSubmit,
  }: { initial?: string; onClose: () => void; onSubmit: (patterns: string[]) => void } = $props();

  type Mode = "folder" | "files" | "manual";
  let mode = $state<Mode>(initial ? "manual" : "folder");

  let folders = $state<string[]>([]);
  let recursive = $state(true);
  let files = $state<string[]>([]);
  let manual = $state(initial);

  // Changer d'onglet repart à zéro : sinon des chips sélectionnées dans un mode
  // restent « fantômes » et le mode actif n'utilise que sa propre liste à la validation.
  function setMode(m: Mode) {
    if (m === mode) return;
    mode = m;
    folders = [];
    files = [];
    recursive = true;
    manual = "";
  }

  function base(p: string): string {
    return p.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? p;
  }
  function merge(into: string[], names: string[]) {
    for (const n of names) {
      const b = base(n);
      if (b && !into.includes(b)) into.push(b);
    }
  }
  async function browseFolders() {
    try {
      merge(folders, await api.pickFolders());
    } catch (e) {
      store.toast("error", String(e));
    }
  }
  async function browseFiles() {
    try {
      merge(files, await api.pickFiles());
    } catch (e) {
      store.toast("error", String(e));
    }
  }

  const result = $derived.by(() => {
    if (mode === "folder") return folders.map((f) => (recursive ? `**/${f}/**` : `${f}/**`));
    if (mode === "files") return files.map((f) => `**/${f}`);
    return manual
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean);
  });

  function submit() {
    if (result.length === 0) return;
    onSubmit(result);
    onClose();
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
    <h1>{initial ? "Modifier l'exclusion" : "Ajouter des exclusions"}</h1>

    <div class="tabs">
      <button class="tab" class:on={mode === "folder"} onclick={() => setMode("folder")}>Dossiers</button>
      <button class="tab" class:on={mode === "files"} onclick={() => setMode("files")}>Fichiers</button>
      <button class="tab" class:on={mode === "manual"} onclick={() => setMode("manual")}>Manuel (glob)</button>
    </div>

    {#if mode === "folder"}
      <ChipsInput
        label="Dossiers à exclure (par nom) — plusieurs possibles"
        placeholder="ex. node_modules"
        bind:items={folders}
        browseLabel="Sélectionner…"
        onBrowse={browseFolders}
      />
      <label class="chk"><input type="checkbox" bind:checked={recursive} /> À n'importe quel niveau (récursif)</label>
    {:else if mode === "files"}
      <ChipsInput
        label="Fichiers à exclure (par nom) — plusieurs possibles"
        placeholder="ex. secret.txt"
        bind:items={files}
        browseLabel="Sélectionner…"
        onBrowse={browseFiles}
      />
    {:else}
      <div class="field">
        <span class="label">Motifs glob (un par ligne)</span>
        <textarea class="input" rows="4" bind:value={manual} placeholder={"**/*.log\n**/cache/**"}></textarea>
      </div>
    {/if}

    {#if result.length}
      <div class="preview">
        <span class="muted">Motif{result.length > 1 ? "s" : ""} :</span>
        {#each result as r}<code>{r}</code>{/each}
      </div>
    {/if}

    <div class="row actions">
      <div class="spacer"></div>
      <button class="btn" onclick={onClose}>Annuler</button>
      <button class="btn" onclick={submit} disabled={result.length === 0}>
        {initial ? "Enregistrer" : `Ajouter${result.length > 1 ? ` (${result.length})` : ""}`}
      </button>
    </div>
  </div>
</div>

<style>
  .tabs {
    display: flex;
    gap: 4px;
    margin: var(--s4) 0;
  }
  .tab {
    flex: 1;
    height: 30px;
    border: 1px solid var(--border);
    background: var(--bg-elev);
    color: var(--text-2);
    border-radius: var(--r-control);
    cursor: pointer;
    font-size: 13px;
  }
  .tab.on {
    color: var(--text);
    background: var(--hover);
    font-weight: 600;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .chk {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    margin-top: var(--s2);
    cursor: pointer;
  }
  .preview {
    margin-top: var(--s4);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .preview code {
    font-family: ui-monospace, Menlo, Consolas, monospace;
    background: var(--hover);
    padding: 2px 8px;
    border-radius: var(--r-control);
  }
  .actions {
    margin-top: var(--s5);
  }
</style>
