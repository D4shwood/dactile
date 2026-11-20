<script lang="ts">
  // In the browser, for the site's owner: who has an account, who is
  // invited, and links for those mail doesn't reach. The desktop never
  // shows this.
  import { ask } from "./ask.svelte";
  import { api, type People } from "./api";

  let { tr }: { tr: boolean } = $props();
  const l = (t: string, e: string) => (tr ? t : e);

  let all = $state<People | null>(null);
  let error = $state("");
  /// The link last made, what for, and for whom; and whether its mail went.
  let link = $state<{ key: string; url: string; why: "davet" | "parola"; sent: boolean } | null>(null);
  let copied = $state(false);
  let mail = $state("");
  let name = $state("");
  /// Asked once more before someone is taken off.

  async function load() {
    try {
      all = await api.people();
    } catch (e) {
      error = String(e);
    }
  }
  void load();

  async function act(f: () => Promise<void>) {
    error = "";
    copied = false;
    try {
      await f();
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  const invite = () =>
    act(async () => {
      const made = await api.personInvite(mail.trim(), name.trim());
      link = { key: mail.trim().toLowerCase(), url: location.origin + made.path, why: "davet", sent: made.sent };
      mail = name = "";
    });

  const reset = (id: string) =>
    act(async () => {
      const made = await api.personReset(id);
      link = { key: id, url: location.origin + made.path, why: "parola", sent: false };
    });

  const uninvite = (to: string) =>
    act(async () => {
      await api.personUninvite(to);
      if (link?.key === to) link = null;
    });

  async function remove(id: string, e: MouseEvent) {
    const who = all?.people.find((p) => p.id === id)?.name || id;
    const yes = await ask({
      title: l(`${who} listeden çıkarılsın mı?`, `Remove ${who} from the list?`),
      detail: l("Oyunları yerinde kalır.", "Their plays stay."),
      yes: l("Çıkar", "Remove"),
      no: l("Vazgeç", "Cancel"),
      danger: true,
      at: e,
    });
    if (!yes) return;
    await act(async () => {
      await api.personRemove(id);
      if (link?.key === id) link = null;
    });
  }

  async function copy() {
    if (!link) return;
    await navigator.clipboard.writeText(link.url).catch(() => {});
    copied = true;
  }

  const day = (secs: number) =>
    new Date(secs * 1000).toLocaleDateString(tr ? "tr-TR" : "en-GB", { day: "numeric", month: "long" });
</script>

{#snippet made(key: string)}
  {#if link?.key === key}
    <tr>
      <td colspan="4" class="link">
        <input readonly value={link.url} onfocus={(e) => e.currentTarget.select()} />
        <button onclick={copy}>{copied ? l("Kopyalandı", "Copied") : l("Kopyala", "Copy")}</button>
        <p class="muted">
          {#if link.why === "davet"}
            {link.sent
              ? l("Davet postayla gitti. Bağlantıyı istersen kendin de gönderebilirsin.", "The invitation went by mail. You can also send the link yourself.")
              : l("Posta gitmedi: bağlantıyı kendin gönder. Yedi gün geçerli, bir kez çalışır.", "No mail went: send the link yourself. Good once, for seven days.")}
          {:else}
            {l(
              "Bir kez çalışır, yedi gün geçerli. Açan kişi bu hesabın parolasını belirler ve girer: yalnızca ona gönder.",
              "Good once, for seven days. Whoever opens it sets this account's password and comes in: send it to them alone.",
            )}
          {/if}
        </p>
      </td>
    </tr>
  {/if}
{/snippet}

<div class="people">
  {#if all}
    <table>
      <tbody>
        {#each all.people as p (p.id)}
          <tr>
            <td class="name">{p.name} <span class="muted">· {p.id}</span></td>
            <td class="muted">
              {p.mail}
              {#if p.mail && !p.checked}<span class="warn">({l("doğrulanmadı", "not checked")})</span>{/if}
            </td>
            <td class="muted">
              {p.role === "owner" ? l("sahip", "owner") : ""}
              {#if !p.password}{l("parolasız", "no password")}{/if}
            </td>
            <td class="does">
              <button onclick={() => reset(p.id)}>{l("Parola bağlantısı", "Password link")}</button>
              {#if p.role !== "owner"}
                <button onclick={(e) => remove(p.id, e)}>{l("Çıkar", "Remove")}</button>
              {/if}
            </td>
          </tr>
          {@render made(p.id)}
        {/each}
        {#each all.invited as i (i.mail)}
          <tr class="invited">
            <td class="name">{i.name || "—"}</td>
            <td class="muted">{i.mail}</td>
            <td class="muted">{l(`davet bekliyor · son gün ${day(i.ends)}`, `invited · until ${day(i.ends)}`)}</td>
            <td class="does">
              <button onclick={() => uninvite(i.mail)}>{l("Geri al", "Take back")}</button>
            </td>
          </tr>
          {@render made(i.mail)}
        {/each}
      </tbody>
    </table>

    <form
      onsubmit={(e) => {
        e.preventDefault();
        void invite();
      }}
    >
      <label>{l("E-posta", "E-mail")} <input bind:value={mail} type="email" required /></label>
      <label>{l("Ad (isteğe bağlı)", "Name (optional)")} <input bind:value={name} /></label>
      <button class="primary">{l("Davet et", "Invite")}</button>
    </form>
    <p class="muted">
      {all.sends
        ? l(
            "Davet edilen kişiye bir bağlantı gider; kullanıcı adını ve parolasını kendisi seçer.",
            "The person gets a link by mail and chooses their own user name and password.",
          )
        : l(
            "Bu site posta göndermiyor: davet bağlantısını kendin iletirsin.",
            "This site sends no mail: you pass the invitation's link on yourself.",
          )}
    </p>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .people {
    max-width: 860px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 18px;
  }
  td {
    padding: 7px 10px 7px 0;
    border-bottom: 1px solid var(--line, rgba(0, 0, 0, 0.12));
    vertical-align: middle;
  }
  .name {
    font-weight: 600;
  }
  .invited .name {
    font-weight: 400;
    font-style: italic;
  }
  .muted {
    opacity: 0.7;
    font-size: 13px;
    font-weight: 400;
  }
  .warn {
    color: var(--warn);
  }
  .does {
    text-align: right;
    white-space: nowrap;
  }
  .link input {
    width: min(100%, 520px);
    font: inherit;
    font-size: 13px;
    padding: 5px 8px;
  }
  .link p {
    margin: 6px 0 0;
  }
  form {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: end;
  }
  label {
    display: grid;
    gap: 4px;
    font-size: 13px;
  }
  input {
    font: inherit;
    padding: 5px 8px;
  }
  .error {
    color: var(--danger);
  }
</style>
