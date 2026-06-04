<script lang="ts">
  import type { CompressionConfig, CompressionMethod } from "$lib/types";
  import { api } from "$lib/ipc";
  import Select from "./Select.svelte";
  import { onMount } from "svelte";

  let { config = $bindable<CompressionConfig>() }: { config: CompressionConfig } = $props();

  let methods = $state<CompressionMethod[]>([]);
  onMount(async () => {
    try {
      methods = await api.detectCompressionMethods();
    } catch {
      methods = [];
    }
    // Compat : ancienne valeur « zip » (ou méthode disparue) -> deflate.
    if (config.method !== "off" && !methods.some((m) => m.id === config.method)) {
      config.method = "deflate";
      if (config.level < 1) config.level = 6;
    }
  });

  const meta = $derived(methods.find((m) => m.id === config.method));
  const maxLevel = $derived(meta?.maxLevel ?? 0);
  const ext = $derived(meta?.extension ?? ".zip");
  const options = $derived([
    { value: "off", label: "Aucune (miroir fichier par fichier)" },
    ...methods.map((m) => ({
      value: m.id,
      label: `${m.name} — ${m.ratio}${m.available ? "" : " (à installer)"}`,
      disabled: !m.available,
    })),
  ]);

  let showPwd = $state(false);
  let prev = $state(config.method);
  $effect(() => {
    if (config.method !== prev) {
      prev = config.method;
      const m = methods.find((x) => x.id === config.method);
      config.level = m ? m.defaultLevel : 0;
      // Méthode sans chiffrement : on jette tout mot de passe résiduel.
      if (!m?.supportsPassword) config.password = null;
    }
  });
</script>

<Select bind:value={config.method} {options} />

{#if config.method !== "off"}
  <div class="note">
    Tout le dossier source est compressé dans <strong>une seule archive</strong>
    (<code>{config.archiveName.trim() || "nom-de-la-paire"}{ext}</code>) placée dans la destination,
    qui ne garde que cette archive.
    {#if meta?.builtin}
      {#if config.password}
        Archive chiffrée (AES-256) : reconstruite entièrement à chaque synchro.
      {:else}
        Synchros suivantes : seuls les fichiers modifiés sont recompressés (incrémental).
      {/if}
    {:else}
      Format externe : l'archive est reconstruite entièrement à chaque synchro.
    {/if}
  </div>
  <div class="fld">
    <span class="label">Nom de l'archive</span>
    <input class="input" bind:value={config.archiveName} placeholder="Vide = nom de la paire" />
  </div>
  {#if maxLevel > 0}
    <div class="lvl">
      <span class="label">Niveau de compression ({config.level}/{maxLevel}) — {meta?.ratio}</span>
      <input type="range" min="1" max={maxLevel} bind:value={config.level} />
    </div>
  {/if}
  {#if meta?.supportsPassword}
    <div class="fld">
      <span class="label">Mot de passe (chiffrement de l'archive)</span>
      <div class="pwd-row">
        <input
          class="input"
          type={showPwd ? "text" : "password"}
          autocomplete="new-password"
          placeholder="Vide = pas de chiffrement"
          value={config.password ?? ""}
          oninput={(e) => (config.password = e.currentTarget.value || null)}
        />
        <button type="button" class="btn btn-sm" onclick={() => (showPwd = !showPwd)}>
          {showPwd ? "Masquer" : "Afficher"}
        </button>
      </div>
      <span class="muted hint">
        {meta?.builtin ? "AES-256 (conteneur ZIP)." : "Chiffrement natif de l'outil (AES-256)."}
        Stocké en clair dans la configuration locale.
      </span>
    </div>
  {/if}
{/if}

<style>
  .note {
    margin: 4px 0 2px;
    padding: var(--s2) var(--s3);
    border-radius: var(--r-control);
    background: var(--bg-2);
    font-size: 12px;
    line-height: 1.5;
  }
  .fld {
    margin-bottom: 6px;
  }
  .lvl {
    margin-bottom: 6px;
  }
  .label {
    font-size: 12px;
  }
  .pwd-row {
    display: flex;
    gap: var(--s2);
    align-items: center;
  }
  .pwd-row .input {
    flex: 1;
  }
  .pwd-row .btn {
    flex: 0 0 auto;
  }
  .hint {
    display: block;
    font-size: 11px;
    margin-top: 4px;
  }
</style>
