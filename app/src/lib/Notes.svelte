<script lang="ts">
  // Notes beside the play, from anyone: kept in `oyun.notlar.md` next to it
  // (core/src/notes.rs). Each finds its place by what it quotes.
  import { ask } from "./asking.svelte";
  import type { NoteCard } from "./api";

  let {
    tr,
    cards,
    picked,
    authors,
    draft = $bindable(),
    pasting = $bindable(),
    me = $bindable(),
    signedIn = false,
    reader = false,
    pick,
    set,
    add,
    reply,
    fix,
    fixMany,
    back,
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
    /// On a site, signed in: the notes are `me`'s, no name to type.
    signedIn?: boolean;
    /// One who only reads the play: notes and replies, and deleting their own.
    reader?: boolean;
    pick: (i: number) => void;
    set: (i: number, state: string) => void;
    add: (author: string, body: string) => void;
    reply: (i: number, author: string, text: string) => void;
    fix: (i: number) => void;
    fixMany: (which: number[]) => void;
    /// To the text, from the list.
    back: () => void;
    paste: (text: string, author: string) => void;
    fromFile: () => void;
    close: () => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  /// Each name its own colour, the same everywhere.
  const hue = (name: string) => [...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7);
  const mayDelete = (c: NoteCard) => !reader || (signedIn && c.author === me);
  let show = $state<"open" | "all" | "archive">("open");
  let replying = $state<number | null>(null);
  let replyText = $state("");
  let pasteFrom = $state("");

  const shown = $derived(
    cards.filter((c) =>
      show === "archive" ? c.archived : !c.archived && (show === "all" || c.state === "open"),
    ),
  );
  const fixable = (list: NoteCard[]) => list.filter((c) => c.fixable && c.state === "open");
  const fixableAll = $derived(fixable(shown));
  const inGroup = (g: string) => fixable(shown.filter((c) => c.group === g));

  let list: HTMLElement | undefined = $state();
  export function focusList() {
    list?.focus();
    if (picked === null && shown.length) pick(shown[0].i);
  }

  /// As in Dactile Light: the arrows move, u applies, g its part, U all.
  function keys(e: KeyboardEvent) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    const at = shown.findIndex((c) => c.i === picked);
    const c = shown[at];
    const step = (d: number) => {
      const next = shown[Math.max(0, Math.min(shown.length - 1, at + d))];
      if (next) pick(next.i);
    };
    const acts: Record<string, () => void> = {
      ArrowUp: () => step(-1),
      ArrowDown: () => step(1),
      PageUp: () => step(-8),
      PageDown: () => step(8),
      Home: () => shown[0] && pick(shown[0].i),
      End: () => shown.length && pick(shown[shown.length - 1].i),
      Escape: back,
      Enter: back,
      n: () => (draft = draft ?? ""),
      U: () => !reader && fixableAll.length && fixMany(fixableAll.map((x) => x.i)),
    };
    if (c) acts.r = () => ((replying = c.i), (replyText = ""));
    if (c && !reader) {
      acts.u = () => c.fixable && c.state === "open" && fix(c.i);
      acts.g = () => {
        const part = inGroup(c.group);
        if (part.length) fixMany(part.map((x) => x.i));
      };
      acts.a = () => set(c.i, c.archived ? "unarchive" : "archive");
      acts.h = () => set(c.i, c.state === "hidden" ? "open" : "hidden");
      acts[" "] = () => set(c.i, c.state === "done" ? "open" : "done");
    }
    const act = acts[e.key];
    if (act) {
      e.preventDefault();
      e.stopPropagation();
      act();
    }
  }

  const counts = $derived({
    open: cards.filter((c) => !c.archived && c.state === "open").length,
    all: cards.filter((c) => !c.archived).length,
    archive: cards.filter((c) => c.archived).length,
  });
</script>

