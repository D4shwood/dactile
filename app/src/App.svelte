<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Home, type Opened } from "./lib/api";
  import { applyLook } from "./lib/look";
  import Start from "./lib/Start.svelte";
  import Writer from "./lib/Writer.svelte";
  import Ask from "./lib/Ask.svelte";
  import { isOnSite, SITE_OUT, siteUpdate, siteUpdateInstall, useSite } from "./lib/site/desk";
  import { notify } from "./lib/asking.svelte";

  let home = $state<Home | null>(null);
  let opened = $state<Opened | null>(null);
  let error = $state("");

  const tr = $derived((home?.settings.lang ?? "tr") === "tr");

  async function reload() {
    home = await api.home();
    applyLook(home.settings);
    if (home.notice) error = home.notice;
  }

  async function open(path: string) {
    try {
      opened = await api.open(path);
      // The play's language may differ from the home screen's.
      if (home) home.settings = await api.settings();
    } catch (e) {
      error = String(e);
      // A site's play that didn't open: home stays this computer's.
      if (!opened && isOnSite()) await useSite(false);
    }
  }

  onMount(async () => {
    // A linked site let this computer go while one of its plays was open.
    window.addEventListener(SITE_OUT, async () => {
      if (!isOnSite()) return;
      opened = null;
      await useSite(false);
      await reload();
      error = tr
        ? "Site bu bilgisayarı artık tanımıyor: Sitede bölümünden yeniden bağlan."
        : "The site no longer knows this computer: link it again from On the site.";
    });
    await reload();
    // A linked site's desktop, newer than this one: offered once, quietly.
    // (In the browser, and with no site linked, nothing.)
    void siteUpdate()
      .then((v) => {
        if (!v) return;
        notify({
          text: tr ? "Sitenin Dactile'ı daha yeni: güncellensin mi?" : "The site's Dactile is newer: update?",
          action: {
            label: tr ? "Güncelle" : "Update",
            run: () =>
              void siteUpdateInstall().catch(
                (e) => (error = (tr ? "Güncelleme olmadı: " : "The update didn't go through: ") + String(e)),
              ),
          },
          ms: 60_000,
        });
      })
      .catch(() => {});
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
        // Back home: the commands are this computer's again. The next play
        // of a site's play (a draft of it) is the site's too.
        if (!next) await useSite(false);
        await reload();
        if (next) await open(next);
      }}
    />
  {/key}
{:else if home}
  <Start {home} {tr} {open} {reload} />
{/if}

<Ask />

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
    max-width: 80vw;
    overflow-wrap: anywhere;
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
