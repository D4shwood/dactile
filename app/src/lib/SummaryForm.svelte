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

<div class="veil" role="presentation" onmousedown={(e) => e.target === e.currentTarget && cancel()}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    class="dialog"
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
    <textarea bind:value={text} rows="3" autofocus placeholder={l("Ne olur bu sahnede?", "What happens in this scene?")}></textarea>
    <div class="row">
      <button class="primary">{l("Kaydet", "Save")}</button>
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
    gap: 8px;
    width: 440px;
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
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--chrome-muted);
  }
  textarea {
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    padding: 8px;
    background: var(--chrome);
    color: var(--chrome-fg);
    font: italic 15px var(--serif);
    resize: vertical;
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
