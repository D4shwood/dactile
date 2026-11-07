<script lang="ts">
  // A new character, or one changed: the terminal app's form. The alias
  // follows the name until it is typed by hand.
  import { untrack } from "svelte";
  import { api, type CharacterForm } from "./api";

  let {
    tr,
    text,
    old,
    start,
    done,
    cancel,
  }: {
    tr: boolean;
    /// The play, for suggesting an alias and counting a rename.
    text: string;
    /// The alias being changed; null for a new one.
    old: string | null;
    start: CharacterForm;
    done: (f: CharacterForm) => Promise<string | null>;
    cancel: () => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let f = $state<CharacterForm>(untrack(() => ({ ...start })));
  let touched = untrack(() => old !== null);
  let error = $state("");
  /// A rename asked about once: the second Enter does it.
  let asked = $state(false);

  async function named() {
    if (touched) return;
    f.alias = f.name.trim() ? await api.suggestAlias(text, f.name) : "";
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    error = "";
    const alias = f.alias.trim();
    if (old !== null && alias !== old && !asked) {
      const n = await api.renames(text, old, alias);
      asked = true;
      error = l(`\\${old} → \\${alias}: ${n} yerde değişecek. Onay için yine Kaydet.`, `\\${old} → \\${alias}: ${n} places change. Save again to confirm.`);
      return;
    }
    const why = await done($state.snapshot(f));
    if (why) error = why;
  }
</script>

<div class="veil" role="presentation" onmousedown={(e) => e.target === e.currentTarget && cancel()}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    class="dialog"
    onsubmit={submit}
    onkeydown={(e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        cancel();
      }
    }}
  >
    <h3>{old === null ? l("Yeni karakter", "New character") : `\\${old} ${l("düzenle", "edit")}`}</h3>
    <div class="two">
      <label>
        {l("Ad (replikte görünen)", "Name (before a line)")}
        <!-- svelte-ignore a11y_autofocus -->
        <input bind:value={f.name} oninput={named} autofocus placeholder="SELİN" />
      </label>
      <label class="alias">
        {l("Kısaltma", "Alias")}
        <input
          bind:value={f.alias}
          oninput={() => ((touched = true), (asked = false))}
          placeholder="se"
          spellcheck="false"
        />
      </label>
    </div>
    <label>
      {l("Tam ad (tabloda, isteğe bağlı)", "Full name (in the table, optional)")}
      <input bind:value={f.full} placeholder={l("Selin Yıldız", "Selin Yıldız")} />
    </label>
    <label>
      {l("Açıklama", "Description")}
      <input bind:value={f.desc} placeholder={l("Köy öğretmeni", "The village teacher")} />
    </label>
    <div class="flags">
      <label class="flag"><input type="checkbox" bind:checked={f.in_table} disabled={f.hidden} /> {l("Kişiler tablosunda", "In the cast table")}</label>
      <label class="flag"><input type="checkbox" bind:checked={f.star} /> {l("Yıldızlı (kapakta)", "Starred (on the cover)")}</label>
      <label class="flag"><input type="checkbox" bind:checked={f.hidden} /> {l("Gizli (HERKES gibi)", "Hidden (like ALL)")}</label>
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="row">
      <button class="primary">{old === null ? l("Ekle", "Add") : l("Kaydet", "Save")}</button>
      <button type="button" onclick={cancel}>{l("Vazgeç", "Cancel")}</button>
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
    width: 420px;
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
  .two {
    display: grid;
    grid-template-columns: 1fr 110px;
    gap: 10px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--chrome-muted);
  }
  .alias input {
    font-family: var(--mono);
  }
  input:not([type]) {
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    padding: 6px 8px;
    background: var(--chrome);
    color: var(--chrome-fg);
    font-size: 14px;
  }
  .flags {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .flag {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--chrome-fg);
  }
  .error {
    margin: 0;
    color: var(--accent-ink);
    font-size: 12.5px;
  }
  .row {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  button {
    border: 1px solid var(--chrome-line);
    background: var(--chrome);
    border-radius: 4px;
    padding: 4px 12px;
    cursor: pointer;
  }
  .primary {
    background: var(--claret);
    color: #f6efe4;
    border-color: var(--claret);
  }
</style>
