<script lang="ts">
  // The cover beside the page: the play's details, and how each part of its
  // cover and cast page looks. The page shows the cover as it prints and
  // marks the part picked here; what changes here shows there at once, and
  // the writer puts it in the play a moment later.
  import type { CoverForm, PartLook } from "./api";

  let {
    tr,
    form = $bindable(),
    part = $bindable(),
    last,
    readOnly,
    changed,
    pdf,
    source,
    close,
  }: {
    tr: boolean;
    form: CoverForm;
    /// The part picked: `coverstyle/title` and the like.
    part: string | null;
    last: Record<string, string>;
    readOnly: boolean;
    changed: () => void;
    pdf: () => void;
    source: () => void;
    close: () => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);

  /// Each field, and the part of the cover it is.
  const FIELDS: [field: string, part: string, tr: string, en: string][] = [
    ["title", "title", "Başlık", "Title"],
    ["author", "author", "Yazar", "Author"],
    ["editedby", "editedby", "Düzenleyen", "Edited by"],
    ["draft", "draft", "Taslak no", "Draft number"],
    ["copyrightnotice", "copyright", "Telif", "Copyright"],
    ["address", "address", "Adres", "Address"],
    ["email", "email", "E-posta", "Email"],
    ["telephone", "telephone", "Telefon", "Telephone"],
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
  const ALIGN: [string, string, string][] = [
    ["", "Sınıfın", "The class's"],
    ["left", "Sol", "Left"],
    ["center", "Orta", "Center"],
    ["right", "Sağ", "Right"],
  ];
  const SIZES: [string, string, string][] = [
    ["tiny", "Çok küçük", "Tiny"],
    ["small", "Küçük", "Small"],
    ["normal", "Normal", "Normal"],
    ["large", "Büyük", "Large"],
    ["Large", "Daha büyük", "Larger"],
    ["LARGE", "Çok büyük", "Very large"],
    ["huge", "Kocaman", "Huge"],
    ["Huge", "En büyük", "Largest"],
  ];
  const STYLES: [string, string, string][] = [
    ["normal", "Düz", "Plain"],
    ["bold", "Kalın", "Bold"],
    ["italic", "İtalik", "Italic"],
    ["bolditalic", "Kalın italik", "Bold italic"],
    ["smallcaps", "Küçük büyük harf", "Small capitals"],
    ["boldsmallcaps", "Kalın küçük büyük harf", "Bold small capitals"],
  ];

  const name = (key: string) => {
    const n = PARTS[key];
    return n ? l(n[0], n[1]) : key;
  };
  const key = (p: { command: string; part: string }) => `${p.command}/${p.part}`;
  const at = $derived(Math.max(0, form.parts.findIndex((p) => key(p) === part)));
  const look = $derived<PartLook | undefined>(form.parts[at]?.look);
  const touched = (k: PartLook) => !!(k.align || k.size || k.style || k.before || k.after);

  const hasLast = $derived(Object.entries(last).some(([k, v]) => v.trim() && !form.fields[k]?.trim()));
  function useLast() {
    for (const [k, v] of Object.entries(last)) if (!form.fields[k]?.trim()) form.fields[k] = v;
    changed();
  }
  function reset() {
    form.parts[at].look = { align: "", size: "", style: "", before: "", after: "" };
    changed();
  }
</script>

<aside class="cover ui-panel" aria-label={l("Kapak", "Cover")}>
  <div class="ui-panel-head">
    <strong>{l("Kapak", "Cover")}</strong>
    <span class="grow"></span>
    <button class="ui-btn ghost icon" title={l("Kapat (Esc)", "Close (Esc)")} onclick={close}>✕</button>
  </div>
  <!-- Every change shows on the page at once: inputs and choices alike. -->
  <fieldset class="list" disabled={readOnly} oninput={changed} onchange={changed}>
    <section>
      <h4 class="ui-eyebrow">{l("Bilgiler", "Details")}</h4>
      {#each FIELDS as [f, p, t, e]}
        <label class="ui-label" class:on={part === `coverstyle/${p}`}>
          {l(t, e)}
          <input class="ui-field" bind:value={form.fields[f]} onfocus={() => (part = `coverstyle/${p}`)} />
        </label>
      {/each}
      {#if hasLast}
        <button type="button" class="ui-link" onclick={useLast}>{l("Son girilenleri kullan", "Use the last ones")}</button>
      {/if}
      <label class="ui-tick"><input type="checkbox" bind:checked={form.final_copy} /> {l("Son hâl (taslak yazmaz)", "Final copy (no draft line)")}</label>
      <label class="ui-tick"><input type="checkbox" bind:checked={form.own_page} /> {l("Kişiler ayrı sayfada", "Cast on a page of its own")}</label>
      <label class="ui-tick"><input type="checkbox" bind:checked={form.contents} /> {l("Kapaktan sonra içindekiler", "Contents after the cover")}</label>
      <label class="ui-tick"><input type="checkbox" bind:checked={form.latin} /> {l("Latince başlık: Dramatis Personae", "Latin title: Dramatis Personae")}</label>
      <label class="ui-label">
        {l("Sahne başlıkları", "Scene headings")}
        <select class="ui-field" bind:value={form.side}>
          {#each ALIGN.slice(1) as [v, t, e]}<option value={v}>{l(t, e)}</option>{/each}
        </select>
      </label>
    </section>

    {#if look}
      <section>
        <h4 class="ui-eyebrow">{l("Görünüş", "Look")}</h4>
        <label class="ui-label">
          {l("Öğe", "Part")}
          <select class="ui-field" value={key(form.parts[at])} onchange={(e) => (part = e.currentTarget.value)}>
            {#each form.parts as p}<option value={key(p)}>{name(key(p))}{touched(p.look) ? " •" : ""}</option>{/each}
          </select>
        </label>
        <div class="ui-seg" role="group" aria-label={l("Hiza", "Alignment")}>
          {#each ALIGN as [v, t, e]}
            <button type="button" class:on={look.align === v} onclick={() => ((look.align = v), changed())}>{l(t, e)}</button>
          {/each}
        </div>
        <div class="two">
          <label class="ui-label">
            {l("Boyut", "Size")}
            <select class="ui-field" bind:value={look.size}>
              <option value="">{l("Sınıfın", "The class's")}</option>
              {#each SIZES as [v, t, e]}<option value={v}>{l(t, e)}</option>{/each}
            </select>
          </label>
          <label class="ui-label">
            {l("Yazı stili", "Font style")}
            <select class="ui-field" bind:value={look.style}>
              <option value="">{l("Sınıfın", "The class's")}</option>
              {#each STYLES as [v, t, e]}<option value={v}>{l(t, e)}</option>{/each}
            </select>
          </label>
          <label class="ui-label">
            {l("Üst boşluk", "Space before")}
            <input class="ui-field" bind:value={look.before} placeholder="1cm" />
          </label>
          <label class="ui-label">
            {l("Alt boşluk", "Space after")}
            <input class="ui-field" bind:value={look.after} placeholder="0.5cm" />
          </label>
        </div>
        {#if touched(look)}
          <button type="button" class="ui-link" onclick={reset}>{l("Bu öğeyi sınıfınkine döndür", "Back to the class's look")}</button>
        {/if}
        {#if part?.startsWith("caststyle/")}
          <p class="muted">{l("Kişiler sayfasının görünüşü PDF'te görünür.", "The cast page's look shows in the PDF.")}</p>
        {/if}
      </section>
    {/if}
  </fieldset>
  <div class="ui-panel-foot">
    <button type="button" class="ui-link" onclick={source}>{l("Kaynakta aç", "Open in source")}</button>
    <span class="grow"></span>
    <button type="button" class="ui-btn small" onclick={pdf}>{l("PDF'te gör", "See it in the PDF")}</button>
  </div>
</aside>

<style>
  .grow {
    flex: 1;
  }
  .list {
    flex: 1;
    min-width: 0;
    min-height: 0;
    margin: 0;
    padding: 0;
    overflow-x: hidden;
    overflow-y: auto;
    border: 0;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 12px 16px;
    border-bottom: 1px solid var(--chrome-line);
  }
  label.on {
    color: var(--accent-ink);
  }
  .two {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 8px;
  }
  .muted {
    margin: 0;
    color: var(--chrome-muted);
    font-size: 12px;
  }
</style>
