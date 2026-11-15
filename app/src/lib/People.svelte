<script lang="ts">
  // In the browser, for the site's owner: who may come in, and a link for
  // each. The desktop never shows this.
  import { api, type Person } from "./api";

  let { tr }: { tr: boolean } = $props();
  const l = (t: string, e: string) => (tr ? t : e);

  let people = $state<Person[]>([]);
  let error = $state("");
  /// The link last made, and whose.
  let link = $state<{ id: string; url: string } | null>(null);
  let copied = $state(false);
  let id = $state("");
  let name = $state("");
  let mail = $state("");
  /// Asked once more before someone is taken off.
  let arming = $state("");

  async function load() {
    try {
      people = await api.people();
    } catch (e) {
      error = String(e);
    }
  }
  void load();

  async function add() {
    error = "";
    try {
      await api.personAdd(id.trim(), name.trim(), mail.trim());
      id = name = mail = "";
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function invite(who: string) {
    error = "";
    copied = false;
    try {
      const made = await api.personInvite(who);
      link = { id: who, url: location.origin + made.path };
    } catch (e) {
      error = String(e);
    }
  }

  async function copy() {
    if (!link) return;
    await navigator.clipboard.writeText(link.url).catch(() => {});
    copied = true;
  }

  async function remove(who: string) {
    if (arming !== who) {
      arming = who;
      return;
    }
    arming = "";
    error = "";
    try {
      await api.personRemove(who);
      if (link?.id === who) link = null;
      await load();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="people">
  <table>
    <tbody>
      {#each people as p (p.id)}
        <tr>
          <td class="name">{p.name}</td>
          <td class="muted">{p.id}</td>
          <td class="muted">{p.mail}</td>
          <td class="muted">{p.role === "owner" ? l("sahip", "owner") : ""}</td>
          <td class="does">
            <button onclick={() => invite(p.id)}>{l("Bağlantı üret", "Make a link")}</button>
            {#if p.role !== "owner"}
              <button class:danger={arming === p.id} onclick={() => remove(p.id)}>
                {arming === p.id ? l("Emin misin?", "Sure?") : l("Çıkar", "Remove")}
              </button>
            {/if}
          </td>
        </tr>
        {#if link?.id === p.id}
          <tr>
            <td colspan="5" class="link">
              <input readonly value={link.url} onfocus={(e) => e.currentTarget.select()} />
              <button onclick={copy}>{copied ? l("Kopyalandı", "Copied") : l("Kopyala", "Copy")}</button>
              <p class="muted">
                {l(
                  `Bir kez çalışır, yedi gün geçerlidir. Açan kişi ${p.name} olarak girer: yalnızca ona gönder.`,
                  `Good once, for seven days. Whoever opens it comes in as ${p.name}: send it to them alone.`,
                )}
              </p>
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>

  <form
    onsubmit={(e) => {
      e.preventDefault();
      void add();
    }}
  >
    <label>{l("Kimlik", "Id")} <input bind:value={id} placeholder="ada" required /></label>
    <label>{l("Ad", "Name")} <input bind:value={name} placeholder="Ada Yıldız" /></label>
    <label>{l("Google adresi (isteğe bağlı)", "Google address (optional)")} <input bind:value={mail} type="email" /></label>
    <button class="primary">{l("Ekle", "Add")}</button>
  </form>
  <p class="muted">
    {l(
      "Kimlik küçük harf, rakam ve tireden oluşur; kişinin klasörünün adıdır ve sonradan değişmez.",
      "An id is small letters, digits and hyphens; it names the person's folder and doesn't change later.",
    )}
  </p>
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .people {
    max-width: 760px;
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
  .muted {
    opacity: 0.7;
    font-size: 13px;
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
  .danger {
    color: #a33;
    font-weight: 600;
  }
  .error {
    color: #a33;
  }
</style>
