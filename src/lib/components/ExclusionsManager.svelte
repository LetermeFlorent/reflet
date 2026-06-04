<script lang="ts">
  import { confirmCtl } from "$lib/confirm.svelte";
  import ExclusionAddModal from "./ExclusionAddModal.svelte";

  let {
    patterns,
    onAdd,
    onReplace,
    onRemove,
    busy = false,
    showAddButton = true,
  }: {
    patterns: string[];
    onAdd: (ps: string[]) => void;
    onReplace: (i: number, ps: string[]) => void;
    onRemove: (i: number) => void;
    busy?: boolean;
    showAddButton?: boolean;
  } = $props();

  let modalOpen = $state(false);
  let editIdx = $state<number | null>(null);

  export function openAdd() {
    editIdx = null;
    modalOpen = true;
  }
  function openEdit(i: number) {
    editIdx = i;
    modalOpen = true;
  }
  function submit(ps: string[]) {
    if (editIdx !== null) onReplace(editIdx, ps);
    else onAdd(ps);
  }
  async function del(i: number) {
    const ok = await confirmCtl.ask(`Supprimer l'exclusion « ${patterns[i]} » ?`, {
      title: "Supprimer l'exclusion",
      confirmLabel: "Supprimer",
      danger: true,
    });
    if (ok) onRemove(i);
  }

  function kind(p: string): string {
    if (p.endsWith("/**")) return "Dossier";
    if (/\*\.[^/]+$/.test(p)) return "Type de fichier";
    if (p.includes("*")) return "Motif";
    return "Fichier / chemin";
  }
</script>

{#if showAddButton}
  <div class="head">
    <div class="spacer"></div>
    <button class="btn" onclick={openAdd} disabled={busy}>+ Ajouter</button>
  </div>
{/if}

{#if patterns.length === 0}
  <p class="muted empty">Aucune exclusion. Clique sur « + Ajouter ».</p>
{:else}
  <table class="grid">
    <thead>
      <tr><th class="c-type">Type</th><th>Motif</th><th class="c-act">Actions</th></tr>
    </thead>
    <tbody>
      {#each patterns as p, i (p + i)}
        <tr>
          <td class="c-type">{kind(p)}</td>
          <td><code>{p}</code></td>
          <td class="c-act">
            <button class="btn btn-sm" onclick={() => openEdit(i)} disabled={busy}>Modifier</button>
            <button class="btn btn-sm btn-danger" onclick={() => del(i)} disabled={busy}>Supprimer</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

{#if modalOpen}
  <ExclusionAddModal
    initial={editIdx !== null ? patterns[editIdx] : ""}
    onClose={() => (modalOpen = false)}
    onSubmit={submit}
  />
{/if}

<style>
  .head {
    display: flex;
    margin-bottom: var(--s2);
  }
  .empty {
    font-size: 13px;
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
    padding: 8px 12px;
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
  .c-type {
    width: 140px;
    color: var(--text-2);
  }
  .c-act {
    width: 1%;
    white-space: nowrap;
    text-align: right;
  }
  .c-act .btn {
    margin-left: 6px;
  }
  .grid code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
  }
</style>
