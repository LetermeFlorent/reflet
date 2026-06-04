<script lang="ts">
  import type { CompressionConfig, CompressionMethod, SyncPair } from "$lib/types";
  import { api } from "$lib/ipc";
  import { store } from "$lib/store.svelte";
  import Switch from "./Switch.svelte";
  import Select from "./Select.svelte";
  import IntervalPicker from "./IntervalPicker.svelte";
  import ExclusionsManager from "./ExclusionsManager.svelte";
  import { formatInterval } from "$lib/format";
  import { onMount } from "svelte";

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
  let scheduleInput = $state("");
  let minFileSize = $state(pair?.minFileSize ?? 0);
  let maxFileSize = $state(pair?.maxFileSize ?? 0);
  let cardColor = $state(pair?.color ?? "");
  let saving = $state(false);

  // Compression
  let compMethod = $state(pair?.compression?.method ?? "off");
  let compLevel = $state(pair?.compression?.level ?? 0);
  let compPassword = $state(pair?.compression?.password ?? "");
  let availableMethods = $state<CompressionMethod[]>([]);

  async function loadMethods() {
    try {
      availableMethods = await api.detectCompressionMethods();
    } catch {
      // silencieux
    }
  }
  onMount(loadMethods);

  const TIME_RE = /^([01]\d|2[0-3]):[0-5]\d$/;
  function addScheduleTime() {
    const t = scheduleInput.trim();
    if (TIME_RE.test(t) && !scheduleTimes.includes(t)) {
      scheduleTimes = [...scheduleTimes, t];
    }
    scheduleInput = "";
  }
  function removeScheduleTime(i: number) {
    scheduleTimes = scheduleTimes.filter((_, j) => j !== i);
  }
  const scheduleInputValid = $derived(TIME_RE.test(scheduleInput.trim()));

  const selectedMethodMeta = $derived(availableMethods.find(m => m.id === compMethod));
  const compLevelMax = $derived(selectedMethodMeta?.maxLevel ?? 0);
  const compLevelDefault = $derived(selectedMethodMeta?.defaultLevel ?? 0);
  const compSupportsPassword = $derived(selectedMethodMeta?.supportsPassword ?? false);
  const availMethods = $derived(availableMethods.filter(m => m.available));
  const unavailMethods = $derived(availableMethods.filter(m => !m.available));
  const compOptions = $derived([
    { value: "off", label: "Aucune (copie brute)" },
    ...availableMethods.map(m => ({
      value: m.id,
      label: `${m.name} (${m.extension})${m.available ? '' : ' — non installé'}`,
      disabled: !m.available,
    })),
  ]);

  // On method change, ensure level is valid
  let prevCompMethod = $state(compMethod);
  $effect(() => {
    if (compMethod !== prevCompMethod) {
      prevCompMethod = compMethod;
      if (compMethod !== "off" && compLevelDefault > 0 && (compLevel === 0 || compLevel > compLevelMax)) {
        compLevel = compLevelDefault;
      }
    }
  });

  const COLORS = [
    { id: "", name: "Aucune" },
    { id: "bleu", name: "Bleu" },
    { id: "vert", name: "Vert" },
    { id: "orange", name: "Orange" },
    { id: "rose", name: "Rose" },
    { id: "violet", name: "Violet" },
    { id: "teal", name: "Teal" },
    { id: "jaune", name: "Jaune" },
    { id: "rouge", name: "Rouge" },
  ];

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
    const times = $state.snapshot(scheduleTimes).filter((t) => TIME_RE.test(t));
    const minSize = Math.max(0, Math.trunc(minFileSize || 0));
    const maxSize = Math.max(0, Math.trunc(maxFileSize || 0));
    const compression: CompressionConfig = {
      method: compMethod,
      level: compMethod === "off" ? 0 : (compLevel || compLevelDefault),
      password: compSupportsPassword && compPassword ? compPassword : null,
    };
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
          compression,
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
          compression,
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
        <span class="label">Planification avancée (horaires spécifiques)</span>
        <div class="row">
          <input class="input" style="width:100px" placeholder="HH:MM" bind:value={scheduleInput} onkeydown={(e) => e.key === 'Enter' && addScheduleTime()} />
          <button class="btn btn-sm" onclick={addScheduleTime} disabled={!scheduleInputValid}>Ajouter</button>
        </div>
        {#if scheduleTimes.length > 0}
          <div class="chips" style="margin-top:6px">
            {#each scheduleTimes as t, i}
              <span class="chip">
                {t}
                <button class="chip-x" onclick={() => removeScheduleTime(i)}>×</button>
              </span>
            {/each}
          </div>
        {/if}
        <span class="muted" style="font-size:12px">Laisse vide pour utiliser l'intervalle classique.</span>
      </div>

      <div class="field">
        <span class="label">Exclusions propres à cette paire (en plus des exclusions globales)</span>
        <ExclusionsManager
          patterns={ignoreList}
          onAdd={excAdd}
          onReplace={excReplace}
          onRemove={excRemove}
        />
      </div>

      <div class="field">
        <span class="label">Compression</span>
        <Select bind:value={compMethod} options={compOptions} />

        {#if compMethod !== "off" && selectedMethodMeta}
          {#if !selectedMethodMeta.available}
            <div class="err" style="margin-top:4px;font-size:12px">
              {selectedMethodMeta.name} n'est pas installé.
              <a href={selectedMethodMeta.downloadUrl} target="_blank" rel="noopener noreferrer">Télécharger {selectedMethodMeta.name}</a>
            </div>
          {:else}
            <div style="margin-bottom:6px">
              <span class="label" style="font-size:12px">Niveau de compression ({compLevel}/{selectedMethodMeta.maxLevel})</span>
              <input class="input" type="range" min="1" max={selectedMethodMeta.maxLevel} bind:value={compLevel} style="width:100%" />
            </div>
            {#if compSupportsPassword}
              <div>
                <span class="label" style="font-size:12px">Mot de passe (optionnel)</span>
                <input class="input" type="password" bind:value={compPassword} placeholder="Laisser vide = pas de mot de passe" />
              </div>
            {/if}
          {/if}
        {/if}
        {#if availableMethods.length === 0}
          <div class="muted" style="font-size:12px">Chargement des méthodes disponibles…</div>
        {/if}
      </div>

      <div class="field">
        <span class="label">Couleur de la carte</span>
        <div class="color-picker">
          {#each COLORS as c}
            <button
              class="color-oval"
              class:selected={cardColor === c.id}
              style={c.id ? `background:var(--color-${c.id}-bg)` : ''}
              title={c.name}
              onclick={() => (cardColor = c.id)}
            >
              {#if !c.id}
                <span class="color-none">∅</span>
              {/if}
            </button>
          {/each}
        </div>
      </div>

      <div class="row toggles">
        <label class="tg">
          <Switch bind:checked={enabled} />
          <span>Activée</span>
        </label>
        <label class="tg">
          <Switch bind:checked={watchRealtime} />
          <span>Temps réel</span>
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
        <strong>Temps réel</strong> surveille le dossier source et déclenche une synchro dès qu'un changement est détecté (avec un délai de 3 secondes).<br />
        Les notifs ne s'affichent que si le type est aussi activé dans Réglages (interrupteur
        maître).
      </p>
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
  .fields-row {
    gap: var(--s4);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--bg-2);
    font-size: 12px;
  }
  .chip-x {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    color: var(--text-2);
    padding: 0 2px;
  }
  .chip-x:hover {
    color: var(--red);
  }
  .color-picker {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .color-oval {
    width: 32px;
    height: 22px;
    border-radius: 11px;
    border: 2px solid var(--border);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-elev);
    transition: border-color 0.15s, transform 0.1s;
  }
  .color-oval:hover {
    transform: scale(1.1);
  }
  .color-oval.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .color-none {
    font-size: 12px;
    color: var(--text-2);
    line-height: 1;
  }

</style>
