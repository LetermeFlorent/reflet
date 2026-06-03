<script lang="ts">
  import type { SyncPair } from "$lib/types";
  import { api } from "$lib/ipc";
  import { store } from "$lib/store.svelte";
  import Switch from "./Switch.svelte";
  import IntervalPicker from "./IntervalPicker.svelte";
  import { formatInterval } from "$lib/format";

  let {
    pair,
    onClose,
    onSaved,
  }: { pair: SyncPair | null; onClose: () => void; onSaved: () => void } = $props();

  const isEdit = pair !== null;

  let name = $state(pair?.name ?? "");
  let source = $state(pair?.source ?? "");
  let destination = $state(pair?.destination ?? "");
  let enabled = $state(pair?.enabled ?? true);
  let notifyPc = $state(pair?.notifyPc ?? true);
  let notifyApp = $state(pair?.notifyApp ?? true);
  let ignoreText = $state((pair?.ignorePatterns ?? []).join("\n"));
  let intervalSec = $state<number | null>(pair?.intervalSecOverride ?? null);
  let saving = $state(false);

  const globalLabel = $derived(
    store.settings ? formatInterval(store.settings.intervalSec) : "15 min",
  );

  function normalize(p: string): string {
    return p.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
  }
  const overlap = $derived.by(() => {
    const a = normalize(source);
    const b = normalize(destination);
    if (!a || !b) return false;
    return a === b || a.startsWith(b + "/") || b.startsWith(a + "/");
  });

  const canSave = $derived(source.trim() !== "" && destination.trim() !== "" && !overlap && !saving);

  async function pick(which: "source" | "destination") {
    try {
      const dir = await api.pickFolder();
      if (dir) {
        if (which === "source") source = dir;
        else destination = dir;
      }
    } catch (err) {
      store.toast("error", String(err));
    }
  }

  async function save() {
    if (!canSave) return;
    saving = true;
    const ignorePatterns = ignoreText
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    const intervalSecOverride = intervalSec;
    try {
      if (isEdit && pair) {
        await api.updatePair({
          ...pair,
          name: name.trim() || source,
          source,
          destination,
          enabled,
          intervalSecOverride,
          notifyPc,
          notifyApp,
          ignorePatterns,
        });
      } else {
        await api.addPair({
          name: name.trim() || source,
          source,
          destination,
          enabled,
          intervalSecOverride,
          notifyPc,
          notifyApp,
          ignorePatterns,
        });
      }
      onSaved();
      onClose();
    } catch (err) {
      store.toast("error", String(err));
      saving = false;
    }
  }
</script>

<div class="modal-backdrop" onclick={onClose} role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
    <h1>{isEdit ? "Modifier la paire" : "Nouvelle paire"}</h1>

    <div class="warn">
      ⚠ Miroir unidirectionnel : la <strong>destination</strong> sera rendue
      <strong>identique à la source</strong> (les fichiers en trop côté destination sont supprimés).
    </div>

    <div class="form">
      <div class="field">
        <span class="label">Nom</span>
        <input class="input" bind:value={name} placeholder="ex. Documents → NAS" />
      </div>

      <div class="field">
        <span class="label">Dossier source (autorité)</span>
        <div class="row">
          <input class="input" bind:value={source} placeholder="Choisir un dossier…" />
          <button class="btn" onclick={() => pick("source")}>Parcourir</button>
        </div>
      </div>

      <div class="field">
        <span class="label">Dossier destination (sera reflété)</span>
        <div class="row">
          <input class="input" bind:value={destination} placeholder="Choisir un dossier…" />
          <button class="btn" onclick={() => pick("destination")}>Parcourir</button>
        </div>
      </div>

      {#if overlap}
        <div class="err">
          Source et destination imbriquées (l'une contient l'autre) — interdit.
        </div>
      {/if}

      <div class="field">
        <span class="label">Intervalle de synchro auto</span>
        <IntervalPicker bind:seconds={intervalSec} allowDefault={true} />
        <span class="muted" style="font-size:12px">
          {intervalSec == null ? `Hérite du défaut global (${globalLabel})` : "Rythme propre à cette paire"}
          · minimum 5 s
        </span>
      </div>

      <div class="field">
        <span class="label">Exclusions (un motif glob par ligne)</span>
        <textarea class="input" rows="3" bind:value={ignoreText} placeholder="**/*.tmp&#10;**/node_modules/**"></textarea>
      </div>

      <div class="row toggles">
        <label class="tg">
          <Switch bind:checked={enabled} />
          <span>Activée</span>
        </label>
        <label class="tg">
          <Switch bind:checked={notifyPc} />
          <span>Notif PC</span>
        </label>
        <label class="tg">
          <Switch bind:checked={notifyApp} />
          <span>Notif app</span>
        </label>
      </div>
      <p class="muted" style="font-size:12px">
        Les notifs ne s'affichent que si le type est aussi activé dans Réglages (interrupteur
        maître).
      </p>
    </div>

    <div class="row actions">
      <div class="spacer"></div>
      <button class="btn" onclick={onClose}>Annuler</button>
      <button class="btn btn-primary" onclick={save} disabled={!canSave}>
        {saving ? "Enregistrement…" : "Enregistrer"}
      </button>
    </div>
  </div>
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    margin: var(--s4) 0;
  }
  .actions {
    margin-top: var(--s4);
  }
  .toggles {
    gap: var(--s5);
  }
  .tg {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .warn {
    margin-top: var(--s3);
    padding: var(--s3);
    border-radius: var(--r-control);
    background: color-mix(in srgb, var(--orange) 14%, transparent);
    font-size: 13px;
  }
  .err {
    padding: var(--s2) var(--s3);
    border-radius: var(--r-control);
    background: color-mix(in srgb, var(--red) 14%, transparent);
    color: var(--red);
    font-size: 13px;
  }
</style>
