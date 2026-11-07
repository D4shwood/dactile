<script lang="ts">
  import { FILE_TAB, TABS, tips, type Act, type Btn } from "./ribbon";

  let {
    tr,
    folded,
    title,
    dirty,
    run,
    file,
    height,
    resized,
    font,
    fonts,
    setFont,
  }: {
    tr: boolean;
    folded: boolean;
    title: string;
    dirty: boolean;
    run: (a: Act) => void;
    file: () => void;
    /// The buttons' row in pixels, dragged by hand; 0 is its own.
    height: number;
    resized: (h: number) => void;
    /// The page's font and the list it comes from; empty is the app's own.
    font: string;
    fonts: string[];
    setFont: (f: string) => void;
  } = $props();

  const LOW = 58;
  const HIGH = 132;
  const OWN = 92;
  /// While the edge is dragged.
  let dragged = $state(0);
  const rowHeight = $derived(dragged || height || OWN);
  function dragEdge(e: PointerEvent) {
    const edge = e.currentTarget as HTMLElement;
    const startY = e.clientY;
    const from = rowHeight;
    edge.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => {
      dragged = Math.round(Math.min(HIGH, Math.max(LOW, from + m.clientY - startY)));
    };
    const up = () => {
      edge.removeEventListener("pointermove", move);
      edge.removeEventListener("pointerup", up);
      if (dragged) resized(dragged);
      dragged = 0;
    };
    edge.addEventListener("pointermove", move);
    edge.addEventListener("pointerup", up);
    e.preventDefault();
  }

  let tab = $state(1);
  /// Folded, a tab opens over the page until a button is pressed.
  let peek = $state(false);
  /// Alt pressed: "tabs" shows a letter on each tab, "btns" on each button.
  let tipMode = $state<"" | "tabs" | "btns">("");
  let query = $state("");
  let searchBox: HTMLInputElement | undefined = $state();

  const name = (x: { tr: string; en: string }) => (tr ? x.tr : x.en);
  const btnTips = $derived(tips(TABS[tab], tr));
  const all = $derived(
    TABS.flatMap((t) => t.groups.flatMap((g) => g.btns.map((b) => ({ b, where: name(t) })))),
  );
  const found = $derived(
    query.trim() === ""
      ? []
      : all
          .filter(({ b }) =>
            name(b).toLocaleLowerCase("tr").includes(query.trim().toLocaleLowerCase("tr")),
          )
          .slice(0, 8),
  );

  function choose(i: number) {
    if (i === FILE_TAB) {
      file();
      return;
    }
    if (folded && tab === i) peek = !peek;
    else peek = folded;
    tab = i;
  }

  function press(b: Btn) {
    peek = false;
    tipMode = "";
    run(b.act);
  }

  export function focusSearch() {
    searchBox?.focus();
  }

  /// Alt pressed and nothing else yet: on its release the letters show.
  let altAlone = false;

  function onkeyup(e: KeyboardEvent) {
    if (e.key === "Alt" && altAlone) {
      // Alt alone, as in Word: the letters show; again, they go.
      tipMode = tipMode ? "" : "tabs";
      e.preventDefault();
    }
    altAlone = false;
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Alt") {
      altAlone = !e.repeat;
      e.preventDefault();
      return;
    }
    // Alt with a key (Alt+N): a shortcut, not the letters.
    altAlone = false;
    if (e.altKey) return;
    if (!tipMode) return;
    if (e.key === "Escape") {
      tipMode = tipMode === "btns" ? "tabs" : "";
      e.preventDefault();
      return;
    }
    const c = e.key.toLocaleUpperCase(tr ? "tr" : "en");
    if (tipMode === "tabs") {
      const i = TABS.findIndex((t) => (tr ? t.tip[0] : t.tip[1]) === c);
      if (i >= 0) {
        e.preventDefault();
        if (i === FILE_TAB) {
          tipMode = "";
          file();
        } else {
          tab = i;
          peek = folded;
          tipMode = "btns";
        }
      }
    } else {
      const i = btnTips.indexOf(c);
      if (i >= 0) {
        e.preventDefault();
        const btn = TABS[tab].groups.flatMap((g) => g.btns)[i];
        press(btn);
      }
    }
  }
</script>

<svelte:window onkeydown={onkey} {onkeyup} />

