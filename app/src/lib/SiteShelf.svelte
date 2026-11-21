<script lang="ts">
  // The desktop's "On the site": linking this computer to a Dactile site,
  // and once linked, the plays and projects there, opened here as in a
  // browser (site/desk.ts sends the window's commands to the site while
  // one of them is open). The desktop only: in the browser the page is the
  // site.
  import { onMount } from "svelte";
  import type { Home, PlayCard } from "./api";
  import { ask, notify } from "./ask.svelte";
  import { API, SignedOut } from "./site/commands";
  import {
    site,
    siteConnect,
    siteDoor,
    siteForget,
    siteLink,
    siteUpdate,
    siteUpdateInstall,
    siteWait,
    useSite,
    type SiteDoor,
    type SiteLink,
  } from "./site/desk";

  let { tr, open }: { tr: boolean; open: (path: string) => void } = $props();
  const l = (t: string, e: string) => (tr ? t : e);

  let link = $state<SiteLink | null>(null);
  let door = $state<SiteDoor | null>(null);
  let home = $state<Pick<Home, "plays" | "projects"> | null>(null);
  /// What went wrong, said in place of the list.
  let trouble = $state("");
  let loading = $state(true);

  // Linking.
  let address = $state("");
  let code = $state("");
  let busy = $state(false);

  /// The site's version and API: a site that speaks another API is not
  /// opened from here until one of them is updated.
  const fits = $derived(door?.api === API);
  /// The site's own desktop, when it is newer than this one.
  let newer = $state<string | null>(null);
  let updating = $state(false);
  async function update() {
    updating = true;
    try {
      await siteUpdateInstall();
    } catch (e) {
      trouble = l("Güncelleme olmadı: ", "The update didn't go through: ") + String(e);
      updating = false;
    }
  }

  async function load() {
    loading = true;
    trouble = "";
    try {
      link = await siteLink();
      if (!link) return;
      door = await siteDoor(link.url);
      newer = await siteUpdate().catch(() => null);
      if (door.api !== API) return;
      home = await site.call<Home>("home", {});
    } catch (e) {
      if (e instanceof SignedOut) {
        // Taken off on the site: the link is no good, and goes.
        await siteForget().catch(() => {});
        link = door = home = null;
        trouble = l(
          "Site bu bilgisayarı artık tanımıyor; yeniden bağlanabilirsin.",
          "The site no longer knows this computer; you can link it again.",
        );
      } else trouble = String(e);
    } finally {
      loading = false;
    }
  }
  onMount(load);

  async function connect() {
    busy = true;
    trouble = "";
    try {
      const d = await siteDoor(address);
      code = (await siteConnect(address)).code;
      door = d;
      link = await siteWait();
      code = "";
      notify({ text: l(`${link.name} olarak bağlandın.`, `Linked as ${link.name}.`) });
      await load();
    } catch (e) {
      code = "";
      trouble = String(e);
    } finally {
      busy = false;
    }
  }

  async function forget(e: MouseEvent) {
    const yes = await ask({
      title: l("Site bağlantısı kaldırılsın mı?", "Unlink the site?"),
      detail: l("Sitedeki oyunların orada kalır; buradan açılmaz.", "Your plays stay on the site; they no longer open here."),
      yes: l("Kaldır", "Unlink"),
      no: l("Vazgeç", "Cancel"),
      danger: true,
      at: e,
    });
    if (!yes) return;
    await siteForget();
    link = door = home = null;
  }

  async function openThere(p: PlayCard) {
    await useSite(true);
    open(p.path);
  }

  const when = (s: number | null) =>
    s ? new Date(s * 1000).toLocaleDateString(tr ? "tr-TR" : "en-GB", { day: "numeric", month: "short" }) : "";
</script>

<h1>{l("Sitede", "On the site")}</h1>

