<script lang="ts">
  // Alt+E: a line or two about the scene, for the board and the page;
  // never printed. Emptied, it goes.
  import { untrack } from "svelte";

  let {
    tr,
    title,
    start,
    done,
    cancel,
  }: {
    tr: boolean;
    title: string;
    start: string;
    done: (text: string) => void;
    cancel: () => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let text = $state(untrack(() => start));
</script>

<div class="ui-veil" role="presentation" onmousedown={(e) => e.target === e.currentTarget && cancel()}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    class="ui-dialog"
    onsubmit={(e) => {
      e.preventDefault();
      done(text);
    }}
    onkeydown={(e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        cancel();
      }
      // Enter keeps it; Shift+Enter breaks the line.
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        done(text);
      }
    }}
  >
    <h3>{title}</h3>
    <p class="hint">{l("Sahnenin özeti: panoda ve sayfada soluk görünür, PDF'e girmez.", "The scene's summary: faint on the board and the page, never printed.")}</p>
    <!-- svelte-ignore a11y_autofocus -->
    <textarea class="ui-field" bind:value={text} rows="3" autofocus placeholder={l("Ne olur bu sahnede?", "What happens in this scene?")}></textarea>
    <div class="ui-actions">
      <button class="ui-btn primary">{l("Kaydet", "Save")}</button>
      <button type="button" class="ui-btn" onclick={cancel}>{l("Vazgeç", "Cancel")}</button>
    </div>
  </form>
</div>

<style>
  textarea {
    font: italic 15px var(--serif);
  }
</style>