<header class="ribbon" class:folded>
  <div class="bar">
    <div class="quick">
      <button title={tr ? "Kaydet (Ctrl+S)" : "Save (Ctrl+S)"} onclick={() => run({ do: "key", id: "save" })}>▣</button>
      <button title={tr ? "Geri al" : "Undo"} onclick={() => run({ do: "key", id: "undo" })}>↶</button>
      <button title={tr ? "Yinele" : "Redo"} onclick={() => run({ do: "key", id: "redo" })}>↷</button>
    </div>
    <div class="title">{title}{dirty ? " •" : ""}</div>
    <div class="search">
      <input
        bind:this={searchBox}
        bind:value={query}
        placeholder={tr ? "Komut ara…" : "Search commands…"}
        onkeydown={(e) => {
          if (e.key === "Enter" && found[0]) {
            press(found[0].b);
            query = "";
          }
          if (e.key === "Escape") query = "";
        }}
      />
      {#if found.length}
        <ul class="found">
          {#each found as f}
            <li>
              <button
                disabled={f.b.act.do === "soon"}
                onclick={() => {
                  press(f.b);
                  query = "";
                }}
              >
                <span class="icon">{f.b.icon}</span>{name(f.b)}<small>{f.where}</small>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>

  <nav class="tabs">
    {#each TABS as t, i}
      <button
        class:file={i === FILE_TAB}
        class:on={i === tab && i !== FILE_TAB && (!folded || peek)}
        onclick={() => choose(i)}
      >
        {name(t)}
        {#if tipMode === "tabs"}<span class="tip">{tr ? t.tip[0] : t.tip[1]}</span>{/if}
      </button>
    {/each}
    <button
      class="fold"
      title={folded ? (tr ? "Şeridi aç" : "Unfold") : tr ? "Şeridi katla (Ctrl+F1)" : "Fold (Ctrl+F1)"}
      onclick={() => run({ do: "key", id: "fold" })}>{folded ? "⌄" : "⌃"}</button
    >
  </nav>

  {#if !folded || peek}
    <div class="groups" class:over={folded} class:short={rowHeight < 76} style={`height:${rowHeight}px`}>
      {#each TABS[tab].groups as grp, gi}
        <section class="group" class:end={grp.end}>
          {#if grp.font}
            <select
              class="font"
              title={tr ? "Sayfanın yazı tipi: ekranda görünür, PDF'i değiştirmez" : "The page's font: on screen, the PDF keeps its own"}
              value={font}
              onchange={(e) => setFont(e.currentTarget.value)}
            >
              <option value="">{tr ? "Kendi yazı tipi" : "Its own font"}</option>
              {#each fonts as f (f)}<option value={f}>{f}</option>{/each}
            </select>
          {/if}
          <div class="btns" class:small={grp.small}>
            {#each grp.btns as btn, bi}
              {@const n = TABS[tab].groups.slice(0, gi).reduce((s, x) => s + x.btns.length, 0) + bi}
              <button
                class="btn"
                class:small={grp.small}
                disabled={btn.act.do === "soon"}
                title={(btn.act.do === "soon" ? (tr ? "Yakında · " : "Soon · ") : "") +
                  name(btn) +
                  (btn.key ? ` (${btn.key})` : "")}
                onclick={() => press(btn)}
              >
                <span class="icon">{btn.icon}</span>
                {#if !grp.small}<span class="label">{name(btn)}</span>{/if}
                {#if tipMode === "btns"}<span class="tip">{btnTips[n]}</span>{/if}
              </button>
            {/each}
          </div>
          <div class="gname">{name(grp)}</div>
        </section>
      {/each}
    </div>
    {#if !folded}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="edge"
        title={tr ? "Sürükle; çift tık: varsayılan" : "Drag; double click: its own height"}
        onpointerdown={dragEdge}
        ondblclick={() => resized(0)}
      ></div>
    {/if}
  {/if}
</header>

<style>
  .ribbon {
    position: relative;
    z-index: 5;
    background: var(--chrome);
    border-bottom: 1px solid var(--chrome-line);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 34px;
    padding: 0 10px;
    background: var(--claret);
    color: #f6efe4;
  }
  .quick {
    display: flex;
    gap: 2px;
  }
  .quick button {
    background: none;
    border: 0;
    width: 28px;
    height: 26px;
    border-radius: 4px;
    color: inherit;
    cursor: pointer;
    font-size: 15px;
  }
  .quick button:hover {
    background: rgba(255, 255, 255, 0.15);
  }
  .title {
    flex: 1;
    text-align: center;
    font-size: 13px;
    opacity: 0.95;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .search {
    position: relative;
    width: 260px;
  }
  .search input {
    width: 100%;
    height: 24px;
    border: 0;
    border-radius: 4px;
    padding: 0 8px;
    background: rgba(255, 255, 255, 0.92);
    color: #2a2520;
  }
  .found {
    position: absolute;
    right: 0;
    top: 28px;
    width: 300px;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--chrome);
    color: var(--chrome-fg);
    border: 1px solid var(--chrome-line);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  }
  .found button {
    display: flex;
    gap: 8px;
    align-items: center;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 5px 8px;
    border-radius: 4px;
    cursor: pointer;
  }
  .found button:hover:not(:disabled) {
    background: var(--chrome-2);
  }
  .found small {
    margin-left: auto;
    color: var(--chrome-muted);
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 0 8px;
    height: 30px;
    align-items: end;
  }
  .tabs .tip {
    top: -4px;
    right: -2px;
  }
  .tabs button {
    position: relative;
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    padding: 4px 12px 5px;
    cursor: pointer;
    color: var(--chrome-fg);
  }
  .tabs button:hover {
    background: var(--chrome-2);
  }
  .tabs button.on {
    border-bottom-color: var(--accent-ink);
    color: var(--accent-ink);
    font-weight: 600;
  }
  .tabs button.file {
    background: var(--claret-deep);
    color: #f6efe4;
    border-radius: 4px 4px 0 0;
  }
  .tabs .fold {
    margin-left: auto;
    color: var(--chrome-muted);
  }
  .groups {
    display: flex;
    gap: 0;
    height: 92px;
    padding: 4px 6px 0;
    background: var(--chrome);
    border-top: 1px solid var(--chrome-line);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .groups.short .gname {
    display: none;
  }
  .edge {
    height: 5px;
    margin-bottom: -1px;
    cursor: row-resize;
    background: var(--chrome);
    border-bottom: 1px solid var(--chrome-line);
    touch-action: none;
  }
  .edge:hover {
    background: var(--chrome-line);
  }
  .groups.over {
    position: absolute;
    left: 0;
    right: 0;
    top: 100%;
    box-shadow: 0 8px 18px rgba(0, 0, 0, 0.15);
  }
  .group {
    display: flex;
    flex-direction: column;
    padding: 0 6px;
    flex: none;
    border-right: 1px solid var(--chrome-line);
  }
  .group.end {
    position: sticky;
    right: -6px;
    margin-left: auto;
    background: var(--chrome);
    border-right: 0;
    border-left: 1px solid var(--chrome-line);
  }
  .btns {
    flex: 1;
    display: flex;
    gap: 2px;
    min-height: 0;
  }
  .btns.small {
    display: grid;
    grid-template-rows: repeat(2, minmax(0, 30px));
    grid-auto-flow: column;
    grid-auto-columns: 30px;
    align-content: center;
  }
  .group:has(.font) .btns.small {
    grid-template-rows: minmax(0, 30px);
  }
  .font {
    width: 112px;
    height: 24px;
    margin-bottom: 2px;
    font-size: 12px;
    color: var(--chrome-fg);
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
  }
  .btn {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    min-width: 58px;
    max-width: 84px;
    padding: 4px 6px;
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
  }
  .btn.small {
    min-width: 0;
    padding: 0;
  }
  .btn.small .icon {
    font-size: 17px;
  }
  .btn:hover:not(:disabled) {
    background: var(--chrome-2);
    border-color: var(--chrome-line);
  }
  .btn:disabled {
    opacity: 0.38;
    cursor: default;
  }
  .btn .icon {
    font-size: 22px;
    line-height: 1;
    color: var(--accent-ink);
  }
  .btn .label {
    font-size: 11.5px;
    line-height: 1.15;
    text-align: center;
  }
  .gname {
    text-align: center;
    font-size: 11px;
    color: var(--chrome-muted);
    padding: 2px 0 4px;
  }
  .tip {
    position: absolute;
    right: 2px;
    top: 2px;
    background: #2a2520;
    color: #f6efe4;
    font-size: 11px;
    font-weight: 700;
    padding: 0 5px;
    border-radius: 3px;
    z-index: 2;
  }
  .found .icon {
    color: var(--accent-ink);
    width: 18px;
    text-align: center;
  }
</style>