{#snippet shelf(title: string, plays: PlayCard[], note: string)}
  <section class="shelf">
    <h2 class="ui-eyebrow">{title} <span class="ui-chip">{plays.length}</span>{#if note}<span class="ui-chip accent">{note}</span>{/if}</h2>
    {#if plays.length}
      <ul>
        {#each plays as p (p.path)}
          <li>
            <button class="ui-card play" onclick={() => openThere(p)} title={p.title}>
              <span class="ui-clip title">{p.title}</span>
              <span class="meta ui-clip">{[p.author, p.draft, when(p.modified)].filter(Boolean).join(" · ")}</span>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="muted">{l("Burada oyun yok.", "No plays here.")}</p>
    {/if}
  </section>
{/snippet}

{#if loading && !code}
  <p class="muted">{l("Bakılıyor…", "Looking…")}</p>
{:else if !link}
  <p class="lead">
    {l(
      "Bu bilgisayarı bir Dactile sitesindeki hesabına bağla: oradaki oyunların ve projelerin burada da açılır, birlikte yazma dahil. Parola buraya yazılmaz; tarayıcıda onaylarsın.",
      "Link this computer to your account on a Dactile site: your plays and projects there open here too, writing together included. No password is typed here; you say yes in the browser.",
    )}
  </p>
  {#if code}
    <div class="ui-card waiting">
      <p>{l("Tarayıcıda açılan sayfada bu kodu gör ve onayla:", "On the page the browser opened, look for this code and say yes:")}</p>
      <p class="code">{code.slice(0, 4)}-{code.slice(4)}</p>
      <p class="muted">{l("Bekleniyor… (on dakika)", "Waiting… (ten minutes)")}</p>
    </div>
  {:else}
    <form
      class="connect"
      onsubmit={(e) => {
        e.preventDefault();
        void connect();
      }}
    >
      <label class="ui-label">
        {l("Sitenin adresi", "The site's address")}
        <input class="ui-field" bind:value={address} placeholder="https://oyun.example" spellcheck="false" autocapitalize="none" required />
      </label>
      <button class="ui-btn primary" disabled={busy || !address.trim()}>{l("Bağlan", "Link")}</button>
    </form>
  {/if}
  {#if trouble}<p class="error">{trouble}</p>{/if}
{:else}
  <div class="who">
    <span class="ui-clip">
      {l("Bağlı", "Linked")}: <b>{link.name}</b> · {link.url.replace(/^https?:\/\//, "")}
      {#if door}<span class="muted">· {l("site", "site")} {door.version}</span>{/if}
    </span>
    <span class="grow"></span>
    {#if newer && fits}
      <button class="ui-btn small primary" disabled={updating} onclick={update}>
        {updating ? l("Güncelleniyor…", "Updating…") : l("Yeni sürüm: güncelle", "New version: update")}
      </button>
    {/if}
    <button class="ui-btn small" onclick={load}>{l("Yenile", "Refresh")}</button>
    <button class="ui-btn small danger" onclick={forget}>{l("Bağlantıyı kaldır", "Unlink")}</button>
  </div>
  {#if trouble}
    <p class="error">{trouble}</p>
  {:else if door && !fits}
    <div class="ui-card waiting">
      <p>
        <b>{l("Sürümler uyuşmuyor.", "The versions don't match.")}</b>
        {l(
          `Site ${door.version}, bu Dactile başka bir sürüm. Sitedeki oyunlar, ikisi aynı sürüme gelince buradan açılır.`,
          `The site is ${door.version}, this Dactile another version. The site's plays open here once both are the same.`,
        )}
      </p>
      {#if newer}
        <button class="ui-btn primary" disabled={updating} onclick={update}>
          {updating ? l("Güncelleniyor…", "Updating…") : l("Sitenin sürümüne güncelle", "Update to the site's version")}
        </button>
      {/if}
    </div>
  {:else if home}
    {@render shelf(l("Oyunlarım", "My plays"), home.plays, "")}
    {#each home.projects ?? [] as pr (pr.id)}
      {@render shelf(pr.name, pr.plays, pr.mode === "together" ? l("birlikte", "together") : l("sırayla", "in turns"))}
    {/each}
  {/if}
{/if}

<style>
  .lead {
    max-width: 640px;
    color: var(--chrome-muted);
  }
  .connect {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: end;
    max-width: 640px;
  }
  .connect .ui-label {
    flex: 1 1 280px;
  }
  .waiting {
    max-width: 520px;
    padding: 14px 18px;
  }
  .waiting p {
    margin: 0 0 6px;
  }
  .code {
    color: var(--accent-ink);
    font: 600 28px var(--mono);
    letter-spacing: 0.12em;
  }
  .who {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
    max-width: 900px;
    margin-bottom: 16px;
  }
  .grow {
    flex: 1;
  }
  .shelf {
    max-width: 900px;
    margin-bottom: 20px;
  }
  .shelf h2 .ui-chip {
    text-transform: none;
    letter-spacing: 0;
  }
  .shelf h2 {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 8px;
  }
  ul {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .play {
    display: grid;
    gap: 2px;
    width: 100%;
    padding: 10px 12px;
    color: var(--chrome-fg);
    font: 13px var(--sans);
    text-align: left;
    cursor: pointer;
  }
  .play:hover {
    border-color: var(--accent-ink);
  }
  .title {
    font: 600 15px var(--serif);
  }
  .meta,
  .muted {
    color: var(--chrome-muted);
    font-size: 12px;
  }
  .error {
    color: var(--danger);
  }
</style>
