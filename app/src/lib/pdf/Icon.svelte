<script lang="ts">
  // The viewer's bar icons, drawn in the bar's own colour.
  const PATHS: Record<string, string> = {
    external: "M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5",
    lens: "M10.5 3a7.5 7.5 0 1 0 0 15 7.5 7.5 0 0 0 0-15zM16 16l5 5",
    hand: "M8 12V6a1.5 1.5 0 0 1 3 0v5m0-1V4.5a1.5 1.5 0 0 1 3 0V11m0-4.5a1.5 1.5 0 0 1 3 0V12m0-3a1.5 1.5 0 0 1 3 0v6a6 6 0 0 1-6 6h-2a6 6 0 0 1-5-3l-3-5a1.5 1.5 0 0 1 2.5-1.5L8 14",
    text: "M9 4h6M9 20h6M12 4v16",
    back: "M10 6l-6 6 6 6M4 12h16",
    forward: "M14 6l6 6-6 6M20 12H4",
    first: "M6 5v14M18 6l-7 6 7 6",
    previous: "M15 6l-7 6 7 6",
    next: "M9 6l7 6-7 6",
    last: "M18 5v14M6 6l7 6-7 6",
    actual: "M7 8v9M5.5 9.5L7 8M17 8v9M15.5 9.5L17 8M12 10v.5M12 14v.5",
    width: "M4 5v14M20 5v14M7 12h10M10 9l-3 3 3 3M14 9l3 3-3 3",
    textwidth: "M4 5v14M20 5v14M8 8h8M8 12h8M8 16h5",
    window: "M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM9 8H8v1M15 8h1v1M9 16H8v-1M15 16h1v-1",
    folder: "M3 7a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z",
    more: "M5 12h.01M12 12h.01M19 12h.01",
    hide: "M5 14l7-7 7 7M5 19h14",
    show: "M5 9l7 7 7-7M5 4h14",
    wide: "M4 9V4h5M20 15v5h-5M4 4l6 6M20 20l-6-6",
    narrow: "M10 5v5H5M14 19v-5h5M10 10L4 4M14 14l6 6",
    close: "M6 6l12 12M18 6L6 18",
    left: "M4 5h16v14H4zM9 5v14",
    right: "M4 5h16v14H4zM15 5v14",
    minus: "M6 12h12",
    plus: "M6 12h12M12 6v12",
  };
  let { name }: { name: string } = $props();
</script>

<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
  <path d={PATHS[name]} fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
</svg>
