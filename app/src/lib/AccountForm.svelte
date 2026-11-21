<script lang="ts">
  // In the browser, the signed-in person's own account: their name, their
  // address, their password, and a way out everywhere. The desktop never
  // shows this.
  import { ask } from "./ask.svelte";
  import { api, type Account, type LinkedDevice } from "./api";

  let { tr, renamed }: { tr: boolean; renamed: (name: string) => void } = $props();
  const l = (t: string, e: string) => (tr ? t : e);

  /// The desktops the site hands out (deploy.sh puts them in /indir/), as
  /// its list of them says: file, and its name in each language.
  const DESKTOPS: Record<string, [string, string, string]> = {
    "windows-x86_64": ["Dactile-kurulum.exe", "Windows için indir", "Download for Windows"],
    "linux-x86_64": ["Dactile.AppImage", "Linux (AppImage)", "Linux (AppImage)"],
  };
  let handed = $state<[string, string, string][]>([]);
  void fetch("/indir/latest.json")
    .then((r) => (r.ok ? r.json() : null))
    .then((list) => {
      if (list?.platforms) handed = Object.keys(DESKTOPS).filter((p) => p in list.platforms).map((p) => DESKTOPS[p]);
    })
    .catch(() => {});

  let me = $state<Account | null>(null);
  let name = $state("");
  let mail = $state("");
  let old = $state("");
  let fresh = $state("");
  let again = $state("");
  /// What the last thing done said, and whether it went wrong.
  let said = $state<{ what: string; bad: boolean; where: string } | null>(null);

  async function load() {
    try {
      me = await api.account();
      name = me.name;
    } catch (e) {
      said = { what: String(e), bad: true, where: "name" };
    }
  }
  void load();

  /// The desktops linked to the account.
  let desks = $state<LinkedDevice[]>([]);
  const loadDesks = async () => (desks = await api.devices().catch(() => []));
  void loadDesks();

  async function unlink(d: LinkedDevice, e: MouseEvent) {
    const yes = await ask({
      title: l(`${d.name} kaldırılsın mı?`, `Take off ${d.name}?`),
      detail: l("Oradaki Dactile sitedeki oyunlarını artık açamaz.", "Its Dactile can no longer open your plays here."),
      yes: l("Kaldır", "Take off"),
      no: l("Vazgeç", "Cancel"),
      danger: true,
      at: e,
    });
    if (!yes) return;
    await act("desks", async () => {
      await api.deviceOff(d.id);
      await loadDesks();
      return l("Bilgisayar kaldırıldı.", "The computer is taken off.");
    });
  }

  async function act(where: string, f: () => Promise<string>) {
    said = null;
    try {
      said = { what: await f(), bad: false, where };
    } catch (e) {
      said = { what: String(e), bad: true, where };
    }
  }

  const saveName = () =>
    act("name", async () => {
      const r = await api.accountName(name);
      name = r.name;
      renamed(r.name);
      return l("Adın kaydedildi.", "Your name is saved.");
    });

  const saveMail = () =>
    act("mail", async () => {
      await api.accountMail(mail);
      const sent = mail;
      mail = "";
      return l(
        `${sent} adresine bir bağlantı gönderdik; açınca adresin o olur.`,
        `We sent a link to ${sent}; once you open it, that is your address.`,
      );
    });

  const savePassword = () =>
    act("password", async () => {
      if (fresh !== again) throw l("İki parola aynı değil.", "The two passwords differ.");
      await api.accountPassword(old, fresh);
      old = fresh = again = "";
      await load();
      return l(
        "Parolan değişti. Öteki cihazlardaki oturumların kapandı.",
        "Your password is changed. You are signed out on other devices.",
      );
    });

  async function outEverywhere(e: MouseEvent) {
    const yes = await ask({
      title: l("Bütün cihazlardan çıkılsın mı?", "Sign out everywhere?"),
      detail: l("Bu cihaz da çıkar; yeniden girmen gerekir.", "This device too; you sign in again."),
      yes: l("Çık", "Sign out"),
      no: l("Vazgeç", "Cancel"),
      danger: true,
      at: e,
    });
    if (yes) await api.accountOut();
  }
</script>

