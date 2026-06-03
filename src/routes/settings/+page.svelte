<script lang="ts">
  import { store } from "$lib/store.svelte";
  import { api } from "$lib/ipc";
  import type { Settings } from "$lib/types";
  import Switch from "$lib/components/Switch.svelte";
  import IntervalPicker from "$lib/components/IntervalPicker.svelte";
  import Select from "$lib/components/Select.svelte";
  import { formatInterval } from "$lib/format";

  const verifyOptions = [
    { value: "off", label: "Taille + date (rapide)" },
    { value: "blake3", label: "+ empreinte blake3" },
  ];
  const deleteOptions = [
    { value: "trash", label: "Corbeille (recommandé)" },
    { value: "permanent", label: "Permanent" },
  ];

  let local = $state<Settings | null>(null);
  let saving = $state(false);

  // Initialise une copie éditable dès que les réglages sont chargés.
  $effect(() => {
    if (store.settings && !local) {
      local = JSON.parse(JSON.stringify(store.settings)) as Settings;
    }
  });

  let ignoreText = $state("");
  let ignoreInit = false;
  $effect(() => {
    if (local && !ignoreInit) {
      ignoreText = local.ignorePatterns.join("\n");
      ignoreInit = true;
    }
  });

  async function save() {
    if (!local) return;
    saving = true;
    local.ignorePatterns = ignoreText
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l.length > 0);
    try {
      await api.updateSettings($state.snapshot(local));
      store.toast("success", "Réglages enregistrés");
      await store.refresh();
    } catch (e) {
      store.toast("error", String(e));
    } finally {
      saving = false;
    }
  }
</script>

<div class="page-scroll">
<header class="page-head">
  <h1>Réglages</h1>
  <div class="spacer"></div>
  <button class="btn btn-primary" onclick={save} disabled={saving || !local}>
    {saving ? "Enregistrement…" : "Enregistrer"}
  </button>
</header>

{#if !local}
  <p class="muted">Chargement…</p>
{:else}
  <div class="sections">
    <section class="card">
      <h2>Synchronisation</h2>

      <div class="setting">
        <div class="info">
          <div class="name">Intervalle par défaut</div>
          <div class="muted">
            Utilisé par les paires sans intervalle propre : {formatInterval(local.intervalSec)}.
            (Chaque paire peut définir le sien.)
          </div>
        </div>
        <IntervalPicker bind:seconds={local.intervalSec} />
      </div>

      <div class="setting">
        <div class="info">
          <div class="name">Planificateur actif</div>
          <div class="muted">Lance la synchro auto au minuteur. Désactivé = manuel uniquement.</div>
        </div>
        <Switch bind:checked={local.schedulerRunning} />
      </div>

      <div class="setting">
        <div class="info">
          <div class="name">Vérification du contenu</div>
          <div class="muted">Compare aussi par empreinte (blake3) — plus sûr, plus lent.</div>
        </div>
        <Select bind:value={local.verifyByContent} options={verifyOptions} width="200px" />
      </div>
    </section>

    <section class="card">
      <h2>Sécurité des suppressions</h2>

      <div class="setting">
        <div class="info">
          <div class="name">Méthode de suppression</div>
          <div class="muted">
            Corbeille = récupérable. Permanent = irréversible.
          </div>
        </div>
        <Select bind:value={local.deleteBehavior} options={deleteOptions} width="200px" />
      </div>

      {#if local.deleteBehavior === "permanent"}
        <div class="danger-note">
          ⚠ Mode permanent : les fichiers supprimés côté destination ne seront pas récupérables.
        </div>
      {/if}

      <div class="setting">
        <div class="info">
          <div class="name">Seuil de sécurité anti-wipe</div>
          <div class="muted">
            Si plus de ce % de la destination devait être supprimé (ex. source vide/non montée), les
            suppressions sont retenues. 100 = désactivé.
          </div>
        </div>
        <input class="input num" type="number" min="1" max="100" bind:value={local.deleteSafetyThresholdPct} />
      </div>

      <div class="setting">
        <div class="info">
          <div class="name">Tolérance horodatage (s)</div>
          <div class="muted">Écart de date toléré (FAT/réseau). Défaut 2.</div>
        </div>
        <input class="input num" type="number" min="0" max="60" bind:value={local.mtimeToleranceSec} />
      </div>
    </section>

    <section class="card">
      <h2>Notifications</h2>

      <div class="setting">
        <div class="info">
          <div class="name">Notifications système (PC)</div>
          <div class="muted">
            Interrupteur maître des notifs Windows/Linux. Chaque paire peut être coupée
            individuellement.
          </div>
        </div>
        <Switch bind:checked={local.notifyPc} />
      </div>

      <div class="setting">
        <div class="info">
          <div class="name">Notifications dans l'app (toasts)</div>
          <div class="muted">Interrupteur maître des bulles affichées dans la fenêtre.</div>
        </div>
        <Switch bind:checked={local.notifyApp} />
      </div>
    </section>

    <section class="card">
      <h2>Démarrage</h2>

      <div class="setting">
        <div class="info">
          <div class="name">Lancer au démarrage de l'ordinateur</div>
          <div class="muted">Reflet démarre avec la session et reste dans la barre système.</div>
        </div>
        <Switch bind:checked={local.autostart} />
      </div>

      <div class="setting">
        <div class="info">
          <div class="name">Démarrer minimisé</div>
          <div class="muted">Au lancement auto, masquer la fenêtre (tray seulement).</div>
        </div>
        <Switch bind:checked={local.startMinimized} />
      </div>
    </section>

    <section class="card">
      <h2>Exclusions globales</h2>
      <p class="muted" style="margin-bottom:var(--s2)">
        Motifs glob (un par ligne) appliqués à toutes les paires, en plus des exclusions propres à
        chaque paire.
      </p>
      <textarea class="input" rows="5" bind:value={ignoreText}></textarea>
    </section>
  </div>
{/if}
</div>

<style>
  .page-head {
    display: flex;
    align-items: center;
    margin-bottom: var(--s5);
  }
  .sections {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    width: 100%;
  }
  section.card {
    padding: var(--s4) var(--s5);
  }
  section h2 {
    margin-bottom: var(--s2);
  }
  .setting {
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: var(--s3) 0;
    border-top: 1px solid var(--hairline);
  }
  .setting:first-of-type {
    border-top: none;
  }
  .info {
    flex: 1;
    min-width: 0;
  }
  .name {
    font-weight: 600;
  }
  .info .muted {
    font-size: 12px;
  }
  .presets {
    display: flex;
    gap: 4px;
    align-items: center;
    flex-wrap: wrap;
  }
  .num {
    width: 72px;
  }
  .sel {
    width: 200px;
  }
  .danger-note {
    padding: var(--s2) var(--s3);
    border-radius: var(--r-control);
    background: color-mix(in srgb, var(--red) 14%, transparent);
    color: var(--red);
    font-size: 13px;
    margin-top: var(--s2);
  }
</style>
