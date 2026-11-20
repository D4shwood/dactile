<script lang="ts">
  // The app's look, applied as it is chosen: on the start screen's settings
  // and in the writer's panel. No setting for the cursor: it always blinks.
  import type { Look } from "./api";
  import { installedFonts } from "./look";
  import ThemeBuilder from "./ThemeBuilder.svelte";

  let {
    tr,
    look,
    set,
  }: { tr: boolean; look: Look; set: <K extends keyof Look>(key: K, value: Look[K]) => void } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  const fonts = $derived(installedFonts(look.page_font));
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
    <ThemeBuilder {tr} {look} {set} />
  </div>

  <label class="check">
    <input type="checkbox" checked={look.character_colors !== false} onchange={(e) => set("character_colors", e.currentTarget.checked)} />
    {l("Her kişinin adı kendi renginde", "Each character's name in a colour of its own")}
  </label>

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
    max-width: 560px;
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
