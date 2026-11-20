<script lang="ts">
  // In the browser: the projects the person is in. Anyone may start one and
  // is its admin; the admin puts people in with a role, says how its plays
  // are written, and names it. The desktop never shows this.
  import { api, type Mode, type Project, type Role } from "./api";

  let { tr, changed }: { tr: boolean; changed: () => void } = $props();
  const l = (t: string, e: string) => (tr ? t : e);

  let all = $state<Project[] | null>(null);
  let everyone = $state<{ id: string; name: string }[]>([]);
  let error = $state("");
  let name = $state("");
  /// Per project: who is being put in, and as what.
  let adding = $state<Record<string, { who: string; role: Role }>>({});
  /// A project being renamed, and its new name.
  let renaming = $state<{ id: string; name: string } | null>(null);
  /// Asked once more before leaving.
  let leaving = $state("");

  const ROLES: [Role, string, string][] = [
    ["admin", "yönetici", "admin"],
    ["writer", "yazar", "writer"],
    ["reader", "okur", "reader"],
  ];
  const roleName = (r: Role | null) => {
    const found = ROLES.find(([k]) => k === r);
    return found ? l(found[1], found[2]) : "";
  };

  async function load() {
    try {
      const [mine, people] = await Promise.all([api.projects(), api.projectPeople()]);
      for (const p of mine) adding[p.id] ??= { who: "", role: "writer" };
      [all, everyone] = [mine, people];
    } catch (e) {
      error = String(e);
    }
  }
  void load();

  async function act(f: () => Promise<unknown>) {
    error = "";
    try {
      await f();
      await load();
      changed();
    } catch (e) {
      error = String(e);
    }
  }

  const create = () =>
    act(async () => {
      await api.projectCreate(name.trim());
      name = "";
    });

  const outside = (p: Project) => everyone.filter((x) => !p.members.some((m) => m.id === x.id));

  const putIn = (p: Project) =>
    act(async () => {
      const a = adding[p.id];
      if (!a?.who) return;
      await api.projectMember(p.id, a.who, a.role);
      adding[p.id] = { who: "", role: "writer" };
    });

  const setMode = (p: Project, mode: Mode) => act(() => api.projectMode(p.id, mode));

  async function leave(p: Project) {
    if (leaving !== p.id) {
      leaving = p.id;
      return;
    }
    leaving = "";
    await act(() => api.projectLeave(p.id));
  }
</script>

