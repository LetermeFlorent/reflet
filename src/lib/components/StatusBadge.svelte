<script lang="ts">
  let { status }: { status: string } = $props();
  const map: Record<string, { label: string; color: string }> = {
    idle: { label: "À jour", color: "var(--green)" },
    syncing: { label: "Synchro…", color: "var(--accent)" },
    error: { label: "Erreur", color: "var(--red)" },
    disabled: { label: "Désactivé", color: "var(--gray)" },
  };
  const info = $derived(map[status] ?? map.idle);
</script>

<span class="badge">
  <span class="dot" class:pulse={status === "syncing"} style="background:{info.color}"></span>
  {info.label}
</span>

<style>
  .pulse {
    animation: pulse 1s ease-in-out infinite;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }
</style>
