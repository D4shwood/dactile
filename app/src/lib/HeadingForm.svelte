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

<div class="veil" role="presentation" onmousedown={(e) => e.target === e.currentTarget && cancel()}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    class="dialog"
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
      <label>
        {l("Başlık (isteğe bağlı)", "Title (optional)")}
        <!-- svelte-ignore a11y_autofocus -->
        <input bind:value={a} autofocus placeholder={l("Kar Yağarken", "While It Snows")} />
      </label>
    {:else}
      <label>
        {l("Yer", "Place")}
        <!-- svelte-ignore a11y_autofocus -->
        <input bind:value={a} list="heading-places" autofocus placeholder={l("İç, Değirmen", "Inside, the mill")} />
      </label>
      <label>
        {l("Zaman", "Time")}
        <input bind:value={b} list="heading-times" placeholder={l("Akşam", "Evening")} />
      </label>
    {/if}
    <div class="row">
      <button class="primary">{l("Ekle", "Add")}</button>
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
    width: 340px;
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
    color: var(--claret);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--chrome-muted);
  }
  input {
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    padding: 6px 8px;
    background: var(--chrome);
    color: var(--chrome-fg);
    font-size: 14px;
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
