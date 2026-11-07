<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Home, type Opened } from "./lib/api";
  import { applyLook } from "./lib/look";
  import Start from "./lib/Start.svelte";
  import Writer from "./lib/Writer.svelte";

  let home = $state<Home | null>(null);
  let opened = $state<Opened | null>(null);
  let error = $state("");

  const tr = $derived((home?.settings.lang ?? "tr") === "tr");

  async function reload() {
    home = await api.home();
    applyLook(home.settings);
  }

  async function open(path: string) {
    try {
      opened = await api.open(path);
      // The play's language may differ from the home screen's.
      if (home) home.settings = await api.settings();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(async () => {
    await reload();
    const first = await api.startup();
    if (first) open(first);
  });
</script>

{#if opened && home}
  {#key opened.path}
    <Writer
      {opened}
      settings={home.settings}
      {tr}
      close={async (next) => {
        opened = null;
        await reload();
        if (next) await open(next);
      }}
    />
  {/key}
{:else if home}
  <Start {home} {tr} {open} {reload} />
{/if}

{#if error}
  <div class="error" role="alert">
    {error}
    <button onclick={() => (error = "")}>✕</button>
  </div>
{/if}

<style>
  .error {
    position: fixed;
    bottom: 16px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--claret);
    color: #f6efe4;
    padding: 8px 14px;
    border-radius: 6px;
    z-index: 50;
  }
  .error button {
    background: none;
    border: 0;
    color: inherit;
    margin-left: 10px;
    cursor: pointer;
  }
</style>
