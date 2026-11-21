<script lang="ts">
  // A new scene (place and time) or act (a title), as the terminal app's
  // forms ask for them. Both fields may stay empty.
  let {
    tr,
    act,
    places,
    times,
    done,
    cancel,
  }: {
    tr: boolean;
    act: boolean;
    places: string[];
    times: string[];
    done: (a: string, b: string) => void;
    cancel: () => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let a = $state("");
  let b = $state("");
</script>

<datalist id="heading-places">
  {#each places as p}<option value={p}></option>{/each}
</datalist>
<datalist id="heading-times">
  {#each times as t}<option value={t}></option>{/each}
</datalist>

<div class="ui-veil" role="presentation" onmousedown={(e) => e.target === e.currentTarget && cancel()}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    class="ui-dialog"
    onsubmit={(e) => {
      e.preventDefault();
      done(a, b);
    }}
    onkeydown={(e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        cancel();
      }
    }}
  >
    <h3>{act ? l("Yeni perde", "New act") : l("Yeni sahne", "New scene")}</h3>
    {#if act}
      <label class="ui-label">
        {l("Başlık (isteğe bağlı)", "Title (optional)")}
        <!-- svelte-ignore a11y_autofocus -->
        <input class="ui-field" bind:value={a} autofocus placeholder={l("Kar Yağarken", "While It Snows")} />
      </label>
    {:else}
      <label class="ui-label">
        {l("Yer", "Place")}
        <!-- svelte-ignore a11y_autofocus -->
        <input class="ui-field" bind:value={a} list="heading-places" autofocus placeholder={l("İç, Değirmen", "Inside, the mill")} />
      </label>
      <label class="ui-label">
        {l("Zaman", "Time")}
        <input class="ui-field" bind:value={b} list="heading-times" placeholder={l("Akşam", "Evening")} />
      </label>
    {/if}
    <div class="ui-actions">
      <button class="ui-btn primary">{l("Ekle", "Add")}</button>
      <button type="button" class="ui-btn" onclick={cancel}>{l("Vazgeç", "Cancel")}</button>
    </div>
  </form>
</div>
