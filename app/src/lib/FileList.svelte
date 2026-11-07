<script lang="ts">
  // What belongs to a play, each a click from opening: on the start screen
  // and in the writer's "My files" panel.
  import { api, type Files, type Spot } from "./api";

  let {
    tr,
    files,
    open,
    reload,
  }: { tr: boolean; files: Files; open: (path: string) => void; reload: () => Promise<void> } = $props();

  let failed = $state("");

  const l = (t: string, e: string) => (tr ? t : e);
  const date = (s: number | null) =>
    s ? new Date(s * 1000).toLocaleDateString(tr ? "tr-TR" : "en-GB", { day: "numeric", month: "short", year: "numeric" }) : "";

  async function launch(s: Spot) {
    failed = "";
    await api.launch(s.path).catch((e) => (failed = String(e)));
  }

  /// A draft may have gone since the list was made: then the list changes
  /// and the play stays open.
  async function openDraft(path: string) {
    failed = "";
    await reload();
    if (files.drafts.some((d) => d.path === path)) open(path);
    else failed = l("Bu taslak artık yok: taşınmış ya da silinmiş olabilir.", "This draft is gone: it may have moved or been deleted.");
  }
</script>

{#snippet row(name: string, s: Spot, note: string)}
  <button class="item" title={s.path} onclick={() => launch(s)}>
    <span class="name">{name}</span>
    <span class="where">{s.shown}</span>
    {#if note}<span class="note">{note}</span>{/if}
  </button>
{/snippet}

{#snippet none(name: string, how: string)}
  <div class="item none">
    <span class="name">{name}</span>
    <span class="note">{how}</span>
  </div>
{/snippet}

<div class="filelist">
  {@render row(l("Klasör", "Folder"), files.folder, "")}
  {#if files.pdf}
    {@render row("PDF", files.pdf, date(files.pdf.modified))}
  {:else}
    {@render none("PDF", l("Henüz PDF yok: oyun açıkken F5 ile derle, yanına yazılır.", "No PDF yet: build it with F5 while the play is open; it goes beside it."))}
  {/if}
  {#if files.table}
    {@render row(l("Dağıtım tablosu", "Role table"), files.table, date(files.table.modified))}
  {/if}
  {#if files.copies}
    {@render row(
      l("Oyuncu kopyaları", "Actors' copies"),
      files.copies,
      [l(`${files.copies.pdfs} PDF`, `${files.copies.pdfs} PDFs`), date(files.copies.modified)].filter(Boolean).join(" · "),
    )}
  {:else}
    {@render none(l("Oyuncu kopyaları", "Actors' copies"), l("Henüz yok: oyun açıkken Dağıtım (F9) içinden yapılır.", "None yet: made from Casting (F9) while the play is open."))}
  {/if}
  {#each files.drafts as d (d.path)}
    <button class="item" title={d.path} onclick={() => openDraft(d.path)}>
      <span class="name">{l("Taslak", "Draft")}{d.draft ? ` ${d.draft}` : ""}</span>
      <span class="where">{d.title}</span>
      <span class="note">{date(d.modified)}</span>
    </button>
  {:else}
    {@render none(l("Taslaklar", "Drafts"), l("Henüz taslak yok: başlangıç ekranında Yeni taslak ile yapılır.", "No drafts yet: New draft on the start screen makes one."))}
  {/each}
  {#if failed}<p class="failed" role="alert">{failed}</p>{/if}
</div>

<style>
  .filelist {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .item {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 8px;
    min-width: 0;
    padding: 5px 6px;
    border: 0;
    border-bottom: 1px solid var(--chrome-line);
    background: none;
    color: var(--chrome-fg);
    font-size: 12px;
    text-align: left;
  }
  button.item {
    cursor: pointer;
  }
  button.item:hover {
    background: var(--chrome-2);
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .where {
    grid-row: 2;
    grid-column: 1 / 3;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--chrome-muted);
  }
  .note {
    color: var(--chrome-muted);
  }
  .none {
    grid-template-columns: minmax(0, 1fr);
  }
  .failed {
    margin: 6px 0 0;
    color: var(--claret);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
</style>