<div class="projects">
  <h1>{l("Projeler", "Projects")}</h1>
  <p class="muted">
    {l(
      "Bir projenin oyunlarını üyeleri görür: yazarlar yazar, okurlar okur. Projeyi açan yöneticisidir; üyeleri ve rollerini o belirler.",
      "A project's plays are its members': writers write them, readers read them. Whoever starts it is its admin, who says who is in and as what.",
    )}
  </p>

  <form
    class="new"
    onsubmit={(e) => {
      e.preventDefault();
      void create();
    }}
  >
    <input bind:value={name} placeholder={l("Projenin adı", "The project's name")} aria-label={l("Projenin adı", "The project's name")} />
    <button class="primary" disabled={!name.trim()}>{l("Yeni proje", "New project")}</button>
  </form>
  {#if error}<p class="error">{error}</p>{/if}

  {#if all && !all.length}
    <p class="muted">{l("Henüz bir projede değilsin.", "You aren't in a project yet.")}</p>
  {/if}

  {#each all ?? [] as p (p.id)}
    {@const admin = p.role === "admin"}
    <section class="project">
      <header>
        {#if renaming?.id === p.id}
          <form
            class="rename"
            onsubmit={(e) => {
              e.preventDefault();
              const n = renaming!.name;
              renaming = null;
              void act(() => api.projectRename(p.id, n));
            }}
          >
            <input bind:value={renaming.name} aria-label={l("Yeni ad", "New name")} />
            <button class="primary">{l("Değiştir", "Rename")}</button>
            <button type="button" class="plain" onclick={() => (renaming = null)}>{l("Vazgeç", "Cancel")}</button>
          </form>
        {:else}
          <h2>{p.name}</h2>
          <span class="badge">{roleName(p.role)}</span>
          {#if admin}
            <button class="plain" onclick={() => (renaming = { id: p.id, name: p.name })}>{l("Adını değiştir", "Rename")}</button>
          {/if}
        {/if}
      </header>

      <div class="mode" role="radiogroup" aria-label={l("Yazma biçimi", "How it is written")}>
        <span>{l("Yazma biçimi", "Writing")}:</span>
        {#each [["turns", "Sırayla", "In turns"], ["together", "Birlikte", "Together"]] as [m, t, e]}
          <button
            role="radio"
            aria-checked={p.mode === m}
            class:on={p.mode === m}
            disabled={!admin}
            onclick={() => p.mode !== m && setMode(p, m as Mode)}>{l(t, e)}</button
          >
        {/each}
        <small class="muted">
          {p.mode === "turns"
            ? l("Bir oyunu aynı anda bir kişi yazar; açan öbürleri o kapatana dek okur.", "One writes a play at a time; the others read it until they close it.")
            : l("Birlikte yazma henüz gelmedi: şimdilik sırayla yazılır.", "Writing together isn't here yet: for now it goes in turns.")}
        </small>
      </div>

      <table>
        <tbody>
          {#each p.members as m (m.id)}
            <tr>
              <td class="name">{m.name} <span class="muted">· {m.id}</span></td>
              <td>
                {#if admin}
                  <select
                    value={m.role}
                    aria-label={l(`${m.name}: rolü`, `${m.name}: role`)}
                    onchange={(e) => act(() => api.projectMember(p.id, m.id, e.currentTarget.value as Role))}
                  >
                    {#each ROLES as [k, t, e]}<option value={k}>{l(t, e)}</option>{/each}
                  </select>
                {:else}
                  <span class="muted">{roleName(m.role)}</span>
                {/if}
              </td>
              <td class="does">
                {#if admin}
                  <button class="plain" onclick={() => act(() => api.projectMember(p.id, m.id, null))}>{l("Çıkar", "Remove")}</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>

      {#if admin}
        {@const a = adding[p.id]}
        {#if a && outside(p).length}
          <form
            class="add"
            onsubmit={(e) => {
              e.preventDefault();
              void putIn(p);
            }}
          >
            <select bind:value={a.who} aria-label={l("Kişi", "Person")}>
              <option value="">{l("Kişi seç…", "Choose someone…")}</option>
              {#each outside(p) as x (x.id)}<option value={x.id}>{x.name} · {x.id}</option>{/each}
            </select>
            <select bind:value={a.role} aria-label={l("Rol", "Role")}>
              {#each ROLES as [k, t, e]}<option value={k}>{l(t, e)}</option>{/each}
            </select>
            <button class="primary" disabled={!a.who}>{l("Ekle", "Add")}</button>
          </form>
        {:else}
          <p class="muted">{l("Sitedeki herkes bu projede.", "Everyone on the site is in it.")}</p>
        {/if}
      {/if}

      <button class="plain leave" class:danger={leaving === p.id} onclick={() => leave(p)}>
        {leaving === p.id ? l("Emin misin? Oyunları projede kalır.", "Sure? Its plays stay in it.") : l("Projeden ayrıl", "Leave the project")}
      </button>
    </section>
  {/each}
</div>

<style>
  .projects {
    max-width: 860px;
  }
  .new,
  .add,
  .rename {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    margin: 12px 0;
  }
  input,
  select {
    font: inherit;
    padding: 5px 8px;
  }
  .new input {
    min-width: 260px;
  }
  .project {
    border-top: 1px solid var(--line, rgba(0, 0, 0, 0.12));
    padding: 16px 0 8px;
    margin-top: 12px;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  h2 {
    margin: 0;
  }
  .badge {
    font-size: 12px;
    padding: 1px 8px;
    border-radius: 9px;
    border: 1px solid currentColor;
    opacity: 0.75;
  }
  .mode {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin: 12px 0;
    font-size: 14px;
  }
  .mode button {
    font: inherit;
    padding: 3px 12px;
    border: 1px solid var(--line, rgba(0, 0, 0, 0.25));
    background: none;
    border-radius: 6px;
  }
  .mode button.on {
    background: var(--claret);
    border-color: var(--claret);
    color: var(--on-claret);
  }
  .mode small {
    flex-basis: 100%;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  td {
    padding: 6px 10px 6px 0;
    border-bottom: 1px solid var(--line, rgba(0, 0, 0, 0.12));
  }
  .name {
    font-weight: 600;
  }
  .does {
    text-align: right;
  }
  .muted {
    opacity: 0.7;
    font-size: 13px;
    font-weight: 400;
  }
  .leave {
    margin-top: 8px;
  }
  .danger {
    color: var(--danger);
    font-weight: 600;
  }
  .error {
    color: var(--danger);
  }
</style>
