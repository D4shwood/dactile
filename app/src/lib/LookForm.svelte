<script lang="ts">
  // The app's look, applied as it is chosen: on the start screen's settings
  // and in the writer's panel. No setting for the cursor: it always blinks.
  import type { Look } from "./api";
  import { ACCENTS, THEMES, installedFonts, newSurprise, surpriseColours } from "./look";

  let {
    tr,
    look,
    set,
  }: { tr: boolean; look: Look; set: <K extends keyof Look>(key: K, value: Look[K]) => void } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  const fonts = $derived(installedFonts(look.page_font));
  const dark = $derived(look.theme === "night" || look.theme === "dark");
  const kept = $derived(look.saved_surprises ?? []);
  const isSurprise = $derived(look.theme === "surprise");
  function keep() {
    if (!kept.includes(look.surprise)) set("saved_surprises", [...kept, look.surprise].slice(-24));
  }
  function forget(s: string) {
    set("saved_surprises", kept.filter((x) => x !== s));
  }
  function wear(s: string) {
    set("surprise", s);
    set("theme", "surprise");
  }

  function surprise() {
    set("surprise", newSurprise(look.surprise));
    set("theme", "surprise");
  }
  const SPACINGS: [number, string, string][] = [
    [130, "Sık", "Tight"],
    [155, "Olağan", "Normal"],
    [180, "Geniş", "Loose"],
    [200, "Çift", "Double"],
  ];
  const WIDTHS: [number, string, string][] = [
    [36, "Dar", "Narrow"],
    [44, "Olağan", "Normal"],
    [52, "Geniş", "Wide"],
    [60, "Çok geniş", "Very wide"],
  ];
</script>

