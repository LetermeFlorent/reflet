<script lang="ts">
  import type { SyncPlan } from "$lib/types";
  import { formatBytes } from "$lib/format";

  let {
    plan,
    pairName,
    loading,
    onClose,
    onConfirm,
  }: {
    plan: SyncPlan | null;
    pairName: string;
    loading: boolean;
    onClose: () => void;
    onConfirm: () => void;
  } = $props();

  const nothing = $derived(
    !!plan &&
      plan.toCopy.length === 0 &&
      plan.toOverwrite.length === 0 &&
      plan.toDelete.length === 0,
  );
</script>

<div class="modal-backdrop" onclick={onClose} role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
    <h1>Aperçu — {pairName}</h1>

    {#if loading}
      <p class="muted" style="margin-top:var(--s4)">Analyse en cours…</p>
    {:else if !plan}
      <p class="muted" style="margin-top:var(--s4)">Aucun plan.</p>
    {:else}
      {#if plan.abortedSafety}
        <div class="safety">
          🛑 Sécurité : {plan.deletePct}% de la destination serait supprimé. Les suppressions
          seront <strong>retenues</strong> tant que ce seuil est dépassé (vérifie que la source est
          bien montée, ou ajuste le seuil dans Réglages).
        </div>
      {/if}

      <div class="summary">
        <div class="stat"><b style="color:var(--green)">{plan.toCopy.length}</b><span>nouveaux</span></div>
        <div class="stat"><b style="color:var(--accent)">{plan.toOverwrite.length}</b><span>modifiés</span></div>
        <div class="stat"><b style="color:var(--red)">{plan.toDelete.length}</b><span>à supprimer</span></div>
        <div class="stat"><b>{formatBytes(plan.totalBytes)}</b><span>à copier</span></div>
      </div>

      {#if nothing}
        <p class="muted">Destination déjà identique à la source. Rien à faire.</p>
      {:else}
        <div class="lists">
          {#if plan.toCopy.length}
            <section>
              <h2 style="color:var(--green)">Nouveaux ({plan.toCopy.length})</h2>
              <ul class="mono">
                {#each plan.toCopy.slice(0, 200) as it}
                  <li>{it.rel}</li>
                {/each}
                {#if plan.toCopy.length > 200}<li class="muted">…et {plan.toCopy.length - 200} de plus</li>{/if}
              </ul>
            </section>
          {/if}
          {#if plan.toOverwrite.length}
            <section>
              <h2 style="color:var(--accent)">Modifiés ({plan.toOverwrite.length})</h2>
              <ul class="mono">
                {#each plan.toOverwrite.slice(0, 200) as it}
                  <li>{it.rel} <span class="muted">({it.reason})</span></li>
                {/each}
                {#if plan.toOverwrite.length > 200}<li class="muted">…et {plan.toOverwrite.length - 200} de plus</li>{/if}
              </ul>
            </section>
          {/if}
          {#if plan.toDelete.length}
            <section class="danger-section">
              <h2 style="color:var(--red)">À supprimer dans la destination ({plan.toDelete.length})</h2>
              <ul class="mono">
                {#each plan.toDelete.slice(0, 200) as it}
                  <li class="del">{it.rel}{it.isDir ? "/" : ""}</li>
                {/each}
                {#if plan.toDelete.length > 200}<li class="muted">…et {plan.toDelete.length - 200} de plus</li>{/if}
              </ul>
            </section>
          {/if}
        </div>
      {/if}
    {/if}

    <div class="row actions">
      <div class="spacer"></div>
      <button class="btn" onclick={onClose}>Fermer</button>
      <button class="btn" onclick={onConfirm} disabled={loading || !plan || nothing}>
        Appliquer maintenant
      </button>
    </div>
  </div>
</div>

<style>
  .summary {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--s3);
    margin: var(--s4) 0;
  }
  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: var(--s3);
    border-radius: var(--r-control);
    background: var(--bg-sunken);
  }
  .stat b {
    font-size: 20px;
  }
  .stat span {
    font-size: 11px;
    color: var(--text-2);
  }
  .lists {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    max-height: 42vh;
    overflow: auto;
  }
  section h2 {
    margin-bottom: var(--s2);
  }
  ul {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .danger-section {
    padding: var(--s3);
    border-radius: var(--r-control);
    background: color-mix(in srgb, var(--red) 8%, transparent);
  }
  .del {
    color: var(--red);
  }
  .safety {
    margin: var(--s4) 0;
    padding: var(--s3);
    border-radius: var(--r-control);
    background: color-mix(in srgb, var(--red) 14%, transparent);
    font-size: 13px;
  }
  .actions {
    margin-top: var(--s4);
  }
</style>
