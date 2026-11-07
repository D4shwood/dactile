<script lang="ts">
  // F9: who plays whom, over the page as the board is. Three views: the
  // casting (one of the play's lineups, "A", "B") with an actor for each
  // role, the scene/role table, and the numbers. Kept beside the play,
  // never printed; out of it come the table and each actor's copy.
  import { onMount } from "svelte";
  import { api, type CastingEdit, type CopyKind, type Roles } from "./api";

  let {
    tr,
    text,
    readOnly,
    go,
    memorize,
    say,
    close,
  }: {
    tr: boolean;
    /// The play as it is now.
    text: () => string;
    readOnly: boolean;
    /// To a scene's heading.
    go: (at: number) => void;
    memorize: (alias: string) => void;
    say: (text: string, warn?: boolean) => void;
    close: () => void;
  } = $props();

  const l = (t: string, e: string) => (tr ? t : e);
  let tab = $state<"casting" | "table" | "numbers">("casting");
  let r = $state<Roles | null>(null);
  /// A casting being named: new, or the one in use renamed.
  let naming = $state<"new" | "rename" | null>(null);
  let name = $state("");
  let deleting = $state(false);
  /// The PDF or the folder last made, to open.
  let out = $state<string | null>(null);
  let busy = $state("");

  async function load() {
    r = await api.castingRoles(text());
  }

  async function edit(e: CastingEdit) {
    try {
      await api.castingEdit(e);
      await load();
      return true;
    } catch (err) {
      say(String(err), true);
      return false;
    }
  }

  async function nameIt() {
    const kind = naming!;
    if (await edit({ kind, name })) naming = null;
  }

  async function copies(kind: CopyKind) {
    busy = l("Oyuncu kopyaları derleniyor…", "Building the actors' copies…");
    try {
      const [dir, n] = await api.castingCopies(text(), kind);
      out = dir;
      say(l(`${n} PDF hazır: ${dir}`, `${n} PDFs ready: ${dir}`));
    } catch (e) {
      say(l(`Olmadı: ${e}`, `Failed: ${e}`), true);
    }
    busy = "";
  }

  async function tablePdf() {
    busy = l("Tablo derleniyor…", "Building the table…");
    try {
      out = await api.castingPdf(text());
      say(l(`Tablo PDF'i hazır: ${out}`, `The table's PDF is ready: ${out}`));
    } catch (e) {
      say(l(`Olmadı: ${e}`, `Failed: ${e}`), true);
    }
    busy = "";
  }

  async function csv() {
    try {
      const p = await api.castingCsv(text());
      say(l(`Tablo yazıldı: ${p}`, `The table is written: ${p}`));
    } catch (e) {
      say(String(e), true);
    }
  }

  const most = $derived(r ? Math.max(1, ...r.roles.map((x) => x.words)) : 1);
  const byWords = $derived(r ? [...r.roles].sort((a, b) => b.words - a.words) : []);
  onMount(load);
</script>

