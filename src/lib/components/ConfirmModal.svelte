<script lang="ts">
  import { confirmCtl } from "$lib/confirm.svelte";

  function onKey(e: KeyboardEvent) {
    if (!confirmCtl.req) return;
    if (e.key === "Escape") confirmCtl.answer(false);
    else if (e.key === "Enter") confirmCtl.answer(true);
  }
</script>

<svelte:window onkeydown={onKey} />

{#if confirmCtl.req}
  <div class="modal-backdrop" onclick={() => confirmCtl.answer(false)} role="presentation">
    <div class="modal confirm" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <h2>{confirmCtl.req.title}</h2>
      <p>{confirmCtl.req.message}</p>
      <div class="row actions">
        <div class="spacer"></div>
        <button class="btn" onclick={() => confirmCtl.answer(false)}>Annuler</button>
        <button
          class="btn"
          class:btn-danger={confirmCtl.req.danger}
          onclick={() => confirmCtl.answer(true)}
        >
          {confirmCtl.req.confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .confirm {
    max-width: 420px;
  }
  .confirm h2 {
    margin-bottom: var(--s3);
  }
  .confirm p {
    color: var(--text-2);
    font-size: 13px;
    line-height: 1.55;
  }
  .actions {
    margin-top: var(--s4);
  }
</style>