<datalist id="note-authors">
  {#each authors as a}<option value={a}></option>{/each}
</datalist>

<aside class="notes ui-panel">
  <div class="ui-panel-head">
    <strong>{l("Notlar", "Notes")}</strong>
    <span class="grow"></span>
    <button class="ui-btn ghost icon" title={l("Buraya not (Alt+N)", "Note here (Alt+N)")} onclick={() => (draft = draft ?? "")}>✚</button>
    {#if !reader}
      <button class="ui-btn ghost icon" title={l("Panodan yapıştır", "Paste a reply")} onclick={() => (pasting = pasting ?? "")}>⎗</button>
      <button class="ui-btn ghost icon" title={l("Dosyadan birleştir: olan notlar iki kez gelmez", "Merge from a file: notes already here don't come twice")} onclick={fromFile}>⇲</button>
    {/if}
    <button class="ui-btn ghost icon" title={l("Kapat", "Close")} onclick={close}>✕</button>
  </div>
  <nav class="filter ui-pills">
    <button class:on={show === "open"} onclick={() => (show = "open")}>{l("Açık", "Open")} {counts.open}</button>
    <button class:on={show === "all"} onclick={() => (show = "all")}>{l("Hepsi", "All")} {counts.all}</button>
    <button class:on={show === "archive"} onclick={() => (show = "archive")}>{l("Arşiv", "Archive")} {counts.archive}</button>
  </nav>

  {#if fixableAll.length && !reader}
    <div class="applyall">
      <button class="ui-btn small" onclick={() => fixMany(fixableAll.map((x) => x.i))}>
        {l(`Tümünü uygula (${fixableAll.length})`, `Apply all (${fixableAll.length})`)}
      </button>
      <kbd>U</kbd>
    </div>
  {/if}

  {#if draft !== null}
    <form
      class="form"
      onsubmit={(e) => {
        e.preventDefault();
        add(me, draft ?? "");
      }}
    >
      {#if !signedIn}
        <input class="ui-field" bind:value={me} list="note-authors" placeholder={l("Kimden", "From")} />
      {/if}
      <!-- svelte-ignore a11y_autofocus -->
      <textarea class="ui-field" bind:value={draft} rows="3" autofocus placeholder={l("“alıntı” ve not", "“quote” and the note")}></textarea>
      <div class="row">
        <button class="ui-btn small primary">{l("Ekle", "Add")}</button>
        <button class="ui-btn small" type="button" onclick={() => (draft = null)}>{l("Vazgeç", "Cancel")}</button>
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
      <input class="ui-field" bind:value={pasteFrom} list="note-authors" placeholder={l("Kimden (Claude, Atölye…)", "From (Claude, Workshop…)")} />
      <textarea class="ui-field" bind:value={pasting} rows="7"></textarea>
      <div class="row">
        <button class="ui-btn small primary" disabled={!pasteFrom.trim() || !pasting?.trim()}>{l("Notlara çevir", "Make notes")}</button>
        <button class="ui-btn small" type="button" onclick={() => (pasting = null)}>{l("Vazgeç", "Cancel")}</button>
      </div>
    </form>
  {/if}

  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_static_element_interactions -->
  <div class="list" tabindex="0" bind:this={list} onkeydown={keys}>
    {#if !shown.length}
      <p class="empty">
        {show === "open"
          ? l("Açık not yok. Bir yeri seçip Alt+N ile not yaz, ya da bir cevabı yapıştır.", "No open notes. Select something and press Alt+N, or paste a reply.")
          : l("Burada not yok.", "No notes here.")}
      </p>
    {/if}
    {#each shown as c, k (c.i)}
      {#if c.group && c.group !== shown[k - 1]?.group}
        <h4 class="ui-eyebrow">
          <span>{c.group}</span>
          {#if inGroup(c.group).length && !reader}
            <button class="ui-btn small" onclick={() => fixMany(inGroup(c.group).map((x) => x.i))}>
              {l(`Bu kısmı uygula (${inGroup(c.group).length})`, `Apply this part (${inGroup(c.group).length})`)}
            </button>
          {/if}
        </h4>
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
            disabled={reader}
            title={l("Yapıldı", "Done")}
            onclick={(e) => {
              e.stopPropagation();
              set(c.i, c.state === "done" ? "open" : "done");
            }}
          />
          <span class="author" title={c.author} style:--who={c.author ? `hsl(${hue(c.author)} 55% 45%)` : "transparent"}>{c.author || "—"}</span>
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
            {#if c.fixable && c.state === "open" && !reader}
              <button class="ui-btn small primary" onclick={(e) => (e.stopPropagation(), fix(c.i))}>{l("Uygula", "Apply")}</button>
            {/if}
            <button class="ui-btn small" onclick={(e) => (e.stopPropagation(), (replying = c.i), (replyText = ""))}>{l("Cevap", "Reply")}</button>
            {#if !reader}
            <button class="ui-btn small" onclick={(e) => (e.stopPropagation(), set(c.i, c.archived ? "unarchive" : "archive"))}>
              {c.archived ? l("Arşivden çıkar", "Unarchive") : l("Arşivle", "Archive")}
            </button>
            <button class="ui-btn small" onclick={(e) => (e.stopPropagation(), set(c.i, c.state === "hidden" ? "open" : "hidden"))}>
              {c.state === "hidden" ? l("Göster", "Show") : l("Gizle", "Hide")}
            </button>
            {/if}
            {#if mayDelete(c)}
            <button
              class="ui-btn small danger"
              onclick={async (e) => {
                e.stopPropagation();
                if (await ask({ title: l("Bu not silinsin mi?", "Delete this note?"), yes: l("Sil", "Delete"), no: l("Vazgeç", "Cancel"), danger: true, at: e }))
                  set(c.i, "delete");
              }}>{l("Sil", "Delete")}</button
            >
            {/if}
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
              <textarea class="ui-field" bind:value={replyText} rows="2" autofocus></textarea>
              <button class="ui-btn small primary" disabled={!replyText.trim()}>{l("Gönder", "Reply")}</button>
            </form>
          {/if}
        {/if}
      </div>
    {/each}
  </div>
</aside>

<style>
  .grow {
    flex: 1;
  }
  .filter {
    padding: 6px 10px;
  }
  .list {
    flex: 1;
    overflow: auto;
    padding: 4px 10px 20px;
    user-select: text;
  }
  .list:focus {
    outline: none;
    box-shadow: inset 0 0 0 1px var(--accent-ink);
  }
  .empty,
  .hint {
    margin: 0;
    color: var(--chrome-muted);
    font-size: 12px;
  }
  .empty {
    margin: 8px 0;
  }
  h4 {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: space-between;
    margin: 14px 0 4px;
  }
  h4 button {
    text-transform: none;
    letter-spacing: 0;
  }
  .note {
    min-width: 0;
    margin: 6px 0;
    padding: 8px 10px;
    overflow-wrap: anywhere;
    background: var(--raised);
    border: 1px solid var(--chrome-line);
    border-left: 3px solid var(--accent-ink);
    border-radius: var(--r-md);
    cursor: pointer;
  }
  .note:hover {
    border-color: color-mix(in srgb, var(--chrome-fg) 25%, var(--chrome-line));
    border-left-color: var(--accent-ink);
  }
  .note.done {
    opacity: 0.55;
    border-left-color: var(--chrome-line);
  }
  .note.picked {
    opacity: 1;
    box-shadow: 0 0 0 2px var(--on);
    border-color: var(--accent-ink);
  }
  .meta {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
    font-size: 12px;
  }
  .meta input {
    accent-color: var(--accent-ink);
  }
  .author::before {
    content: "";
    display: inline-block;
    width: 0.55em;
    height: 0.55em;
    margin-right: 0.35em;
    border-radius: 50%;
    background: var(--who);
    vertical-align: 0.08em;
  }
  .author {
    min-width: 0;
    overflow: hidden;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .line,
  .gone,
  .lost {
    color: var(--chrome-muted);
  }
  .text {
    margin-top: 3px;
    font: 14px var(--serif);
    white-space: pre-wrap;
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
  .form {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 6px 10px;
    padding: 10px;
    background: var(--raised);
    border: 1px solid var(--chrome-line);
    border-radius: var(--r-md);
  }
  .form.inline {
    margin: 6px 0 0;
    padding: 0;
    background: none;
    border: 0;
  }
  .form.inline button {
    align-self: flex-end;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  .applyall {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 12px;
    border-bottom: 1px solid var(--chrome-line);
  }
  .applyall kbd {
    color: var(--chrome-muted);
    font-size: 11px;
  }
</style>
