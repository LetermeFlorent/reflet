<script lang="ts">
  import { store } from "$lib/store.svelte";
  import { api } from "$lib/ipc";
  import type { Settings } from "$lib/types";
  import Switch from "$lib/components/Switch.svelte";
  import IntervalPicker from "$lib/components/IntervalPicker.svelte";
  import Select from "$lib/components/Select.svelte";
  import { formatInterval } from "$lib/format";
  import { onMount, onDestroy } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";

  const verifyOptions = [
    { value: "off", label: "Taille + date (rapide)" },
    { value: "blake3", label: "+ empreinte blake3" },
  ];
  const deleteOptions = [
    { value: "trash", label: "Corbeille (recommandé)" },
    { value: "permanent", label: "Permanent" },
  ];
  const themeOptions = [
    { value: "system", label: "Système (suit l'OS)" },
    { value: "light", label: "Clair" },
    { value: "dark", label: "Sombre" },
  ];

  let appVersion = $state("");
  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch {
      appVersion = "";
    }
  });

  let local = $state<Settings | null>(null);
  let pending = $state(false);
  let lastSent = "";
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (store.settings && !local) {
      local = JSON.parse(JSON.stringify(store.settings)) as Settings;
    }
  });

  $effect(() => {
    if (!local) return;
    const snap = JSON.stringify(local);
    if (lastSent === "") {
      lastSent = snap;
      return;
    }
    if (snap === lastSent) return;
    lastSent = snap;
    pending = true;
    clearTimeout(timer);
    timer = setTimeout(async () => {
      try {
        await api.updateSettings(JSON.parse(snap) as Settings);
        await store.refresh();
      } catch (e) {
        store.toast("error", String(e));
      } finally {
        pending = false;
      }
    }, 400);
  });

  onDestroy(() => {
    clearTimeout(timer);
    // Sauvegarde en attente (réglage modifié <400 ms avant de quitter la page) : on la
    // flush au lieu de la perdre. clearTimeout au-dessus évite un double-envoi.
    if (pending && local) {
      api.updateSettings(JSON.parse(JSON.stringify(local)) as Settings).catch(() => {});
    }
  });
</script>

<div class="page-scroll">
<header class="page-head">
  <h1>Réglages</h1>
  <div class="spacer"></div>
  <span class="savestate muted">{pending ? "Enregistrement…" : "✓ Enregistré automatiquement"}</span>
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
      <h2>Affichage</h2>

      <div class="setting">
        <div class="info">
          <div class="name">Cartes compactes</div>
          <div class="muted">
            La ligne du bas n'affiche que le statut court (« À jour » / « Jamais synchronisé ») +
            le temps avant la prochaine synchro, au lieu du détail copiés/màj/suppr.
          </div>
        </div>
        <Switch bind:checked={local.compactCards} />
      </div>

      <div class="setting">
        <div class="info">
          <div class="name">Thème</div>
          <div class="muted">« Système » suit le thème de l'OS, ou force « Clair » / « Sombre ».</div>
        </div>
        <Select bind:value={local.theme} options={themeOptions} width="200px" />
      </div>
    </section>

    <section class="card">
      <h2>À propos</h2>
      <div class="setting">
        <div class="info">
          <div class="name">Version</div>
          <div class="muted">Reflet — synchronisation miroir unidirectionnelle.</div>
        </div>
        <span class="muted">v{appVersion} <span class="oas">o.a.s</span></span>
      </div>
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
  .savestate {
    font-size: 13px;
  }
  .num {
    width: 72px;
  }
  .oas {
    font-size: 9px;
    opacity: 0.5;
    margin-left: 2px;
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
