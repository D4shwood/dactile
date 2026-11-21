<script lang="ts">
  // The themes: the ready ones, and one's own made on a colour wheel. The
  // outer ring turns the accent, and the harmony's other points turn with
  // it; the disc inside picks the neutrals' hue, greyer at the middle and
  // warmer at the edge. What changes shows at once on the whole app; it is
  // kept when the handle is let go.
  import type { Look } from "./api";
  import { HARMONIES, PRESETS, WHO, fit, harmony, hex, palette, parseSeed, randomSeed, ratio, seedText, type Seed } from "./color";
  import { lookSeed } from "./look";
  import { drag } from "./pointer";

  let { tr, look, set }: { tr: boolean; look: Look; set: <K extends keyof Look>(key: K, value: Look[K]) => void } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  const own = $derived(look.theme === "surprise");
  const worn = $derived(lookSeed(look));
  /// The seed being made: while a handle is dragged, it runs ahead of the
  /// settings.
  let draft = $state<Seed | null>(null);
  const seed = $derived(draft ?? worn);
  const colours = $derived(palette(seed));
  const kept = $derived((look.saved_surprises ?? []).filter((s) => parseSeed(s)));
  /// The ready themes' colours never change: made once.
  const READY = PRESETS.map(([name, t, e, s]) => ({ name, t, e, p: palette(s) }));
  const keptColours = $derived(kept.map((t) => ({ t, p: palette(parseSeed(t)!) })));
  let open = $state(false);
  const showBuilder = $derived(own || open);

  /// Shows a seed on the whole app at once, without keeping it.
  function show(s: Seed) {
    draft = s;
    const root = document.documentElement;
    root.dataset.mode = s.mode;
    for (const [k, v] of Object.entries(palette(s))) root.style.setProperty(k, v);
  }
  /// Keeps a seed as the user's own theme.
  function keep(s: Seed) {
    draft = null;
    set("surprise", seedText(s));
    set("theme", "surprise");
  }
  const change = (part: Partial<Seed>) => keep({ ...seed, ...part });

  function wearPreset(name: string) {
    open = false;
    // An accent from older settings would colour the ready theme over.
    if (look.accent && look.accent !== "claret") set("accent", "claret");
    set("theme", name);
  }
  function startOwn() {
    open = true;
    keep(worn);
  }
  function save() {
    const t = seedText(seed);
    if (!kept.includes(t)) set("saved_surprises", [...kept, t].slice(-24));
  }
  const forget = (t: string) =>
    set(
      "saved_surprises",
      kept.filter((x) => x !== t),
    );

  // ---------------------------------------------------------------- wheel

  const SIZE = 216;
  const C = SIZE / 2;
  const RING_OUT = 106;
  const RING_IN = 84;
  const DISC = 70;
  const MOST_WARMTH = 0.08;
  const at = (h: number, r: number) => {
    const a = ((h - 90) * Math.PI) / 180;
    return [C + r * Math.cos(a), C + r * Math.sin(a)];
  };
  /// A slice of the ring or disc from hue `h` to `h + step`.
  function slice(h: number, step: number, r0: number, r1: number) {
    const [a, b, c, d] = [at(h, r1), at(h + step, r1), at(h + step, r0), at(h, r0)];
    return `M${a[0]},${a[1]} A${r1},${r1} 0 0 1 ${b[0]},${b[1]} L${c[0]},${c[1]} ${r0 ? `A${r0},${r0} 0 0 0 ${d[0]},${d[1]}` : ""} Z`;
  }
  const STEP = 6;
  const HUES = Array.from({ length: 360 / STEP }, (_, i) => i * STEP);
  const ring = $derived(HUES.map((h) => hex(fit(seed.mode === "dark" ? 0.72 : 0.6, Math.max(0.06, seed.vivid), h + STEP / 2))));
  const disc = HUES.map((h) => hex(fit(0.8, 0.07, h + STEP / 2)));
  const points = $derived(harmony(seed));
  const baseAt = $derived(at(seed.base, (Math.min(seed.warmth, MOST_WARMTH) / MOST_WARMTH) * DISC));

  let svg: SVGSVGElement | undefined = $state();
  function pointer(e: PointerEvent) {
    const r = svg!.getBoundingClientRect();
    const x = ((e.clientX - r.left) / r.width) * SIZE - C;
    const y = ((e.clientY - r.top) / r.height) * SIZE - C;
    const hue = (((Math.atan2(y, x) * 180) / Math.PI + 90) % 360 + 360) % 360;
    return { hue: Math.round(hue), dist: Math.hypot(x, y) };
  }
  function grab(e: PointerEvent) {
    const first = pointer(e);
    const what = first.dist > DISC + 4 ? "accent" : "base";
    const move = (m: PointerEvent) => {
      const p = pointer(m);
      show(what === "accent" ? { ...seed, accent: p.hue } : { ...seed, base: p.hue, warmth: +(Math.min(1, p.dist / DISC) * MOST_WARMTH).toFixed(3) });
    };
    drag(e, move, () => draft && keep(draft));
    move(e);
    e.preventDefault();
  }
  /// Arrow keys on a handle turn it by 5°.
  function turnKey(e: KeyboardEvent, what: "accent" | "base") {
    const by = e.key === "ArrowRight" || e.key === "ArrowUp" ? 5 : e.key === "ArrowLeft" || e.key === "ArrowDown" ? -5 : 0;
    if (!by) return;
    e.preventDefault();
    change(what === "accent" ? { accent: (seed.accent + by + 360) % 360 } : { base: (seed.base + by + 360) % 360 });
  }

  // -------------------------------------------------------------- sliders

  /// A slider's value shown while it moves, kept when it stops.
  function slide(key: "vivid" | "depth") {
    return {
      oninput: (e: Event) => show({ ...seed, [key]: Number((e.currentTarget as HTMLInputElement).value) }),
      onchange: (e: Event) => keep({ ...seed, [key]: Number((e.currentTarget as HTMLInputElement).value) }),
    };
  }

  // ------------------------------------------------------- what it reads at

  const CHECKS: [string, string, string, string][] = [
    ["--fg", "--page", "Metin", "Text"],
    ["--heading", "--page", "Başlık", "Heading"],
    ["--accent-ink", "--chrome", "Vurgu", "Accent"],
    ["--name-ref", "--page", "Kişi", "Character"],
  ];
  const need = $derived(seed.contrast === "high" ? 7 : 4.5);
  const fmt = (n: number) => n.toFixed(1).replace(".", tr ? "," : ".");
  const who = $derived(look.character_colors !== false);
