<script lang="ts">
  let {
    checked = $bindable(false),
    disabled = false,
    onchange,
  }: { checked?: boolean; disabled?: boolean; onchange?: (checked: boolean) => void } = $props();

  function handle(e: Event) {
    onchange?.((e.target as HTMLInputElement).checked);
  }
</script>

<label class="switch">
  <input type="checkbox" bind:checked {disabled} onchange={handle} />
  <span class="slider"></span>
</label>
