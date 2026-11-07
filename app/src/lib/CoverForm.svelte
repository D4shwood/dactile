<script lang="ts">
  // Oyun bilgisi: the play's details, and how its cover and cast page look.
  import { api, type CoverForm, type PartLook } from "./api";

  let {
    tr,
    text,
    last,
    tab: firstTab,
    readOnly,
    done,
    source,
    cancel,
  }: {
    tr: boolean;
    text: string;
    last: Record<string, string>;
    tab: "details" | "look";
    readOnly: boolean;
    done: (f: CoverForm) => void;
    source: () => void;
    cancel: () => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let form = $state<CoverForm | null>(null);
  let tab = $state<"details" | "look">("details");
  let chosen = $state(0);
  let error = $state("");

  $effect(() => {
    tab = firstTab;
    api.coverRead(text).then(
      (f) => (form = f),
      (e) => (error = String(e)),
    );
  });

  const FIELDS: [string, string, string][] = [
    ["title", "Başlık", "Title"],
    ["author", "Yazar", "Author"],
    ["editedby", "Düzenleyen", "Edited by"],
    ["draft", "Taslak no", "Draft number"],
    ["copyrightnotice", "Telif", "Copyright"],
    ["address", "Adres", "Address"],
    ["email", "E-posta", "Email"],
    ["telephone", "Telefon", "Telephone"],
  ];
  const PARTS: Record<string, [string, string]> = {
    "coverstyle/title": ["Kapak: başlık", "Cover: title"],
    "coverstyle/author": ["Kapak: yazar", "Cover: author"],
    "coverstyle/editedby": ["Kapak: düzenleyen", "Cover: edited by"],
    "coverstyle/draft": ["Kapak: taslak", "Cover: draft"],
    "coverstyle/starred": ["Kapak: yıldızlı kişiler", "Cover: starred characters"],
    "coverstyle/copyright": ["Kapak: telif", "Cover: copyright"],
    "coverstyle/address": ["Kapak: adres", "Cover: address"],
    "coverstyle/telephone": ["Kapak: telefon", "Cover: telephone"],
    "coverstyle/email": ["Kapak: e-posta", "Cover: email"],
    "caststyle/title": ["Kişiler: başlık", "Cast: title"],
    "caststyle/names": ["Kişiler: adlar", "Cast: names"],
    "caststyle/stars": ["Kişiler: yıldızlı adlar", "Cast: starred names"],
    "caststyle/descriptions": ["Kişiler: açıklamalar", "Cast: descriptions"],
    "caststyle/others": ["Kişiler: diğer kişiler", "Cast: others"],
  };
  const partName = (c: string, p: string) => {
    const n = PARTS[`${c}/${p}`];
    return n ? l(n[0], n[1]) : p;
  };
  const ALIGN: [string, string, string][] = [
    ["left", "Sol", "Left"],
    ["center", "Orta", "Center"],
    ["right", "Sağ", "Right"],
  ];
  const SIZES = ["tiny", "small", "normal", "large", "Large", "LARGE", "huge", "Huge"];
  const SIZE_NAMES: Record<string, [string, string]> = {
    tiny: ["Çok küçük", "Tiny"],
    small: ["Küçük", "Small"],
    normal: ["Normal", "Normal"],
    large: ["Büyük", "Large"],
    Large: ["Daha büyük", "Larger"],
    LARGE: ["Çok büyük", "Very large"],
    huge: ["Kocaman", "Huge"],
    Huge: ["En büyük", "Largest"],
  };
  const STYLES: [string, string, string][] = [
    ["normal", "Düz", "Plain"],
    ["bold", "Kalın", "Bold"],
    ["italic", "İtalik", "Italic"],
    ["bolditalic", "Kalın italik", "Bold italic"],
    ["smallcaps", "Küçük büyük harf", "Small capitals"],
    ["boldsmallcaps", "Kalın küçük büyük harf", "Bold small capitals"],
  ];

  function useLast() {
    if (!form) return;
    for (const [k, v] of Object.entries(last)) if (!form.fields[k]?.trim()) form.fields[k] = v;
  }
  const hasLast = $derived(Object.values(last).some((v) => v.trim()));
  const look = $derived<PartLook | null>(form?.parts[chosen]?.look ?? null);
  function reset() {
    if (form) form.parts[chosen].look = { align: "", size: "", style: "", before: "", after: "" };
  }
</script>

<div class="veil" role="presentation" onmousedown={(e) => e.target === e.currentTarget && cancel()}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    class="dialog"
    onsubmit={(e) => {
      e.preventDefault();
      if (form && !readOnly) done($state.snapshot(form));
    }}
    onkeydown={(e) => {
      e.stopPropagation();
      if (e.key === "Escape") {
        e.preventDefault();
        cancel();
      }
    }}
  >
    <h3>{l("Oyun bilgisi", "Cover details")}</h3>
    <div class="tabs">
      <button type="button" class:on={tab === "details"} onclick={() => (tab = "details")}>{l("Bilgiler", "Details")}</button>
      <button type="button" class:on={tab === "look"} onclick={() => (tab = "look")}>{l("Kapak ve kişiler", "Cover and cast")}</button>
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    {#if form && tab === "details"}
      <div class="grid">
        {#each FIELDS as [cmd, t, e]}
          <label>
            {l(t, e)}
            <input bind:value={form.fields[cmd]} />
          </label>
        {/each}
      </div>
      {#if hasLast}
        <button type="button" class="link" onclick={useLast}>{l("Son girilenleri kullan", "Use the last ones")}</button>
      {/if}
      <label class="tick"><input type="checkbox" bind:checked={form.final_copy} /> {l("Son hâl (final)", "Final copy")}</label>
      <label class="tick"><input type="checkbox" bind:checked={form.own_page} /> {l("Kişiler ayrı sayfada", "Cast on a page of its own")}</label>
      <label class="tick"><input type="checkbox" bind:checked={form.latin} /> {l("Latince başlık: Dramatis Personae", "Latin title: Dramatis Personae")}</label>
      <label class="line">
        {l("Sahne başlıkları", "Scene headings")}
        <select bind:value={form.side}>
          {#each ALIGN as [v, t, e]}<option value={v}>{l(t, e)}</option>{/each}
        </select>
      </label>
    {:else if form && look}
      <label>
        {l("Öğe", "Part")}
        <select bind:value={chosen}>
          {#each form.parts as p, i}<option value={i}>{partName(p.command, p.part)}{p.look.align || p.look.size || p.look.style || p.look.before || p.look.after ? " •" : ""}</option>{/each}
        </select>
      </label>
      <div class="grid">
        <label>
          {l("Hiza", "Alignment")}
          <select bind:value={form.parts[chosen].look.align}>
            <option value="">{l("Sınıfın", "The class's")}</option>
            {#each ALIGN as [v, t, e]}<option value={v}>{l(t, e)}</option>{/each}
          </select>
        </label>
        <label>
          {l("Boyut", "Size")}
          <select bind:value={form.parts[chosen].look.size}>
            <option value="">{l("Sınıfın", "The class's")}</option>
            {#each SIZES as v}<option value={v}>{l(SIZE_NAMES[v][0], SIZE_NAMES[v][1])}</option>{/each}
          </select>
        </label>
        <label>
          {l("Yazı stili", "Font style")}
          <select bind:value={form.parts[chosen].look.style}>
            <option value="">{l("Sınıfın", "The class's")}</option>
            {#each STYLES as [v, t, e]}<option value={v}>{l(t, e)}</option>{/each}
          </select>
        </label>
        <span></span>
        <label>
          {l("Üst boşluk", "Space before")}
          <input bind:value={form.parts[chosen].look.before} placeholder="1cm" />
        </label>
        <label>
          {l("Alt boşluk", "Space after")}
          <input bind:value={form.parts[chosen].look.after} placeholder="0.5cm" />
        </label>
      </div>
      <button type="button" class="link" onclick={reset}>{l("Bu öğeyi sıfırla", "Reset this part")}</button>
    {/if}
    <div class="row">
      <button type="button" class="link" onclick={source}>{l("Kaynakta aç", "Open in source")}</button>
      <span class="grow"></span>
      <button type="button" class="plain" onclick={cancel}>{l("Vazgeç", "Cancel")}</button>
      <button class="primary" disabled={readOnly || !form}>{l("Uygula", "Apply")}</button>
    </div>
  </form>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.18);
  }
  .dialog {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(520px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    overflow: auto;
    padding: 16px 18px;
    background: var(--chrome);
    color: var(--chrome-fg);
    border: 1px solid var(--chrome-line);
    border-radius: 8px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.25);
  }
  h3 {
    margin: 0;
    font: 600 16px var(--serif);
    color: var(--accent-ink);
  }
  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--chrome-line);
  }
  .tabs button {
    padding: 5px 10px;
    font: inherit;
    font-size: 13px;
    color: var(--chrome-muted);
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    cursor: pointer;
  }
  .tabs button.on {
    color: var(--chrome-fg);
    border-bottom-color: var(--accent-ink);
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--chrome-muted);
  }
  label.tick {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--chrome-fg);
  }
  label.line {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
    color: var(--chrome-fg);
  }
  input:not([type="checkbox"]),
  select {
    min-width: 0;
    padding: 5px 7px;
    font: inherit;
    font-size: 13px;
    color: var(--chrome-fg);
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }
  .grow {
    flex: 1;
  }
  .row button:not(.link) {
    padding: 5px 12px;
    font: inherit;
    font-size: 13px;
    color: var(--chrome-fg);
    background: var(--chrome-2);
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    cursor: pointer;
  }
  .row .primary {
    color: var(--chrome);
    background: var(--accent-ink);
    border-color: var(--accent-ink);
  }
  .row .primary:disabled {
    opacity: 0.5;
  }
  .link {
    align-self: flex-start;
    padding: 0;
    font: inherit;
    font-size: 12.5px;
    color: var(--accent-ink);
    background: none;
    border: 0;
    cursor: pointer;
  }
  .error {
    margin: 0;
    color: var(--accent-ink);
  }
  @media (max-width: 480px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
