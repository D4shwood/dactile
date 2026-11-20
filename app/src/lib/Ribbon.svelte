<script lang="ts">
  import { drag } from "./pointer";
  import { FILE_TAB, TABS, tips, type Act, type Btn } from "./ribbon";

  let {
    tr,
    folded,
    slim,
    title,
    dirty,
    run,
    file,
    height,
    resized,
    font,
    fonts,
    setFont,
    size,
    setSize,
  }: {
    tr: boolean;
    folded: boolean;
    /// One row, the tabs as menus, instead of the ribbon.
    slim: boolean;
    title: string;
    dirty: boolean;
    run: (a: Act) => void;
    file: () => void;
    /// The buttons' row in pixels, dragged by hand; 0 is its own.
    height: number;
    resized: (h: number) => void;
    /// The play's font and the list it comes from; empty is the class's.
    font: string;
    fonts: string[];
    setFont: (f: string) => void;
    /// The play's text size in points; empty is the class's.
    size: string;
    setSize: (s: string) => void;
  } = $props();

  const LOW = 58;
  const HIGH = 132;
  const OWN = 92;
  /// While the edge is dragged.
  let dragged = $state(0);
  const rowHeight = $derived(dragged || height || OWN);
  function dragEdge(e: PointerEvent) {
    const startY = e.clientY;
    const from = rowHeight;
    drag(
      e,
      (m) => (dragged = Math.round(Math.min(HIGH, Math.max(LOW, from + m.clientY - startY)))),
      () => {
        if (dragged) resized(dragged);
        dragged = 0;
      },
    );
    e.preventDefault();
  }

  let tab = $state(1);
  /// The slim bar's tabs, for where a menu opens.
  let menuTabs: HTMLButtonElement[] = $state([]);
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
    if ((folded || slim) && tab === i) peek = !peek;
    else peek = folded || slim;
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
    if (!tipMode && slim && peek && e.key === "Escape") {
      peek = false;
      e.preventDefault();
      e.stopImmediatePropagation();
      return;
    }
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
          peek = folded || slim;
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

