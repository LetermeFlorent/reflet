<script lang="ts">
  import type { CompressionConfig, SyncPair } from "$lib/types";
  import { api } from "$lib/ipc";
  import { store } from "$lib/store.svelte";
  import Switch from "./Switch.svelte";
  import IntervalPicker from "./IntervalPicker.svelte";
  import ExclusionsManager from "./ExclusionsManager.svelte";
  import ScheduleTimesEditor from "./ScheduleTimesEditor.svelte";
  import CompressionSettings from "./CompressionSettings.svelte";
  import ColorPicker from "./ColorPicker.svelte";
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
  let ignoreList = $state<string[]>([...(pair?.ignorePatterns ?? [])]);
  let intervalSec = $state<number | null>(pair?.intervalSecOverride ?? null);
  let watchRealtime = $state(pair?.watchRealtime ?? false);
  let scheduleTimes = $state<string[]>([...(pair?.scheduleTimes ?? [])]);
  let minFileSize = $state(pair?.minFileSize ?? 0);
  let maxFileSize = $state(pair?.maxFileSize ?? 0);
  let cardColor = $state(pair?.color ?? "");
  let backupMode = $state(pair?.backupMode ?? false);
  let saving = $state(false);

  let compression = $state<CompressionConfig>({
    method: pair?.compression?.method ?? store.settings?.defaultCompressionMethod ?? "off",
    level: pair?.compression?.level ?? store.settings?.defaultCompressionLevel ?? 0,
    password: pair?.compression?.password ?? null,
    archiveName: pair?.compression?.archiveName ?? "",
  });

  function excAdd(ps: string[]) {
    for (const p of ps) if (!ignoreList.includes(p)) ignoreList.push(p);
  }
  function excReplace(i: number, ps: string[]) {
    ignoreList.splice(i, 1, ...ps);
  }
  function excRemove(i: number) {
    ignoreList.splice(i, 1);
  }

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
    const ignorePatterns = $state.snapshot(ignoreList);
    const intervalSecOverride = intervalSec;
    const times = $state.snapshot(scheduleTimes);
    const minSize = Math.max(0, Math.trunc(minFileSize || 0));
    const maxSize = Math.max(0, Math.trunc(maxFileSize || 0));
    const compressionCfg = $state.snapshot(compression);
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
          watchRealtime,
          scheduleTimes: times,
          minFileSize: minSize,
          maxFileSize: maxSize,
          color: cardColor,
          compression: compressionCfg,
          backupMode,
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
          watchRealtime,
          scheduleTimes: times,
          minFileSize: minSize,
          maxFileSize: maxSize,
          color: cardColor,
          compression: compressionCfg,
          backupMode,
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
      <section class="sec">
        <h2 class="sec-t">Général</h2>
        <div class="sec-body">
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
          <label class="tg">
            <Switch bind:checked={enabled} />
            <span>Paire activée</span>
          </label>
        </div>
      </section>

      <section class="sec">
        <h2 class="sec-t">Synchronisation automatique</h2>
        <div class="sec-body">
          <div class="field">
            <span class="label">Intervalle</span>
            <IntervalPicker bind:seconds={intervalSec} allowDefault={true} />
            <span class="hint">
              {intervalSec == null ? `Hérite du défaut global (${globalLabel})` : "Rythme propre à cette paire"}
              · minimum 5 s
            </span>
          </div>
          <div class="field">
            <span class="label">Horaires spécifiques</span>
            <ScheduleTimesEditor bind:times={scheduleTimes} />
            <span class="hint">Laisse vide pour utiliser l'intervalle ci-dessus.</span>
          </div>
          <div class="field">
            <label class="tg">
              <Switch bind:checked={watchRealtime} />
              <span>Temps réel</span>
            </label>
            <span class="hint">
              Surveille la source et déclenche une synchro dès qu'un changement est détecté (délai 3 s).
            </span>
          </div>
        </div>
      </section>

      <section class="sec">
        <h2 class="sec-t">Filtres</h2>
        <div class="sec-body">
          <div class="row fields-row">
            <div class="field" style="flex:1">
              <span class="label">Taille min. (octets)</span>
              <input class="input" type="number" min="0" bind:value={minFileSize} placeholder="0 = aucun filtre" />
            </div>
            <div class="field" style="flex:1">
              <span class="label">Taille max. (octets)</span>
              <input class="input" type="number" min="0" bind:value={maxFileSize} placeholder="0 = aucun filtre" />
            </div>
          </div>
          <div class="field">
            <span class="label">Exclusions propres à cette paire</span>
            <ExclusionsManager
              patterns={ignoreList}
              onAdd={excAdd}
              onReplace={excReplace}
              onRemove={excRemove}
            />
            <span class="hint">En plus des exclusions globales (Réglages).</span>
          </div>
        </div>
      </section>

      <section class="sec">
        <h2 class="sec-t">Destination</h2>
        <div class="sec-body">
          <div class="field">
            <label class="tg">
              <Switch bind:checked={backupMode} />
              <span>Mode sauvegarde (copies horodatées)</span>
            </label>
            <span class="hint">
              Au lieu de refléter la source, crée à chaque synchro un dossier
              <code>backup-AAAA-MM-JJ_HH-MM-SS</code> avec une copie complète. Aucune suppression.
            </span>
          </div>
          <div class="field">
            <span class="label">Compression</span>
            <CompressionSettings bind:config={compression} />
          </div>
        </div>
      </section>

      <section class="sec">
        <h2 class="sec-t">Apparence &amp; notifications</h2>
        <div class="sec-body">
          <div class="field">
            <span class="label">Couleur de la carte</span>
            <ColorPicker bind:value={cardColor} />
          </div>
          <div class="row toggles">
            <label class="tg">
              <Switch bind:checked={notifyPc} />
              <span>Notif PC</span>
            </label>
            <label class="tg">
              <Switch bind:checked={notifyApp} />
              <span>Notif app</span>
            </label>
          </div>
          <span class="hint">
            Les notifs ne s'affichent que si le canal est aussi activé dans Réglages (interrupteur maître).
          </span>
        </div>
      </section>
    </div>

    <div class="row actions">
      <div class="spacer"></div>
      <button class="btn" onclick={onClose}>Annuler</button>
      <button class="btn" onclick={save} disabled={!canSave}>
        {saving ? "Enregistrement…" : "Enregistrer"}
      </button>
    </div>
  </div>
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    margin: var(--s3) 0 var(--s2);
  }
  .sec {
    padding: var(--s4) 0;
    border-top: 1px solid var(--hairline);
  }
  .sec:first-child {
    border-top: none;
    padding-top: var(--s2);
  }
  .sec-t {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-2);
    margin-bottom: var(--s3);
  }
  .sec-body {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }
  .hint {
    font-size: 12px;
    color: var(--text-2);
    line-height: 1.45;
  }
  .actions {
    margin-top: var(--s4);
    padding-top: var(--s3);
    border-top: 1px solid var(--hairline);
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
  .fields-row {
    gap: var(--s4);
  }
</style>