</script>

<div class="themes">
  <div class="presets" role="radiogroup" aria-label={l("Hazır temalar", "Ready themes")}>
    {#each READY as { name, t, e, p } (name)}
      <button class="card" class:on={look.theme === name} role="radio" aria-checked={look.theme === name} title={l(t, e)} onclick={() => wearPreset(name)}>
        <span class="sheet" style:background={p["--page"]} style:border-color={p["--chrome-line"]}>
          <span class="bar" style:background={p["--chrome"]}><i style:background={p["--claret"]}></i></span>
          <b style:color={p["--heading"]}>Aa</b>
          <span class="dots">{#each [0, 1, 2, 3] as i (i)}<i style:background={p[`--who-${i}`]}></i>{/each}</span>
        </span>
        <small>{l(t, e)}</small>
      </button>
    {/each}
    <button class="card" class:on={own} role="radio" aria-checked={own} title={l("Renk çarkıyla kendi temanı yap", "Make your own on the colour wheel")} onclick={startOwn}>
      <span class="sheet wheel-mini"></span>
      <small>{l("Kendi teman", "Your own")}</small>
    </button>
  </div>

  {#if !showBuilder}
    <button class="ui-btn small" onclick={startOwn}>{l("Bu temadan kendi temanı yap…", "Make your own from this one…")}</button>
  {:else}
    <div class="builder">
      <div class="wheel">
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <svg bind:this={svg} viewBox="0 0 {SIZE} {SIZE}" onpointerdown={grab} aria-label={l("Renk çarkı", "Colour wheel")}>
          {#each HUES as h, i (h)}
            <path d={slice(h, STEP + 0.5, RING_IN, RING_OUT)} fill={ring[i]} />
            <path d={slice(h, STEP + 0.5, 0, DISC)} fill={disc[i]} />
          {/each}
          <defs>
            <radialGradient id="grey">
              <stop offset="0" stop-color={hex(fit(0.8, 0, 0))} stop-opacity="1" />
              <stop offset="1" stop-color={hex(fit(0.8, 0, 0))} stop-opacity="0" />
            </radialGradient>
          </defs>
          <circle cx={C} cy={C} r={DISC} fill="url(#grey)" />
          {#each points.slice(1) as h (h)}
            {@const [x, y] = at(h, (RING_IN + RING_OUT) / 2)}
            <line x1={C} y1={C} x2={x} y2={y} class="spoke" />
            <circle cx={x} cy={y} r="7" class="point" />
          {/each}
          {#if points.length}
            {@const [x, y] = at(seed.accent, (RING_IN + RING_OUT) / 2)}
            <line x1={C} y1={C} x2={x} y2={y} class="spoke main" />
            <circle
              cx={x}
              cy={y}
              r="10"
              class="handle"
              fill={colours["--claret"]}
              tabindex="0"
              role="slider"
              aria-label={l("Vurgu tonu", "Accent hue")}
              aria-valuemin="0"
              aria-valuemax="359"
              aria-valuenow={seed.accent}
              onkeydown={(e) => turnKey(e, "accent")}
            />
          {/if}
          <circle
            cx={baseAt[0]}
            cy={baseAt[1]}
            r="8"
            class="handle"
            fill={colours["--page"]}
            tabindex="0"
            role="slider"
            aria-label={l("Zemin tonu", "Base hue")}
            aria-valuemin="0"
            aria-valuemax="359"
            aria-valuenow={seed.base}
            onkeydown={(e) => turnKey(e, "base")}
          />
        </svg>
        <p class="hint">{l("Halka: vurgu. İç daire: zemin; ortası gri, kenarı renkli.", "Ring: the accent. Inside: the base; grey in the middle, tinted at the edge.")}</p>
      </div>

      <div class="controls">
        <div class="ui-seg" role="radiogroup" aria-label={l("Uyum", "Harmony")}>
          {#each HARMONIES as [h, t, e] (h)}
            <button class:on={seed.harmony === h} role="radio" aria-checked={seed.harmony === h} onclick={() => change({ harmony: h })}>{l(t, e)}</button>
          {/each}
        </div>
        <div class="row">
          <div class="ui-seg two" role="radiogroup" aria-label={l("Açık ya da koyu", "Light or dark")}>
            <button class:on={seed.mode === "light"} role="radio" aria-checked={seed.mode === "light"} onclick={() => change({ mode: "light" })}>{l("Açık", "Light")}</button>
            <button class:on={seed.mode === "dark"} role="radio" aria-checked={seed.mode === "dark"} onclick={() => change({ mode: "dark" })}>{l("Koyu", "Dark")}</button>
          </div>
          <label class="ui-tick">
            <input type="checkbox" checked={seed.contrast === "high"} onchange={(e) => change({ contrast: e.currentTarget.checked ? "high" : "normal" })} />
            {l("Yüksek kontrast", "High contrast")}
          </label>
        </div>
        <label class="slider">
          <span>{l("Canlılık", "Vividness")}</span>
          <input type="range" min="0.04" max="0.2" step="0.005" value={seed.vivid} {...slide("vivid")} />
        </label>
        <label class="slider">
          <span>{l("Derinlik", "Depth")}</span>
          <input type="range" min="0" max="0.1" step="0.005" value={seed.depth} {...slide("depth")} />
        </label>
        <div class="row">
          <button class="ui-btn small" title={l("Uyumlu, rastgele bir tema", "A random theme in harmony")} onclick={() => keep(randomSeed(seed))}>🎲 {l("Zar at", "Roll")}</button>
          <button class="ui-btn small" disabled={kept.includes(seedText(seed))} onclick={save}>{l("Kaydet", "Save")}</button>
        </div>
      </div>

      <div class="preview" style:background={colours["--page"]} style:color={colours["--fg"]} style:border-color={colours["--chrome-line"]}>
        <div class="pchrome" style:background={colours["--chrome"]} style:color={colours["--chrome-fg"]} style:border-color={colours["--chrome-line"]}>
          <span class="pbtn" style:background={colours["--claret"]}>{l("Derle", "Build")}</span>
          <span style:color={colours["--accent-ink"]}>{l("Giriş", "Home")}</span>
          <span style:color={colours["--chrome-muted"]}>{l("Görünüm", "View")}</span>
        </div>
        <div class="ppage">
          <b class="phead" style:color={colours["--heading"]}>I. {l("PERDE", "ACT")}</b>
          <p><span class="pname" style:color={who ? colours["--who-0"] : colours["--star"]}>ADA</span> {l("Kar hâlâ yağıyor mu?", "Is it still snowing?")}</p>
          <p><span class="pname" style:color={who ? colours["--who-1"] : colours["--name-ref"]}>BARAN</span> {l("Durdu.", "It stopped.")} <i style:color={colours["--muted"]}>({l("pencereye bakar", "looks out")})</i></p>
          <p><span class="pname" style:color={who ? colours["--who-2"] : colours["--name-ref"]}>CEM</span> <span class="psel" style:background={colours["--selection"]}>{l("Yine başlar.", "It'll start again.")}</span></p>
        </div>
        <div class="checks" style:background={colours["--chrome"]} style:color={colours["--chrome-muted"]}>
          {#each CHECKS as [f, b, t, e] (f)}
            {@const r = ratio(colours[f], colours[b])}
            <span class:low={r < need - 0.05} title={l(`${t}: zemininde ${fmt(r)}:1 (en az ${need}:1)`, `${e}: ${fmt(r)}:1 on its ground (at least ${need}:1)`)}>{l(t, e)} {fmt(r)}</span>
          {/each}
        </div>
        {#if who}
          <div class="whos" style:background={colours["--page"]}>
            {#each Array.from({ length: WHO }, (_, i) => i) as i (i)}<i style:background={colours[`--who-${i}`]}></i>{/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  {#if kept.length}
    <div class="kept">
      <span class="ui-eyebrow">{l("Kaydedilen temalar", "Saved themes")}</span>
      <div class="swatches">
        {#each keptColours as { t, p } (t)}
          <span class="saved">
            <button
              class="mini"
              class:on={own && look.surprise === t}
              title={l("Bu temaya geç", "Wear this theme")}
              aria-label={l("Bu temaya geç", "Wear this theme")}
              style:background={p["--page"]}
              style:border-color={p["--chrome-line"]}
              onclick={() => keep(parseSeed(t)!)}
            >
              <i style:background={p["--claret"]}></i><i style:background={p["--who-0"]}></i><i style:background={p["--who-1"]}></i>
            </button>
            <button class="forget" title={l("Kayıttan sil", "Forget it")} aria-label={l("Kayıttan sil", "Forget it")} onclick={() => forget(t)}>×</button>
          </span>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .themes {
    display: grid;
    gap: 12px;
    min-width: 0;
    container-type: inline-size;
  }
  .presets {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(72px, 1fr));
    gap: 8px;
  }
  .card {
    display: grid;
    justify-items: center;
    gap: 4px;
    min-width: 0;
    padding: 4px;
    color: var(--chrome-fg);
    background: none;
    border: 2px solid transparent;
    border-radius: var(--r-md);
    cursor: pointer;
  }
  .card:hover {
    background: var(--hover);
  }
  .card.on {
    border-color: var(--accent-ink);
  }
  .card small {
    max-width: 100%;
    font-size: 11px;
    line-height: 1.2;
    text-align: center;
    overflow-wrap: anywhere;
  }
  .sheet {
    position: relative;
    display: grid;
    place-items: center;
    width: 100%;
    max-width: 64px;
    height: 46px;
    overflow: hidden;
    border: 1px solid;
    border-radius: var(--r-sm);
  }
  .sheet .bar {
    position: absolute;
    inset: 0 0 auto;
    display: flex;
    justify-content: flex-end;
    height: 9px;
    padding: 2px 3px;
  }
  .sheet .bar i {
    width: 10px;
    border-radius: 2px;
  }
  .sheet b {
    margin-top: 6px;
    font: 600 16px var(--serif);
  }
  .dots {
    position: absolute;
    bottom: 3px;
    display: flex;
    gap: 2px;
  }
  .dots i,
  .whos i,
  .mini i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  .wheel-mini {
    border-color: var(--chrome-line);
    background: conic-gradient(from 0deg, #c96a7a, #c98a4a, #a8a040, #5aa070, #3fa0a8, #5a80c8, #9a6ac8, #c96a7a);
  }
  .builder {
    display: grid;
    grid-template-columns: minmax(0, 220px) minmax(0, 1fr);
    gap: 14px;
    align-items: start;
    padding: 12px;
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: var(--r-lg);
  }
  .wheel svg {
    display: block;
    width: 100%;
    touch-action: none;
    cursor: crosshair;
  }
  .hint {
    margin: 4px 0 0;
    color: var(--chrome-muted);
    font-size: 11px;
    text-align: center;
  }
  .spoke {
    stroke: var(--chrome-fg);
    stroke-opacity: 0.25;
    stroke-dasharray: 3 3;
  }
  .spoke.main {
    stroke-opacity: 0.45;
    stroke-dasharray: none;
  }
  .point {
    fill: none;
    stroke: var(--chrome-fg);
    stroke-width: 2;
  }
  .handle {
    stroke: var(--chrome-fg);
    stroke-width: 2.5;
    cursor: grab;
  }
  .handle:focus-visible {
    outline: none;
    stroke: var(--accent-ink);
    stroke-width: 4;
  }
  .controls {
    display: grid;
    gap: 10px;
    min-width: 0;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .slider {
    display: grid;
    gap: 2px;
    color: var(--chrome-muted);
    font-size: 12px;
  }
  .slider input {
    width: 100%;
    accent-color: var(--accent-ink);
  }
  .preview {
    grid-column: 1 / -1;
    overflow: hidden;
    border: 1px solid;
    border-radius: var(--r-md);
  }
  .pchrome {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 6px 10px;
    font: 12px var(--sans);
    border-bottom: 1px solid;
  }
  .pbtn {
    padding: 2px 8px;
    color: #fbf6ee;
    border-radius: var(--r-sm);
  }
  .ppage {
    padding: 10px 16px 6px;
    font: 15px/1.5 var(--serif);
  }
  .ppage p {
    margin: 0 0 2px;
  }
  .phead {
    display: block;
    margin-bottom: 4px;
    font: 600 13px var(--sans);
    letter-spacing: 0.08em;
  }
  .pname {
    display: inline-block;
    width: 4.5em;
    font: 600 12px var(--sans);
    letter-spacing: 0.04em;
  }
  .psel {
    padding: 0 2px;
  }
  .checks {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    padding: 5px 10px;
    font: 11px var(--sans);
  }
  .checks .low {
    color: var(--warn);
    font-weight: 700;
  }
  .whos {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 4px 10px 8px;
  }
  .whos i {
    width: 12px;
    height: 12px;
  }
  .kept {
    display: grid;
    gap: 6px;
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .saved {
    position: relative;
  }
  .mini {
    display: flex;
    gap: 3px;
    align-items: center;
    justify-content: center;
    width: 46px;
    height: 30px;
    border: 2px solid;
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  .mini.on {
    outline: 2px solid var(--accent-ink);
    outline-offset: 1px;
  }
  .forget {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 16px;
    height: 16px;
    padding: 0;
    color: var(--chrome-fg);
    font-size: 11px;
    line-height: 1;
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
  @container (max-width: 420px) {
    .builder {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
