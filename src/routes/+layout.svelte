<script lang="ts">
  import "../styles/global.css";
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { store } from "$lib/store.svelte";
  import { api } from "$lib/ipc";
  import ConfirmModal from "$lib/components/ConfirmModal.svelte";
  import Loader from "$lib/components/Loader.svelte";
  import { fly } from "svelte/transition";

  let { children } = $props();

  // Thème : « Système » suit l'OS (matchMedia), « Clair »/« Sombre » forcés via data-theme.
  $effect(() => {
    const t = store.settings?.theme ?? "system";
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const resolve = () => {
      const dark = t === "dark" || (t === "system" && mql.matches);
      document.documentElement.dataset.theme = dark ? "dark" : "light";
    };
    resolve();
    if (t === "system") {
      mql.addEventListener("change", resolve);
      return () => mql.removeEventListener("change", resolve);
    }
  });

  async function toggleFullscreen() {
    const w = getCurrentWindow();
    await w.setFullscreen(!(await w.isFullscreen()));
  }

  onMount(() => {
    store.refresh();
    store.loadCompressionMethods();
    let cleanup: (() => void) | undefined;
    store.initListeners().then((fn) => (cleanup = fn));

    const onKey = async (e: KeyboardEvent) => {
      if (e.key === "F11") {
        e.preventDefault();
        await toggleFullscreen();
      } else if (e.key === "Escape") {
        const w = getCurrentWindow();
        if (await w.isFullscreen()) await w.setFullscreen(false);
      }
    };
    window.addEventListener("keydown", onKey);

    return () => {
      cleanup?.();
      window.removeEventListener("keydown", onKey);
    };
  });

  const nav = [
    { href: "/", label: "Tableau de bord", icon: "▦" },
    { href: "/exclusions", label: "Exclusions", icon: "⊘" },
    { href: "/compression", label: "Compression", icon: "▤" },
    { href: "/settings", label: "Réglages", icon: "⚙" },
    { href: "/logs", label: "Journal", icon: "≣" },
  ];
</script>

{#if !store.loaded}
  <Loader />
{/if}

<div class="app">
  <aside class="side">
    <div class="brand">
      <span class="logo">◑</span>
      Reflet
    </div>
    <nav>
      {#each nav as n}
        <a href={n.href} class="navitem" class:active={$page.url.pathname === n.href}>
          <span class="ic">{n.icon}</span>{n.label}
        </a>
      {/each}
    </nav>
    <div class="spacer"></div>
    <div class="side-foot">
      <button class="btn" onclick={() => api.quitApp()}>Quitter</button>
    </div>
  </aside>

  <main class="main">
    {#key $page.url.pathname}
      <div class="route" in:fly={{ y: 8, duration: 180 }}>
        {@render children()}
      </div>
    {/key}
  </main>
</div>

<div class="toasts">
  {#each store.toasts as t (t.id)}
    <button class="toast {t.kind}" onclick={() => store.dismiss(t.id)}>{t.message}</button>
  {/each}
</div>

<ConfirmModal />

<style>
  .app {
    display: grid;
    grid-template-columns: 216px 1fr;
    height: 100vh;
    overflow: hidden;
    background: var(--bg);
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s3) var(--s2);
    background: var(--bg-sunken);
    border-right: 1px solid var(--hairline);
    overflow: hidden;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    font-size: 16px;
    letter-spacing: -0.01em;
    padding: 6px 8px var(--s3);
  }
  .logo {
    color: var(--accent);
    font-size: 18px;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .navitem {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: 5px 9px;
    font-size: 13px;
    border-radius: var(--r-control);
    color: var(--text-2);
    font-weight: 500;
    transition: background 0.15s var(--ease), color 0.15s var(--ease);
  }
  .navitem:hover {
    background: var(--hover);
    color: var(--text);
  }
  .navitem.active {
    background: var(--hover);
    color: var(--text);
    font-weight: 600;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.07);
  }
  .navitem.active::after {
    content: "";
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%) scale(1);
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--text);
    animation: navDot 0.22s var(--ease);
  }
  @keyframes navDot {
    from {
      opacity: 0;
      transform: translateY(-50%) scale(0);
    }
  }
  .ic {
    width: 18px;
    text-align: center;
    opacity: 0.9;
  }
  .side-foot {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s2);
  }
  .main {
    overflow: hidden;
    padding: 0;
    min-width: 0;
    min-height: 0;
  }
  .route {
    height: 100%;
  }

  .toasts {
    position: fixed;
    bottom: var(--s4);
    right: var(--s4);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    z-index: 100;
  }
  .toast {
    text-align: left;
    border: none;
    cursor: pointer;
    color: #fff;
    padding: 10px 14px;
    border-radius: var(--r-control);
    box-shadow: var(--shadow-modal);
    font-size: 13px;
    max-width: 360px;
    animation: rise 0.25s var(--ease);
  }
  .toast.info {
    background: #3a3a3c;
  }
  .toast.success {
    background: var(--green);
  }
  .toast.error {
    background: var(--red);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
