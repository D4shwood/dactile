<script lang="ts">
  // In the browser, the signed-in person's own account: their name, their
  // address, their password, and a way out everywhere. The desktop never
  // shows this.
  import { api, type Account } from "./api";

  let { tr, renamed }: { tr: boolean; renamed: (name: string) => void } = $props();
  const l = (t: string, e: string) => (tr ? t : e);

  let me = $state<Account | null>(null);
  let name = $state("");
  let mail = $state("");
  let old = $state("");
  let fresh = $state("");
  let again = $state("");
  /// What the last thing done said, and whether it went wrong.
  let said = $state<{ what: string; bad: boolean; where: string } | null>(null);
  let arming = $state(false);

  async function load() {
    try {
      me = await api.account();
      name = me.name;
    } catch (e) {
      said = { what: String(e), bad: true, where: "name" };
    }
  }
  void load();

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

  async function outEverywhere() {
    if (!arming) {
      arming = true;
      return;
    }
    await api.accountOut();
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

    <p class="out">
      <button class:danger={arming} onclick={outEverywhere}>
        {arming ? l("Emin misin? Bu cihaz da çıkar", "Sure? This device too") : l("Bütün cihazlardan çık", "Sign out everywhere")}
      </button>
    </p>
  </div>
{:else if said}
  <p class="error">{said.what}</p>
{/if}

<style>
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
    color: var(--warn, #b05820);
    font-size: 13px;
  }
  .said {
    margin: -4px 0 14px;
    font-size: 13px;
  }
  .error {
    color: #a33;
  }
  .out {
    margin-top: 18px;
  }
  .danger {
    color: #a33;
    font-weight: 600;
  }
</style>