{#if me}
  <div class="account">
    <p class="who">
      <span class="muted">{l("Kullanıcı adı", "User name")}:</span> <b>{me.id}</b>
      {#if me.mail}
        · <span class="muted">{l("E-posta", "E-mail")}:</span> {me.mail}
        {#if !me.checked}<span class="warn">({l("doğrulanmadı", "not checked")})</span>{/if}
      {/if}
    </p>

    <form
      onsubmit={(e) => {
        e.preventDefault();
        void saveName();
      }}
    >
      <label>{l("Görünen ad", "Name shown")} <input bind:value={name} /></label>
      <button>{l("Kaydet", "Save")}</button>
    </form>
    {#if said?.where === "name"}<p class:error={said.bad} class="said">{said.what}</p>{/if}

    {#if me.sends}
      <form
        onsubmit={(e) => {
          e.preventDefault();
          void saveMail();
        }}
      >
        <label>{l("Yeni e-posta adresi", "New e-mail address")} <input bind:value={mail} type="email" required /></label>
        <button>{l("Bağlantı gönder", "Send a link")}</button>
      </form>
      {#if said?.where === "mail"}<p class:error={said.bad} class="said">{said.what}</p>{/if}
    {/if}

    <form
      onsubmit={(e) => {
        e.preventDefault();
        void savePassword();
      }}
    >
      {#if me.password}
        <label
          >{l("Şimdiki parola", "Password now")}
          <input bind:value={old} type="password" autocomplete="current-password" required /></label
        >
      {/if}
      <label
        >{me.password ? l("Yeni parola", "New password") : l("Parola belirle", "Set a password")}
        <input bind:value={fresh} type="password" autocomplete="new-password" minlength="10" required /></label
      >
      <label
        >{l("Yeniden", "Again")}
        <input bind:value={again} type="password" autocomplete="new-password" minlength="10" required /></label
      >
      <button>{l("Parolayı değiştir", "Change password")}</button>
    </form>
    {#if said?.where === "password"}<p class:error={said.bad} class="said">{said.what}</p>{/if}

    <h3 class="ui-eyebrow">{l("Bağlı bilgisayarlar", "Linked computers")}</h3>
    {#if desks.length}
      <ul class="desks">
        {#each desks as d (d.id)}
          <li>
            <span class="ui-clip" title={d.name}>{d.name}</span>
            <button class="ui-btn small danger" onclick={(e) => unlink(d, e)}>{l("Kaldır", "Take off")}</button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="muted none">
        {l(
          "Bağlı bilgisayar yok. Masaüstündeki Dactile, Sitede bölümünden bağlanır.",
          "No computer is linked. Dactile on a desktop links from On the site.",
        )}
      </p>
    {/if}
    {#if said?.where === "desks"}<p class:error={said.bad} class="said">{said.what}</p>{/if}
    {#if handed.length}
      <p class="muted get">
        {l("Masaüstü uygulaması:", "The desktop app:")}
        {#each handed as [file, t, e], i (file)}{#if i}<span> · </span>{/if}<a class="ui-link" href={`/indir/${file}`} download>{l(t, e)}</a>{/each}
      </p>
    {/if}

    <p class="out">
      <button onclick={outEverywhere}>{l("Bütün cihazlardan çık", "Sign out everywhere")}</button>
    </p>
  </div>
{:else if said}
  <p class="error">{said.what}</p>
{/if}

<style>
  .get a {
    margin-left: 4px;
  }
  .account {
    max-width: 760px;
  }
  .who {
    margin: 0 0 14px;
  }
  form {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: end;
    margin-bottom: 10px;
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
  .muted {
    opacity: 0.7;
  }
  .warn {
    color: var(--warn);
    font-size: 13px;
  }
  .said {
    margin: -4px 0 14px;
    font-size: 13px;
  }
  .error {
    color: var(--danger);
  }
  .out {
    margin-top: 18px;
  }
  h3 {
    margin: 22px 0 8px;
  }
  .desks {
    display: grid;
    gap: 6px;
    max-width: 420px;
    margin: 0 0 10px;
    padding: 0;
    list-style: none;
  }
  .desks li {
    display: flex;
    gap: 10px;
    align-items: center;
    justify-content: space-between;
    min-width: 0;
    padding: 6px 6px 6px 10px;
    background: var(--raised);
    border: 1px solid var(--chrome-line);
    border-radius: var(--r-md);
  }
  .none {
    margin: 0 0 10px;
    font-size: 13px;
  }
</style>
