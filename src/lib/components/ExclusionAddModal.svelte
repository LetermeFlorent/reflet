<script lang="ts">
  import { api } from "$lib/ipc";
  import { store } from "$lib/store.svelte";

  let {
    initial = "",
    onClose,
    onSubmit,
  }: { initial?: string; onClose: () => void; onSubmit: (patterns: string[]) => void } = $props();

  type Mode = "folder" | "files" | "manual";
  let mode = $state<Mode>(initial ? "manual" : "folder");

  let folderName = $state("");
  let recursive = $state(true);
  const folderPattern = $derived.by(() => {
    const n = folderName.trim().replace(/[\\/]+$/, "");
    if (!n) return "";
    return recursive ? `**/${n}/**` : `${n}/**`;
  });

  let files = $state<string[]>([]);
  let fileName = $state("");

  let manual = $state(initial);

  function base(p: string): string {
    return p.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? p;
  }

  async function browseFolder() {
    try {
      const d = await api.pickFolder();
      if (d) folderName = base(d);
    } catch (e) {
      store.toast("error", String(e));
    }
  }
  async function browseFiles() {
    try {
      for (const f of await api.pickFiles()) {
        const b = base(f);
        if (b && !files.includes(b)) files.push(b);
      }
    } catch (e) {
      store.toast("error", String(e));
    }
  }
  function addFileName() {
    const b = fileName.trim();
    if (b && !files.includes(b)) files.push(b);
    fileName = "";
  }

  const result = $derived.by(() => {
    if (mode === "folder") return folderPattern ? [folderPattern] : [];
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
    <h1>{initial ? "Modifier l'exclusion" : "Ajouter une exclusion"}</h1>

    <div class="tabs">
      <button class="tab" class:on={mode === "folder"} onclick={() => (mode = "folder")}>Dossier</button>
      <button class="tab" class:on={mode === "files"} onclick={() => (mode = "files")}>Fichiers</button>
      <button class="tab" class:on={mode === "manual"} onclick={() => (mode = "manual")}>Manuel (glob)</button>
    </div>

    {#if mode === "folder"}
      <div class="field">
        <span class="label">Nom du dossier à exclure</span>
        <div class="row">
          <input class="input" bind:value={folderName} placeholder="ex. node_modules" />
          <button class="btn" onclick={browseFolder}>Parcourir…</button>
        </div>
      </div>
      <label class="chk"><input type="checkbox" bind:checked={recursive} /> À n'importe quel niveau (récursif)</label>
    {:else if mode === "files"}
      <div class="field">
        <span class="label">Fichiers à exclure (par nom)</span>
        <div class="row">
          <input
            class="input"
            bind:value={fileName}
            placeholder="ex. secret.txt"
            onkeydown={(e) => e.key === "Enter" && addFileName()}
          />
          <button class="btn" onclick={addFileName}>Ajouter</button>
          <button class="btn" onclick={browseFiles}>Sélectionner…</button>
        </div>
      </div>
      {#if files.length}
        <div class="chips">
          {#each files as f, i (f)}
            <span class="chip">{f}<button class="x" onclick={() => files.splice(i, 1)} aria-label="retirer">×</button></span>
          {/each}
        </div>
      {/if}
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
        {initial ? "Enregistrer" : "Ajouter"}
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
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: var(--s3);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--hover);
    border-radius: var(--r-control);
    padding: 3px 4px 3px 10px;
    font-size: 12px;
    font-family: ui-monospace, Menlo, Consolas, monospace;
  }
  .chip .x {
    border: none;
    background: none;
    cursor: pointer;
    color: var(--text-2);
    font-size: 15px;
    line-height: 1;
    padding: 0 4px;
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