<div class="look">
  <div class="field">
    <span class="name">{l("Tema", "Theme")}</span>
    <div class="swatches">
      {#each THEMES as [id, t, e, page, ink] (id)}
        <button class="theme" class:on={look.theme === id} aria-pressed={look.theme === id} title={l(t, e)} onclick={() => set("theme", id)}>
          <span class="sheet" style:background={page} style:color={ink}>Aa</span>
          <small>{l(t, e)}</small>
        </button>
      {/each}
      <button
        class="theme"
        class:on={look.theme === "surprise"}
        aria-pressed={look.theme === "surprise"}
        title={l("Her basışta yeni renkler", "New colours at every press")}
        onclick={surprise}
      >
        <span class="sheet dice">?</span>
        <small>{l("Sürpriz yap", "Surprise me")}</small>
      </button>
    </div>
    {#if isSurprise && look.surprise && !kept.includes(look.surprise)}
      <button class="keep" onclick={keep}>{l("Bu görünümü kaydet", "Save this look")}</button>
    {/if}
  </div>

  {#if kept.length}
    <div class="field">
      <span class="name">{l("Kaydedilen görünümler", "Saved looks")}</span>
      <div class="swatches">
        {#each kept as s (s)}
          {@const c = surpriseColours(s)}
          <span class="saved">
            <button class="theme" class:on={isSurprise && look.surprise === s} aria-pressed={isSurprise && look.surprise === s} title={l("Bu görünüme geç", "Wear this look")} onclick={() => wear(s)}>
              <span class="sheet" style:background={c["--page"]} style:color={c["--accent-ink"]}>Aa</span>
            </button>
            <button class="forget" title={l("Kayıttan sil", "Forget it")} aria-label={l("Kayıttan sil", "Forget it")} onclick={() => forget(s)}>×</button>
          </span>
        {/each}
      </div>
    </div>
  {/if}

  {#if look.theme !== "surprise"}
  <div class="field">
    <span class="name">{l("Vurgu rengi", "Accent colour")}</span>
    <div class="swatches">
      {#each ACCENTS as [id, t, e, light, night] (id)}
        <button class="accent" class:on={look.accent === id} aria-pressed={look.accent === id} aria-label={l(t, e)} title={l(t, e)} onclick={() => set("accent", id)}>
          <span style:background={dark ? night : light}></span>
        </button>
      {/each}
    </div>
  </div>
  {/if}

  <label class="field">
    <span class="name">{l("Sayfanın yazı tipi", "Page font")}</span>
    <select value={look.page_font} onchange={(e) => set("page_font", e.currentTarget.value)}>
      <option value="">{l("Dactile'ın yazı tipi", "Dactile's own")}</option>
      {#each fonts as f (f)}<option value={f}>{f}</option>{/each}
    </select>
  </label>

  <label class="field">
    <span class="name">{l("Yazı boyutu", "Text size")} <em>{look.page_size} px</em></span>
    <input type="range" min="13" max="24" value={look.page_size} oninput={(e) => set("page_size", Number(e.currentTarget.value))} />
  </label>

  <label class="field">
    <span class="name">{l("Sayfa yakınlığı", "Page zoom")} <em>%{look.page_zoom}</em></span>
    <input type="range" min="50" max="200" step="10" value={look.page_zoom} oninput={(e) => set("page_zoom", Number(e.currentTarget.value))} />
  </label>

  <label class="field">
    <span class="name">{l("Satır aralığı", "Line spacing")}</span>
    <select value={look.line_spacing} onchange={(e) => set("line_spacing", Number(e.currentTarget.value))}>
      {#each SPACINGS as [n, t, e] (n)}<option value={n}>{l(t, e)}</option>{/each}
    </select>
  </label>

  <label class="field">
    <span class="name">{l("Sayfa genişliği", "Page width")}</span>
    <select value={look.page_width} onchange={(e) => set("page_width", Number(e.currentTarget.value))}>
      {#each WIDTHS as [n, t, e] (n)}<option value={n}>{l(t, e)}</option>{/each}
    </select>
  </label>

  <label class="check">
    <input type="checkbox" checked={look.animations} onchange={(e) => set("animations", e.currentTarget.checked)} />
    {l("Animasyonlar", "Animations")}
  </label>
</div>

<style>
  .look {
    display: grid;
    gap: 14px;
    max-width: 480px;
    min-width: 0;
  }
  .field {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .name {
    font-size: 13px;
    color: var(--chrome-muted);
  }
  .name em {
    font-style: normal;
    color: var(--chrome-fg);
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .keep {
    align-self: flex-start;
    margin-top: 6px;
    padding: 3px 9px;
    font-size: 12px;
    color: inherit;
    background: none;
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    cursor: pointer;
  }
  .saved {
    position: relative;
  }
  .forget {
    position: absolute;
    top: -5px;
    right: -5px;
    width: 16px;
    height: 16px;
    padding: 0;
    font-size: 11px;
    line-height: 1;
    color: var(--chrome-fg);
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 50%;
    cursor: pointer;
    opacity: 0;
  }
  .saved:hover .forget,
  .forget:focus {
    opacity: 1;
  }
  .theme {
    display: grid;
    justify-items: center;
    gap: 3px;
    width: 76px;
    padding: 4px;
    border: 2px solid transparent;
    border-radius: 6px;
    background: none;
    color: var(--chrome-fg);
    cursor: pointer;
  }
  .theme small {
    font-size: 11px;
    text-align: center;
  }
  .sheet {
    display: grid;
    place-items: center;
    width: 52px;
    height: 40px;
    border: 1px solid var(--chrome-line);
    border-radius: 3px;
    font: 18px var(--serif);
  }
  .dice {
    background: conic-gradient(#d97757, #d9a95a, #7cc4a0, #8ab4e8, #c79ad6, #d97757);
    color: #1f1e1d;
    font-weight: 700;
  }
  .accent {
    display: grid;
    padding: 3px;
    border: 2px solid transparent;
    border-radius: 50%;
    background: none;
    cursor: pointer;
  }
  .accent span {
    width: 24px;
    height: 24px;
    border-radius: 50%;
  }
  .theme.on,
  .accent.on {
    border-color: var(--accent-ink);
  }
  select {
    min-width: 0;
    max-width: 100%;
    padding: 4px 6px;
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    background: var(--chrome);
    color: var(--chrome-fg);
  }
  input[type="range"] {
    width: 100%;
    accent-color: var(--accent-ink);
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 13px;
  }
</style>
