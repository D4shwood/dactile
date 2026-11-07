<script lang="ts">
  // Notes beside the play, from anyone: kept in `oyun.notlar.md` next to it
  // (core/src/notes.rs). Each finds its place by what it quotes.
  import type { NoteCard } from "./api";

  let {
    tr,
    cards,
    picked,
    authors,
    draft = $bindable(),
    pasting = $bindable(),
    me = $bindable(),
    pick,
    set,
    add,
    reply,
    fix,
    paste,
    fromFile,
    close,
  }: {
    tr: boolean;
    cards: NoteCard[];
    picked: number | null;
    authors: string[];
    /// A new note being written: its text, or null.
    draft: string | null;
    /// A reply being pasted: its text, or null.
    pasting: string | null;
    me: string;
    pick: (i: number) => void;
    set: (i: number, state: string) => void;
    add: (author: string, body: string) => void;
    reply: (i: number, author: string, text: string) => void;
    fix: (i: number) => void;
    paste: (text: string, author: string) => void;
    fromFile: () => void;
    close: () => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let show = $state<"open" | "all" | "archive">("open");
  let replying = $state<number | null>(null);
  let replyText = $state("");
  let pasteFrom = $state("");

  const shown = $derived(
    cards.filter((c) =>
      show === "archive" ? c.archived : !c.archived && (show === "all" || c.state === "open"),
    ),
  );
  const counts = $derived({
    open: cards.filter((c) => !c.archived && c.state === "open").length,
    all: cards.filter((c) => !c.archived).length,
    archive: cards.filter((c) => c.archived).length,
  });
</script>

<datalist id="note-authors">
  {#each authors as a}<option value={a}></option>{/each}
</datalist>

<aside class="notes">
  <div class="head">
    <strong>{l("Notlar", "Notes")}</strong>
    <span class="grow"></span>
    <button title={l("Buraya not (Alt+N)", "Note here (Alt+N)")} onclick={() => (draft = draft ?? "")}>✚</button>
    <button title={l("Panodan yapıştır", "Paste a reply")} onclick={() => (pasting = pasting ?? "")}>⎗</button>
    <button title={l("Dosyadan", "From a file")} onclick={fromFile}>⇲</button>
    <button title={l("Kapat", "Close")} onclick={close}>✕</button>
  </div>
  <nav class="filter">
    <button class:on={show === "open"} onclick={() => (show = "open")}>{l("Açık", "Open")} {counts.open}</button>
    <button class:on={show === "all"} onclick={() => (show = "all")}>{l("Hepsi", "All")} {counts.all}</button>
    <button class:on={show === "archive"} onclick={() => (show = "archive")}>{l("Arşiv", "Archive")} {counts.archive}</button>
  </nav>

  {#if draft !== null}
    <form
      class="form"
      onsubmit={(e) => {
        e.preventDefault();
        add(me, draft ?? "");
      }}
    >
      <input bind:value={me} list="note-authors" placeholder={l("Kimden", "From")} />
      <!-- svelte-ignore a11y_autofocus -->
      <textarea bind:value={draft} rows="3" autofocus placeholder={l("“alıntı” ve not", "“quote” and the note")}></textarea>
      <div class="row">
        <button class="primary">{l("Ekle", "Add")}</button>
        <button type="button" onclick={() => (draft = null)}>{l("Vazgeç", "Cancel")}</button>
      </div>
    </form>
  {/if}

  {#if pasting !== null}
    <form
      class="form"
      onsubmit={(e) => {
        e.preventDefault();
        paste(pasting ?? "", pasteFrom);
      }}
    >
      <p class="hint">
        {l(
          "Numaralı maddeler not olur: “139: …” ya da “- 20: …”. Bir AI'dan, atölyeden, okurdan.",
          "Numbered items become notes: “139: …” or “- 20: …”. From an AI, a workshop, a reader.",
        )}
      </p>
      <input bind:value={pasteFrom} list="note-authors" placeholder={l("Kimden (Claude, Atölye…)", "From (Claude, Workshop…)")} />
      <textarea bind:value={pasting} rows="7"></textarea>
      <div class="row">
        <button class="primary" disabled={!pasteFrom.trim() || !pasting?.trim()}>{l("Notlara çevir", "Make notes")}</button>
        <button type="button" onclick={() => (pasting = null)}>{l("Vazgeç", "Cancel")}</button>
      </div>
    </form>
  {/if}

  <div class="list">
    {#if !shown.length}
      <p class="empty">
        {show === "open"
          ? l("Açık not yok. Bir yeri seçip Alt+N ile not yaz, ya da bir cevabı yapıştır.", "No open notes. Select something and press Alt+N, or paste a reply.")
          : l("Burada not yok.", "No notes here.")}
      </p>
    {/if}
    {#each shown as c, k (c.i)}
      {#if c.group && c.group !== shown[k - 1]?.group}
        <h4>{c.group}</h4>
      {/if}
      <div
        class="note"
        class:picked={picked === c.i}
        class:done={c.state !== "open"}
        role="button"
        tabindex="0"
        onclick={() => pick(c.i)}
        onkeydown={(e) => e.key === "Enter" && pick(c.i)}
      >
        <div class="meta">
          <input
            type="checkbox"
            checked={c.state === "done"}
            title={l("Yapıldı", "Done")}
            onclick={(e) => {
              e.stopPropagation();
              set(c.i, c.state === "done" ? "open" : "done");
            }}
          />
          <span class="author">{c.author || "—"}</span>
          {#if c.line}<span class="line">{c.line}</span>{/if}
          {#if c.gone && c.state === "open"}<span class="gone" title={l("Gösterdiği yer değişmiş: düzeltilmiş olabilir.", "What it points at changed: maybe put right.")}>✓?</span>{/if}
          {#if c.from === null}<span class="lost" title={l("Metinde yeri bulunamadı", "Its place wasn't found")}>?</span>{/if}
        </div>
        <div class="text">{c.text}</div>
        {#each c.replies as [who, what]}
          <div class="reply"><b>{who}</b>: {what}</div>
        {/each}
        {#if picked === c.i}
          <div class="actions">
            {#if c.fixable && c.state === "open"}
              <button class="primary" onclick={(e) => (e.stopPropagation(), fix(c.i))}>{l("Uygula", "Apply")}</button>
            {/if}
            <button onclick={(e) => (e.stopPropagation(), (replying = c.i), (replyText = ""))}>{l("Cevap", "Reply")}</button>
            <button onclick={(e) => (e.stopPropagation(), set(c.i, c.archived ? "unarchive" : "archive"))}>
              {c.archived ? l("Arşivden çıkar", "Unarchive") : l("Arşivle", "Archive")}
            </button>
            <button onclick={(e) => (e.stopPropagation(), set(c.i, c.state === "hidden" ? "open" : "hidden"))}>
              {c.state === "hidden" ? l("Göster", "Show") : l("Gizle", "Hide")}
            </button>
            <button
              class="danger"
              onclick={(e) => {
                e.stopPropagation();
                if (confirm(l("Bu not silinsin mi?", "Delete this note?"))) set(c.i, "delete");
              }}>{l("Sil", "Delete")}</button
            >
          </div>
          {#if replying === c.i}
            <form
              class="form inline"
              onsubmit={(e) => {
                e.preventDefault();
                reply(c.i, me, replyText);
                replying = null;
              }}
            >
              <!-- svelte-ignore a11y_autofocus -->
              <textarea bind:value={replyText} rows="2" autofocus></textarea>
              <button class="primary" disabled={!replyText.trim()}>{l("Gönder", "Reply")}</button>
            </form>
          {/if}
        {/if}
      </div>
    {/each}
  </div>
</aside>

<style>
  .notes {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--chrome);
    border-left: 1px solid var(--chrome-line);
  }
  .head {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 6px 10px;
    border-bottom: 1px solid var(--chrome-line);
  }
  .head button {
    border: 1px solid var(--chrome-line);
    background: var(--chrome);
    border-radius: 4px;
    width: 28px;
    height: 24px;
    cursor: pointer;
  }
  .grow {
    flex: 1;
  }
  .filter {
    display: flex;
    gap: 4px;
    padding: 6px 10px;
  }
  .filter button {
    border: 0;
    background: none;
    padding: 3px 8px;
    border-radius: 10px;
    cursor: pointer;
    color: var(--chrome-muted);
    font-size: 12px;
  }
  .filter button.on {
    background: var(--claret);
    color: #f6efe4;
  }
  .list {
    flex: 1;
    overflow: auto;
    padding: 4px 10px 20px;
    user-select: text;
  }
  .empty,
  .hint {
    color: var(--chrome-muted);
    font-size: 12px;
  }
  h4 {
    margin: 12px 0 4px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--chrome-muted);
  }
  .note {
    padding: 8px 10px;
    margin: 6px 0;
    border-radius: 6px;
    background: var(--chrome-2);
    border-left: 3px solid var(--accent-ink);
    cursor: pointer;
  }
  .note.done {
    opacity: 0.55;
    border-left-color: var(--chrome-line);
  }
  .note.picked {
    outline: 2px solid var(--accent-ink);
    opacity: 1;
  }
  .meta {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 12px;
  }
  .author {
    font-weight: 700;
  }
  .line,
  .gone,
  .lost {
    color: var(--chrome-muted);
  }
  .text {
    margin-top: 3px;
    white-space: pre-wrap;
    font-family: var(--serif);
    font-size: 14px;
  }
  .reply {
    margin: 4px 0 0 12px;
    font-size: 12.5px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  button {
    border: 1px solid var(--chrome-line);
    background: var(--chrome);
    border-radius: 4px;
    padding: 3px 9px;
    cursor: pointer;
    font-size: 12px;
  }
  .primary {
    background: var(--claret);
    color: #f6efe4;
    border-color: var(--claret);
  }
  .primary:disabled {
    opacity: 0.5;
  }
  .danger {
    color: var(--accent-ink);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 6px 10px;
    padding: 8px;
    background: var(--chrome-2);
    border-radius: 6px;
  }
  .form.inline {
    margin: 6px 0 0;
  }
  .form input,
  .form textarea {
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    padding: 5px 8px;
    background: var(--chrome);
    color: var(--chrome-fg);
    resize: vertical;
  }
  .row {
    display: flex;
    gap: 6px;
  }
</style>
