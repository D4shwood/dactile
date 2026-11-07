<script lang="ts">
  // The viewer's top bar, in TeXstudio's order.
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  import type { Mode } from "./layout";
  import type { Fit } from "./sheets";

  let {
    tr,
    mode = $bindable(),
    fit,
    page,
    pages,
    label,
    canBack,
    canForward,
    wide,
    disabled,
    act,
    go,
    more,
    box = $bindable(),
  }: {
    tr: boolean;
    mode: Mode;
    fit: Fit;
    page: number;
    pages: number;
    /// The page's own label, when the PDF numbers its pages otherwise.
    label: string;
    canBack: boolean;
    canForward: boolean;
    wide: boolean;
    disabled: boolean;
    act: (what: string) => void;
    go: (to: string) => void;
    more: Snippet;
    box?: HTMLInputElement;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let typed = $state("");
  let typing = $state(false);
</script>

{#snippet b(icon: string, title: string, what: string, on = false, off = disabled)}
  <button {title} aria-label={title} aria-pressed={on} class:on disabled={off} onclick={() => act(what)}><Icon name={icon} /></button>
{/snippet}

<div class="bar" role="toolbar" aria-label={l("PDF araçları", "PDF tools")}>
  {@render b("external", l("Dış görüntüleyicide aç", "Open in external viewer"), "external")}
  <i></i>
  <button title={l("Büyüteç: basılı tutunca merceği açar", "Magnify: hold the button for the lens")} aria-label={l("Büyüteç", "Magnify")} aria-pressed={mode === "magnify"} class:on={mode === "magnify"} onclick={() => (mode = "magnify")}><Icon name="lens" /></button>
  <button title={l("Kaydır: sürükleyerek gez", "Scroll: drag the pages")} aria-label={l("Kaydır", "Scroll")} aria-pressed={mode === "scroll"} class:on={mode === "scroll"} onclick={() => (mode = "scroll")}><Icon name="hand" /></button>
  <button title={l("Metin seç", "Select text")} aria-label={l("Metin seç", "Select text")} aria-pressed={mode === "select"} class:on={mode === "select"} onclick={() => (mode = "select")}><Icon name="text" /></button>
  <i></i>
  {@render b("back", l("Geri (Alt+Sol)", "Back (Alt+Left)"), "back", false, !canBack)}
  {@render b("forward", l("İleri (Alt+Sağ)", "Forward (Alt+Right)"), "forward", false, !canForward)}
  <i></i>
  {@render b("first", l("İlk sayfa (Home)", "First page (Home)"), "first", false, disabled || page <= 1)}
  {@render b("previous", l("Önceki sayfa", "Previous page"), "previous", false, disabled || page <= 1)}
  <label class="page" title={l("Sayfaya git (Ctrl+J)", "Go to page (Ctrl+J)")}>
    <input
      bind:this={box}
      aria-label={l("Sayfa", "Page")}
      value={typing ? typed : label || page}
      {disabled}
      onfocus={(e) => {
        typing = true;
        typed = e.currentTarget.value;
        e.currentTarget.select();
      }}
      oninput={(e) => (typed = e.currentTarget.value)}
      onblur={() => (typing = false)}
      onkeydown={(e) => {
        if (e.key !== "Enter") return;
        e.stopPropagation();
        go(typed);
        e.currentTarget.blur();
      }}
    />
    <span>/ {pages}</span>
  </label>
  {@render b("next", l("Sonraki sayfa", "Next page"), "next", false, disabled || page >= pages)}
  {@render b("last", l("Son sayfa (End)", "Last page (End)"), "last", false, disabled || page >= pages)}
  <i></i>
  {@render b("actual", l("Gerçek boyut (Ctrl+1)", "Actual size (Ctrl+1)"), "actual")}
  {@render b("width", l("Genişliğe sığdır (Ctrl+2)", "Fit width (Ctrl+2)"), "fit-width", fit === "width")}
  {@render b("textwidth", l("Metin genişliğine sığdır (Ctrl+4)", "Fit text width (Ctrl+4)"), "fit-text", fit === "text")}
  {@render b("window", l("Pencereye sığdır (Ctrl+3)", "Fit window (Ctrl+3)"), "fit-window", fit === "window")}
  <span class="grow"></span>
  {@render b("folder", l("Dosya gezgininde göster", "Show in file manager"), "reveal")}
  {@render more()}
  <i></i>
  {@render b("hide", l("Araç çubuğunu gizle", "Hide toolbar"), "hide", false, false)}
  {@render b(wide ? "narrow" : "wide", wide ? l("Yazarın yanına dön", "Back beside the writer") : l("Pencereyi kapla", "Fill the window"), "wide", wide, false)}
  {@render b("close", l("Kapat", "Close"), "close", false, false)}
</div>

<style>
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 1px;
    padding: 3px 6px;
    background: var(--chrome);
    color: var(--chrome-fg);
    border-bottom: 1px solid var(--chrome-line);
  }
  i {
    width: 1px;
    height: 18px;
    margin: 0 4px;
    background: var(--chrome-line);
  }
  .grow {
    flex: 1;
  }
  .bar :global(button) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 4px;
    background: none;
    color: inherit;
    cursor: pointer;
  }
  .bar :global(button:hover:not(:disabled)) {
    background: var(--chrome-2);
  }
  .bar :global(button:focus-visible) {
    outline: 2px solid var(--accent-ink);
    outline-offset: -1px;
  }
  .bar :global(button.on) {
    background: var(--chrome-2);
    border-color: var(--chrome-line);
    color: var(--accent-ink);
  }
  .bar :global(button:disabled) {
    opacity: 0.35;
    cursor: default;
  }
  .page {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin: 0 3px;
    font-size: 12px;
    white-space: nowrap;
  }
  .page input {
    width: 3.4em;
    height: 22px;
    box-sizing: border-box;
    padding: 0 4px;
    text-align: right;
    font: inherit;
    color: inherit;
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 3px;
    caret-color: var(--accent-ink);
  }
</style>
