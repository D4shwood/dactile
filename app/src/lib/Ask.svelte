<script lang="ts">
  // The question and the notice of ask.svelte.ts, beside where the user
  // pressed or what has focus, kept on the screen.
  import { tick } from "svelte";
  import { shown } from "./ask.svelte";

  const q = $derived(shown.question);
  const n = $derived(shown.notice);

  /// A box's place: its corner at the point, turned back where it would
  /// leave the screen.
  function spot(at: { x: number; y: number } | null, w: number, h: number) {
    if (!at || !w) return { left: innerWidth / 2 - w / 2, top: innerHeight / 2 - h / 2 };
    const left = Math.max(8, Math.min(at.x, innerWidth - w - 8));
    const top = at.y + h + 8 > innerHeight ? Math.max(8, at.y - h - 8) : at.y + 8;
    return { left, top };
  }
  let qw = $state(0);
  let qh = $state(0);
  let nw = $state(0);
  let nh = $state(0);
  const qAt = $derived(q ? spot(q.at, qw, qh) : null);
  const nAt = $derived(n ? spot(n.at, nw, nh) : null);

  let first: HTMLButtonElement | undefined = $state();
  let back: Element | null = null;
  $effect(() => {
    if (!q) return;
    back = document.activeElement;
    void tick().then(() => first?.focus());
    return () => {
      if (back instanceof HTMLElement && document.contains(back)) back.focus();
    };
  });

  /// The notice's action: taken before the notice goes, which takes it too.
  function act() {
    const run = shown.notice?.action?.run;
    shown.notice = null;
    run?.();
  }

  function key(e: KeyboardEvent) {
    if (!q) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      q.answer(false);
    }
  }
</script>

<svelte:window onkeydown={key} />

{#if q}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim" onpointerdown={() => q.answer(false)}></div>
  <div
    class="ask"
    role="alertdialog"
    aria-modal="true"
    aria-label={q.title}
    bind:clientWidth={qw}
    bind:clientHeight={qh}
    style:left="{qAt?.left ?? 0}px"
    style:top="{qAt?.top ?? 0}px"
    style:visibility={qw ? "visible" : "hidden"}
  >
    <strong>{q.title}</strong>
    {#if q.detail}<p>{q.detail}</p>{/if}
    <div class="row">
      {#if q.danger}
        <button class="ui-btn small" bind:this={first} onclick={() => q.answer(false)}>{q.no}</button>
        <button class="ui-btn small danger" onclick={() => q.answer(true)}>{q.yes}</button>
      {:else}
        <button class="ui-btn small" onclick={() => q.answer(false)}>{q.no}</button>
        <button class="ui-btn small primary" bind:this={first} onclick={() => q.answer(true)}>{q.yes}</button>
      {/if}
    </div>
  </div>
{/if}

{#if n}
  {#key n.id}
    <div
      class="notice"
      class:warn={n.warn}
      role="status"
      bind:clientWidth={nw}
      bind:clientHeight={nh}
      style:left="{nAt?.left ?? 0}px"
      style:top="{nAt?.top ?? 0}px"
      style:visibility={nw ? "visible" : "hidden"}
    >
      <span>{n.text}</span>
      {#if n.action}
        <button class="ui-btn small" onclick={act}>{n.action.label}</button>
      {/if}
      <button class="x" aria-label="✕" onclick={() => (shown.notice = null)}>✕</button>
    </div>
  {/key}
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 90;
  }
  .ask,
  .notice {
    position: fixed;
    z-index: 91;
    max-width: min(340px, calc(100vw - 16px));
    color: var(--chrome-fg);
    font: 13px var(--sans);
    overflow-wrap: anywhere;
    background: var(--raised);
    border: 1px solid var(--chrome-line);
    border-radius: var(--r-md);
    box-shadow: var(--sh-2);
  }
  .ask {
    display: grid;
    gap: 6px;
    padding: 12px 14px 10px;
  }
  .ask p {
    margin: 0;
    color: var(--chrome-muted);
  }
  .row {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
    margin-top: 4px;
  }
  .danger {
    color: var(--danger);
    border-color: var(--danger);
  }
  .notice {
    z-index: 92;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 7px 8px 7px 12px;
    animation: rise 0.15s ease-out;
  }
  .notice.warn {
    border-left: 3px solid var(--warn);
  }
  .notice span {
    flex: 1;
    min-width: 0;
  }
  .x {
    padding: 0 4px;
    color: var(--chrome-muted);
    background: none;
    border: 0;
    cursor: pointer;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
</style>
