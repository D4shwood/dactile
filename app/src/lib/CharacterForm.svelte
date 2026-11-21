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

<div class="ui-veil" role="presentation" onmousedown={(e) => e.target === e.currentTarget && cancel()}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    class="ui-dialog"
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
      <label class="ui-label">
        {l("Ad (replikte görünen)", "Name (before a line)")}
        <!-- svelte-ignore a11y_autofocus -->
        <input class="ui-field" bind:value={f.name} oninput={named} autofocus placeholder="SELİN" />
      </label>
      <label class="ui-label">
        {l("Kısaltma", "Alias")}
        <input
          class="ui-field alias"
          bind:value={f.alias}
          oninput={() => ((touched = true), (asked = false))}
          placeholder="se"
          spellcheck="false"
        />
      </label>
    </div>
    <label class="ui-label">
      {l("Tam ad (tabloda, isteğe bağlı)", "Full name (in the table, optional)")}
      <input class="ui-field" bind:value={f.full} placeholder={l("Selin Yıldız", "Selin Yıldız")} />
    </label>
    <label class="ui-label">
      {l("Açıklama", "Description")}
      <input class="ui-field" bind:value={f.desc} placeholder={l("Köy öğretmeni", "The village teacher")} />
    </label>
    <div class="flags">
      <label class="ui-tick"><input type="checkbox" bind:checked={f.in_table} disabled={f.hidden} /> {l("Kişiler tablosunda", "In the cast table")}</label>
      <label class="ui-tick"><input type="checkbox" bind:checked={f.star} /> {l("Yıldızlı (kapakta)", "Starred (on the cover)")}</label>
      <label class="ui-tick"><input type="checkbox" bind:checked={f.hidden} /> {l("Gizli (HERKES gibi)", "Hidden (like ALL)")}</label>
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="ui-actions">
      <button class="ui-btn primary">{old === null ? l("Ekle", "Add") : l("Kaydet", "Save")}</button>
      <button type="button" class="ui-btn" onclick={cancel}>{l("Vazgeç", "Cancel")}</button>
    </div>
  </form>
</div>

<style>
  .two {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 110px;
    gap: 10px;
  }
  .alias {
    font-family: var(--mono);
  }
  .flags {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