{#snippet quickBtns()}
    <div class="quick">
      <button title={tr ? "Ana menü" : "Home"} onclick={() => run({ do: "key", id: "home" })}>⌂</button>
      <button title={tr ? "Geri" : "Back"} onclick={() => run({ do: "key", id: "back" })}>←</button>
      <button title={tr ? "Geri al" : "Undo"} onclick={() => run({ do: "key", id: "undo" })}>↶</button>
      <button title={tr ? "Yinele" : "Redo"} onclick={() => run({ do: "key", id: "redo" })}>↷</button>
    </div>
{/snippet}

{#snippet searchBox_()}
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
{/snippet}

{#snippet fontRow()}
            <div class="fontrow">
              <select
                class="font"
                title={tr ? "Oyunun yazı tipi: sayfa, PDF ve Word buna uyar" : "The play's font: the page, the PDF and Word follow it"}
                value={font}
                onchange={(e) => setFont(e.currentTarget.value)}
              >
                <option value="">{tr ? "Kendi yazı tipi" : "Its own font"}</option>
                {#each fonts as f (f)}<option value={f}>{f}</option>{/each}
              </select>
              <select
                class="size"
                title={tr ? "Oyun metninin boyutu: PDF ve Word buna uyar" : "The play's text size: the PDF and Word follow it"}
                value={size}
                onchange={(e) => setSize(e.currentTarget.value)}
              >
                <option value="">12</option>
                {#each ["10", "11", "13", "14"] as n (n)}<option value={n}>{n}</option>{/each}
                {#if size && !["10", "11", "13", "14"].includes(size)}<option value={size}>{size}</option>{/if}
              </select>
            </div>
{/snippet}

{#if slim}
  <header class="ribbon slim">
    <div class="row">
      {@render quickBtns()}
      <nav class="menus">
        {#each TABS as t, i}
          <button
            bind:this={menuTabs[i]}
            class:file={i === FILE_TAB}
            class:on={i === tab && peek && i !== FILE_TAB}
            aria-haspopup={i === FILE_TAB ? undefined : "menu"}
            aria-expanded={i === FILE_TAB ? undefined : i === tab && peek}
            onclick={() => choose(i)}
          >
            {name(t)}
            {#if tipMode === "tabs"}<span class="tip">{tr ? t.tip[0] : t.tip[1]}</span>{/if}
          </button>
        {/each}
      </nav>
      <div class="title">{title}{dirty ? " •" : ""}</div>
      {@render searchBox_()}
      <button
        class="mode"
        title={tr ? "Şeride geç: düğmeler sekmelerin altında" : "Switch to the ribbon: the buttons under the tabs"}
        onclick={() => run({ do: "key", id: "toolbar" })}>▤</button
      >
    </div>
    {#if peek && tab !== FILE_TAB}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="scrim" onclick={() => (peek = false)}></div>
      <div class="menu" role="menu" style:left="{menuTabs[tab]?.offsetLeft ?? 0}px">
        {#each TABS[tab].groups as grp, gi}
          <section>
            <p class="ui-eyebrow">{name(grp)}</p>
            {#if grp.font}{@render fontRow()}{/if}
            <div class="items" class:icons={grp.icons}>
              {#each grp.btns as btn, bi}
                {@const n = TABS[tab].groups.slice(0, gi).reduce((s, x) => s + x.btns.length, 0) + bi}
                <button
                  role="menuitem"
                  disabled={btn.act.do === "soon"}
                  title={(btn.act.do === "soon" ? (tr ? "Yakında · " : "Soon · ") : "") + name(btn) + (btn.key ? ` (${btn.key})` : "")}
                  onclick={() => press(btn)}
                >
                  <span class="icon">{btn.icon}</span>
                  {#if !grp.icons}<span class="label">{name(btn)}</span>{#if btn.key}<kbd>{btn.key}</kbd>{/if}{/if}
                  {#if tipMode === "btns"}<span class="tip">{btnTips[n]}</span>{/if}
                </button>
              {/each}
            </div>
          </section>
        {/each}
      </div>
    {/if}
  </header>
{:else}
<header class="ribbon" class:folded>
  <div class="bar">
    {@render quickBtns()}
    <div class="title">{title}{dirty ? " •" : ""}</div>
    {@render searchBox_()}
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
      class="mode"
      title={tr ? "İnce çubuğa geç: sekmeler menü olur" : "Switch to the slim bar: the tabs become menus"}
      onclick={() => run({ do: "key", id: "toolbar" })}>▭</button
    >
    <button
      class="fold"
      title={folded ? (tr ? "Şeridi aç" : "Unfold") : tr ? "Şeridi katla (Ctrl+F1)" : "Fold (Ctrl+F1)"}
      onclick={() => run({ do: "key", id: "fold" })}>{folded ? "⌄" : "⌃"}</button
    >
  </nav>

  {#if !folded || peek}
    <div class="groups" onwheel={(e) => (e.currentTarget.scrollLeft += e.deltaY)} class:over={folded} class:short={rowHeight < 76} style={`height:${rowHeight}px`}>
      {#each TABS[tab].groups as grp, gi}
        <section class="group" class:end={grp.end} class:shrinks={grp.shrinks}>
          {#if grp.font}{@render fontRow()}{/if}
          <div class="btns" class:small={grp.small} class:icons={grp.icons}>
            {#each grp.btns as btn, bi}
              {@const n = TABS[tab].groups.slice(0, gi).reduce((s, x) => s + x.btns.length, 0) + bi}
              <button
                class="btn"
                class:small={grp.small}
                class:icons={grp.icons}
                disabled={btn.act.do === "soon"}
                title={(btn.act.do === "soon" ? (tr ? "Yakında · " : "Soon · ") : "") +
                  name(btn) +
                  (btn.key ? ` (${btn.key})` : "")}
                onclick={() => press(btn)}
              >
                <span class="icon">{btn.icon}</span>
                {#if !grp.icons}<span class="label">{name(btn)}</span>{/if}
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
{/if}

<style>
  /* The slim bar: one row on the chrome, the tabs open as menus. */
  .slim {
    z-index: 6;
  }
  /* Over the scrim: another tab opens its menu with one click. */
  .slim .row {
    position: relative;
    z-index: 3;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 10px;
  }
  .slim .quick button {
    color: var(--chrome-fg);
  }
  .slim .quick button:hover {
    background: var(--hover);
  }
  .menus {
    display: flex;
    gap: 2px;
  }
  .menus button {
    position: relative;
    padding: 5px 10px;
    color: var(--chrome-fg);
    background: none;
    border: 0;
    border-radius: var(--r-md);
    cursor: pointer;
  }
  .menus button:hover,
  .menus button.on {
    background: var(--hover);
  }
  .menus button.on {
    color: var(--accent-ink);
    font-weight: 600;
  }
  .menus button.file {
    color: var(--on-claret);
    background: var(--claret);
  }
  .menus button.file:hover {
    background: var(--claret-deep);
  }
  .slim .title {
    color: var(--chrome-muted);
  }
  .slim .search input {
    color: var(--chrome-fg);
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: var(--r-md);
    height: 28px;
  }
  .slim .search .found {
    top: 32px;
  }
  .mode {
    width: 28px;
    height: 26px;
    padding: 0;
    color: var(--chrome-muted);
    background: none;
    border: 0;
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  .mode:hover {
    color: var(--accent-ink);
    background: var(--hover);
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 1;
  }
  .menu {
    position: absolute;
    top: 38px;
    z-index: 2;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    max-width: min(760px, calc(100vw - 20px));
    max-height: calc(100vh - 60px);
    padding: 10px;
    overflow-y: auto;
    background: var(--chrome);
    border: 1px solid var(--chrome-line);
    border-radius: var(--r-lg);
    box-shadow: var(--sh-2);
  }
  .menu section {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 190px;
  }
  .menu .ui-eyebrow {
    padding: 4px 8px 2px;
  }
  .menu .fontrow {
    padding: 0 6px 4px;
  }
  .items {
    display: flex;
    flex-direction: column;
  }
  .items.icons {
    flex-direction: row;
    flex-wrap: wrap;
  }
  .items button {
    position: relative;
    display: flex;
    gap: 9px;
    align-items: center;
    padding: 5px 8px;
    color: var(--chrome-fg);
    font-size: 13px;
    text-align: left;
    background: none;
    border: 0;
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  .items.icons button {
    justify-content: center;
    width: 34px;
  }
  .items button:hover:not(:disabled),
  .items button:focus-visible {
    background: var(--hover);
  }
  .items button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .items .icon {
    width: 18px;
    color: var(--accent-ink);
    font-size: 15px;
    text-align: center;
  }
  .items .label {
    flex: 1;
    white-space: nowrap;
  }
  .items kbd {
    color: var(--chrome-muted);
    font: 11px var(--sans);
  }
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
    color: var(--on-claret);
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
    color: var(--on-claret);
    border-radius: 4px 4px 0 0;
  }
  .tabs .mode {
    margin-left: auto;
    align-self: center;
  }
  .tabs .fold {
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
    grid-auto-columns: max-content;
    align-content: center;
    column-gap: 4px;
  }
  .btns.icons {
    grid-template-rows: minmax(0, 30px);
    grid-auto-columns: 40px;
    justify-content: space-between;
  }
  .fontrow {
    display: flex;
    gap: 4px;
    margin-bottom: 3px;
  }
  .fontrow select {
    height: 26px;
    font-size: 12.5px;
    color: var(--chrome-fg);
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
  }
  .font {
    width: 196px;
  }
  .size {
    width: 58px;
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
    flex-direction: row;
    justify-content: flex-start;
    gap: 7px;
    min-width: 84px;
    max-width: none;
    padding: 0 8px;
  }
  .btn.small .icon {
    width: 18px;
    font-size: 16px;
    text-align: center;
  }
  .btn.small .label {
    font-size: 12px;
    white-space: nowrap;
  }
  @media (max-width: 1520px) {
    .shrinks .btn.small {
      justify-content: center;
      min-width: 34px;
      padding: 0;
    }
    .shrinks .btn.small .label {
      display: none;
    }
  }
  .btn.icons {
    justify-content: center;
    min-width: 0;
    padding: 0;
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
    background: var(--chrome-fg);
    color: var(--chrome);
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