<div class="casting">
  <div class="head">
    <strong>{l("Dağıtım", "Casting")}</strong>
    {#if r}
      <span class="castings">
        {#if r.castings.length === 0}
          <span class="muted">{l("henüz dağıtım yok", "no casting yet")}</span>
        {/if}
        {#each r.castings as c, i (i)}
          <button class="chip" class:on={i === r.casting} onclick={() => edit({ kind: "pick", casting: i })}>{c || "?"}</button>
        {/each}
      </span>
      {#if !readOnly}
        {#if naming}
          <form class="name" onsubmit={(e) => (e.preventDefault(), void nameIt())}>
            <!-- svelte-ignore a11y_autofocus -->
            <input bind:value={name} autofocus placeholder={l("A, B, prömiyer…", "A, B, premiere…")} />
            <button class="primary">{naming === "new" ? l("Ekle", "Add") : l("Adlandır", "Rename")}</button>
            <button type="button" onclick={() => (naming = null)}>{l("Vazgeç", "Cancel")}</button>
          </form>
        {:else if deleting}
          <span class="warn">{l(`"${r.castings[r.casting]}" silinsin mi?`, `Delete "${r.castings[r.casting]}"?`)}</span>
          <button class="primary" onclick={() => ((deleting = false), void edit({ kind: "delete" }))}>{l("Sil", "Delete")}</button>
          <button onclick={() => (deleting = false)}>{l("Vazgeç", "Cancel")}</button>
        {:else}
          <button
            title={r.castings.length ? l("Şimdikinin kopyası olarak başlar", "Starts as a copy of this one") : ""}
            onclick={() => ((naming = "new"), (name = ""))}>+ {l("Yeni dağıtım", "New casting")}</button
          >
          <button onclick={() => ((naming = "rename"), (name = r?.castings[r.casting] ?? ""))}>{l("Adı", "Name")}</button>
          {#if r.castings.length}
            <button onclick={() => (deleting = true)}>{l("Sil", "Delete")}</button>
          {/if}
        {/if}
      {/if}
    {/if}
    <span class="grow"></span>
    <span class="tabs">
      <button class:on={tab === "casting"} onclick={() => (tab = "casting")}>{l("Roller", "Roles")}</button>
      <button class:on={tab === "table"} onclick={() => (tab = "table")}>{l("Tablo", "Table")}</button>
      <button class:on={tab === "numbers"} onclick={() => (tab = "numbers")}>{l("Sayılar", "Numbers")}</button>
    </span>
    <button onclick={close}>{l("Kapat", "Close")} · F9</button>
  </div>

  {#if busy}
    <div class="busy">{busy}</div>
  {/if}

  <div class="body">
    {#if !r}
      <p class="muted">…</p>
    {:else if r.roles.length === 0}
      <p class="muted">{l("Henüz karakter yok: Ctrl+O ile ekle.", "No characters yet: Ctrl+O adds one.")}</p>
    {:else if tab === "casting"}
      <table class="roles">
        <thead>
          <tr>
            <th>{l("Rol", "Role")}</th>
            <th>{l("Oyuncu", "Actor")} <span class="muted">{r.castings.length ? `(${r.castings[r.casting]})` : ""}</span></th>
            <th></th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each r.roles as role (role.alias)}
            <tr>
              <td class="role">{role.name}</td>
              <td>
                <input
                  class="actor"
                  list="dactile-actors"
                  value={role.actor ?? ""}
                  placeholder="—"
                  disabled={readOnly}
                  onchange={(e) => edit({ kind: "actor", alias: role.alias, actor: e.currentTarget.value })}
                  onkeydown={(e) => e.key === "Enter" && e.currentTarget.blur()}
                />
              </td>
              <td class="muted">
                {l(
                  `${role.lines} replik · ${role.words} kelime · ≈ ${role.time} · ${role.scenes} sahne`,
                  `${role.lines} lines · ${role.words} words · ≈ ${role.time} · ${role.scenes} scenes`,
                )}
              </td>
              <td><button class="small" onclick={() => memorize(role.alias)}>{l("Ezber", "Memorize")}</button></td>
            </tr>
          {/each}
        </tbody>
      </table>
      <datalist id="dactile-actors">
        {#each r.known as a (a)}<option value={a}></option>{/each}
      </datalist>
      <div class="out">
        <span>{l("Oyuncu kopyaları", "Actors' copies")}:</span>
        <button disabled={!!busy} onclick={() => copies("full")} title={l("Ötekilerin replikleri soluk", "The others' lines faint")}>{l("Tam", "Full")}</button>
        <button disabled={!!busy} onclick={() => copies("rehearsal")} title={l("Ötekilerin replikleri yerine not boşluğu", "Room for notes where the others' lines were")}>{l("Prova", "Rehearsal")}</button>
        <button disabled={!!busy} onclick={() => copies("light")} title={l("Ötekilerin ilk bir iki kelimesi", "The others' first word or two")}>{l("Prova, hafif", "Rehearsal, light")}</button>
        {#if out}<button class="link" onclick={() => api.launch(out!)}>{l("Aç", "Open")}</button>{/if}
        <span class="muted">{l("Her oyuncuya bir PDF: kendi replikleri kalın, girişleri işaretli.", "A PDF for each actor: their lines bold, their entrances marked.")}</span>
      </div>
    {:else if tab === "table"}
      <div class="scroll">
        <table class="grid">
          <thead>
            <tr>
              <th></th>
              {#each r.roles as role (role.alias)}
                <th class="col"><span>{role.name}</span>{#if role.actor}<small>{role.actor}</small>{/if}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each r.scenes as s, i (i)}
              <tr onclick={() => go(s.at)} title={l("Sahneye git", "Go to the scene")}>
                <td class="scene"><b>{s.name}</b> {s.place}</td>
                {#each s.here as h, j (j)}
                  <td class="mark" class:speaks={h === 2} class:on={h === 1}>{h === 2 ? "●" : h === 1 ? "○" : "·"}</td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <div class="out">
        <span class="muted">● {l("konuşur", "speaks")} · ○ {l("sahnede", "on stage")}</span>
        <span class="grow"></span>
        <button onclick={csv}>CSV</button>
        <button disabled={!!busy} onclick={tablePdf}>PDF</button>
        {#if out}<button class="link" onclick={() => api.launch(out!)}>{l("Aç", "Open")}</button>{/if}
      </div>
    {:else}
      <div class="numbers">
        <h4>{l("Oyun", "The play")} <span class="muted">{r.play}</span></h4>
        <h4>{l("Roller", "Roles")}</h4>
        <table>
          <tbody>
            {#each byWords as role (role.alias)}
              <tr>
                <td class="role">{role.name}</td>
                <td class="num">{role.lines} {l("replik", "lines")}</td>
                <td class="num">{role.words} {l("kelime", "words")}</td>
                <td class="num">{role.time}</td>
                <td class="num">{tr ? `%${role.share}` : `${role.share}%`}</td>
                <td class="bar"><span style:width="{(role.words / most) * 100}%"></span></td>
              </tr>
            {/each}
          </tbody>
        </table>
        <h4>{l("Sahneler", "Scenes")}</h4>
        <table>
          <tbody>
            {#each r.scenes as s, i (i)}
              {#if s.act}<tr><td colspan="4" class="act">{s.act}</td></tr>{/if}
              <tr class="go" onclick={() => go(s.at)}>
                <td>{s.name}</td>
                <td>{s.place}</td>
                <td class="num">{s.time}</td>
                <td class="num">{s.lines} {l("replik", "lines")}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="wpm">
          {l("Dakikada kelime", "Words a minute")}:
          <input
            type="number"
            min="30"
            max="400"
            value={r.wpm}
            disabled={readOnly}
            onchange={(e) => edit({ kind: "wpm", wpm: Number(e.currentTarget.value) })}
          />
          <span class="muted">{l("süreler buna göre", "the times follow it")}</span>
        </p>
      </div>
    {/if}
  </div>
</div>

<style>
  .casting {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--desk);
    color: var(--chrome-fg);
  }
  .head {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    padding: 8px 14px;
    background: var(--chrome);
    border-bottom: 1px solid var(--chrome-line);
  }
  .head strong {
    font: 600 15px var(--serif);
    color: var(--accent-ink);
    margin-right: 6px;
  }
  button {
    border: 1px solid var(--chrome-line);
    background: var(--chrome);
    color: var(--chrome-fg);
    border-radius: 4px;
    padding: 3px 10px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .primary,
  .chip.on,
  .tabs .on {
    background: var(--claret);
    color: #f6efe4;
    border-color: var(--claret);
  }
  .castings {
    display: flex;
    gap: 4px;
  }
  .tabs {
    display: flex;
  }
  .tabs button {
    border-radius: 0;
  }
  .tabs button:first-child {
    border-radius: 4px 0 0 4px;
  }
  .tabs button:last-child {
    border-radius: 0 4px 4px 0;
  }
  .name {
    display: flex;
    gap: 4px;
  }
  input {
    border: 1px solid var(--chrome-line);
    border-radius: 4px;
    padding: 3px 6px;
    background: var(--chrome);
    color: var(--chrome-fg);
  }
  .warn {
    color: var(--accent-ink);
  }
  .muted {
    color: var(--chrome-muted);
    font-size: 12px;
  }
  .grow {
    flex: 1;
  }
  .busy {
    padding: 6px 14px;
    background: color-mix(in srgb, var(--claret) 15%, var(--chrome));
    font-size: 13px;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 14px 20px;
    overflow: auto;
    background: var(--chrome);
  }
  table {
    border-collapse: collapse;
  }
  td,
  th {
    padding: 4px 10px;
    text-align: left;
  }
  th {
    font-weight: 600;
    font-size: 12px;
    color: var(--chrome-muted);
  }
  .role {
    font-weight: 700;
    letter-spacing: 0.03em;
  }
  .roles tbody tr:hover,
  .grid tbody tr:hover,
  .go:hover {
    background: color-mix(in srgb, var(--claret) 8%, transparent);
  }
  .actor {
    width: 220px;
  }
  .small {
    font-size: 12px;
    padding: 1px 8px;
  }
  .out {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--chrome-line);
  }
  .link {
    border-color: var(--claret);
    color: var(--accent-ink);
  }
  .scroll {
    overflow: auto;
    flex: 1;
    min-height: 0;
  }
  .grid {
    cursor: pointer;
  }
  .grid .col {
    text-align: center;
    font-size: 12px;
    color: var(--chrome-fg);
  }
  .grid .col span,
  .grid .col small {
    display: block;
  }
  .grid .col small {
    font-weight: 400;
    color: var(--chrome-muted);
  }
  .grid .scene {
    white-space: nowrap;
  }
  .mark {
    text-align: center;
    color: var(--chrome-muted);
    opacity: 0.5;
  }
  .mark.on {
    opacity: 1;
  }
  .mark.speaks {
    color: var(--accent-ink);
    opacity: 1;
  }
  .numbers h4 {
    margin: 14px 0 6px;
    font: 600 15px var(--serif);
    color: var(--accent-ink);
  }
  .numbers h4:first-child {
    margin-top: 0;
  }
  .numbers .muted {
    font: 13px var(--sans, inherit);
    margin-left: 8px;
  }
  .num {
    text-align: right;
    color: var(--chrome-muted);
    white-space: nowrap;
  }
  .bar {
    width: 220px;
  }
  .bar span {
    display: block;
    height: 10px;
    background: var(--claret);
    border-radius: 2px;
  }
  .act {
    padding-top: 10px;
    font-weight: 700;
  }
  .go {
    cursor: pointer;
  }
  .wpm {
    margin-top: 16px;
  }
  .wpm input {
    width: 70px;
  }
</style>
