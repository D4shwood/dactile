<script lang="ts">
  // The site's door, for someone not signed in: signing in, an invitation
  // taken up, a forgotten password, an address checked. Which of them, the
  // address says. Turkish: the site is.
  import { onMount } from "svelte";

  type Way = { google: boolean; mail: boolean };
  let { way }: { way: Way } = $props();

  const path = location.pathname;
  const code = (prefix: string) => {
    const m = path.match(new RegExp(`^/${prefix}/([0-9a-f]+)$`));
    return m ? m[1] : "";
  };
  const davet = code("davet");
  const parola = code("parola") || code("giris");
  const dogrula = code("dogrula");
  /// A desktop asking to be linked: the code it shows.
  const cihaz = path.match(/^\/cihaz\/([A-Za-z0-9-]{4,16})$/)?.[1] ?? "";
  type Screen = "giris" | "davet" | "unuttum" | "parola" | "dogrula" | "cihaz";
  let screen = $state<Screen>(
    cihaz ? "cihaz" : davet ? "davet" : parola ? "parola" : dogrula ? "dogrula" : path === "/parola" ? "unuttum" : "giris",
  );

  let error = $state("");
  let busy = $state(false);
  /// A screen whose link was no good: what to say instead of it.
  let gone = $state(false);

  class SignedOut extends Error {}

  async function ask<T>(url: string, body?: unknown): Promise<T> {
    const r = await fetch(url, {
      method: body === undefined ? "GET" : "POST",
      headers: body === undefined ? {} : { "content-type": "application/json", "x-dactile": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    if (r.status === 401) throw new SignedOut();
    const said = await r.json().catch(() => null);
    if (!r.ok) throw typeof said === "string" ? said : "Bir şeyler ters gitti; biraz sonra yine dene.";
    return said as T;
  }

  async function act(f: () => Promise<void>) {
    error = "";
    busy = true;
    try {
      await f();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function go(to: string) {
    history.pushState(null, "", to);
    error = "";
    gone = false;
    screen = to === "/parola" ? "unuttum" : "giris";
  }

  // Signing in.
  let who = $state("");
  let password = $state("");
  const signIn = () =>
    act(async () => {
      await ask("/api/giris", { who, password });
      // Signed in on the way to linking a desktop: back to that.
      location.href = cihaz ? path : "/";
    });

  // An invitation.
  let invited = $state<{ mail: string; name: string; by: string } | null>(null);
  let id = $state("");
  let name = $state("");
  let mail = $state("");
  let again = $state("");
  const idOk = $derived(/^[a-z0-9][a-z0-9-]{0,31}$/.test(id));
  const accept = () =>
    act(async () => {
      if (password !== again) throw "İki parola aynı değil.";
      await ask(`/api/davet/${davet}`, { id, name, mail, password });
      location.href = "/";
    });
  const acceptWithGoogle = () =>
    act(async () => {
      if (!idOk) throw "Önce bir kullanıcı adı seç.";
      const r = await ask<{ to: string }>(`/api/davet/${davet}/google`, { id, name });
      location.href = r.to;
    });

  // A forgotten password.
  let forgotMail = $state("");
  let sent = $state(false);
  const forgot = () =>
    act(async () => {
      await ask("/api/parola", { mail: forgotMail });
      sent = true;
    });

  // A new password.
  let whose = $state<{ name: string } | null>(null);
  const reset = () =>
    act(async () => {
      if (password !== again) throw "İki parola aynı değil.";
      await ask(`/api/parola/${parola}`, { password });
      location.href = "/";
    });

  // An address.
  let checkedMail = $state("");

  // A desktop to link.
  let desk = $state<{ name: string; code: string } | null>(null);
  let linked = $state<boolean | null>(null);
  const answerDesk = (yes: boolean) =>
    act(async () => {
      await ask(`/api/cihaz/${yes ? "onay" : "ret"}/${cihaz}`, {});
      linked = yes;
    });

  onMount(async () => {
    try {
      if (screen === "davet") {
        const got = await ask<{ mail: string; name: string; by: string }>(`/api/davet/${davet}`);
        invited = got;
        mail = got.mail;
        name = got.name;
      } else if (screen === "parola") {
        whose = await ask(`/api/parola/${parola}`);
      } else if (screen === "dogrula") {
        checkedMail = (await ask<{ mail: string }>(`/api/dogrula/${dogrula}`, {})).mail;
      } else if (screen === "cihaz") {
        desk = await ask(`/api/cihaz/bak/${cihaz}`);
      }
    } catch (e) {
      // Linking wants someone signed in: first that, then back here.
      if (e instanceof SignedOut) {
        screen = "giris";
        return;
      }
      gone = true;
      error = String(e);
    }
  });
</script>

<main>
  <header class="brand" aria-label="Dactile">
    <div class="name">D A C T I L E<span class="caret"></span></div>
    <div class="what">Dynamic Stagewright</div>
  </header>

  <section class="card">
    {#if gone}
      <h1>Bu bağlantı artık geçerli değil</h1>
      <p class="muted">Kullanılmış ya da süresi dolmuş olabilir.</p>
      <div class="row">
        <button class="primary" onclick={() => go("/")}>Girişe dön</button>
        {#if screen === "parola" && way.mail}
          <button onclick={() => go("/parola")}>Yeni bağlantı iste</button>
        {/if}
      </div>
    {:else if screen === "giris"}
      <h1>Giriş</h1>
      {#if way.google}
        <a class="google" href="/giris/google"><span class="g">G</span> Google ile gir</a>
        <div class="or"><span>ya da</span></div>
      {/if}
      <form
        onsubmit={(e) => {
          e.preventDefault();
          void signIn();
        }}
      >
        <label>
          Kullanıcı adı ya da e-posta
          <input bind:value={who} autocomplete="username" autocapitalize="none" spellcheck="false" required />
        </label>
        <label>
          Parola
          <input bind:value={password} type="password" autocomplete="current-password" required />
        </label>
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        <button class="primary" disabled={busy}>Gir</button>
      </form>
      <button class="link" onclick={() => go("/parola")}>Parolamı unuttum</button>
    {:else if screen === "davet"}
      {#if invited}
        <h1>Dactile'a hoş geldin</h1>
        <p class="lead">
          {invited.by || "Site sahibi"} seni Dactile'a çağırıyor. Hesabını aç; kullanıcı adın klasörünün adı da olacak.
        </p>
        <form
          onsubmit={(e) => {
            e.preventDefault();
            void accept();
          }}
        >
          <label>
            Kullanıcı adı
            <input
              bind:value={id}
              autocomplete="username"
              autocapitalize="none"
              spellcheck="false"
              maxlength="32"
              placeholder="ornek-ad"
              required
            />
            <small class:bad={id !== "" && !idOk}>Küçük harf, rakam ve tire; sonradan değişmez.</small>
          </label>
          <label>
            Görünen ad
            <input bind:value={name} autocomplete="name" placeholder="Adın Soyadın" />
          </label>
          {#if way.google}
            <button type="button" class="google" disabled={busy} onclick={acceptWithGoogle}>
              <span class="g">G</span> Google hesabımla devam et
            </button>
            <div class="or"><span>ya da parolayla</span></div>
          {/if}
          <label>
            E-posta
            <input bind:value={mail} type="email" autocomplete="email" required />
            {#if mail.trim().toLowerCase() !== invited.mail}
              <small>Bu adrese bir doğrulama bağlantısı gidecek.</small>
            {/if}
          </label>
          <label>
            Parola
            <input bind:value={password} type="password" autocomplete="new-password" minlength="10" required />
            <small>En az 10 karakter.</small>
          </label>
          <label>
            Parola (yeniden)
            <input bind:value={again} type="password" autocomplete="new-password" minlength="10" required />
          </label>
          {#if error}<p class="error" role="alert">{error}</p>{/if}
          <button class="primary" disabled={busy || !idOk}>Hesabı aç</button>
        </form>
      {:else}
        <p class="muted">Davet açılıyor…</p>
      {/if}
    {:else if screen === "unuttum"}
      <h1>Parolamı unuttum</h1>
      {#if !way.mail}
        <p>Bu site posta göndermiyor. Yeni bir parola bağlantısı için site sahibine yaz.</p>
      {:else if sent}
        <p>Bu adresle bir hesap varsa yeni parola bağlantısını gönderdik. Bir saat geçerli.</p>
        <p class="muted">Gelmediyse istenmeyen postalara da bak.</p>
      {:else}
        <form
          onsubmit={(e) => {
            e.preventDefault();
            void forgot();
          }}
        >
          <label>
            Hesabının e-posta adresi
            <input bind:value={forgotMail} type="email" autocomplete="email" required />
          </label>
          {#if error}<p class="error" role="alert">{error}</p>{/if}
          <button class="primary" disabled={busy}>Bağlantı gönder</button>
        </form>
      {/if}
      <button class="link" onclick={() => go("/")}>Girişe dön</button>
    {:else if screen === "parola"}
      {#if whose}
        <h1>Yeni parola</h1>
        <p class="lead">Merhaba {whose.name}. Yeni parolanı seç; öteki cihazlardaki oturumların kapanacak.</p>
        <form
          onsubmit={(e) => {
            e.preventDefault();
            void reset();
          }}
        >
          <label>
            Yeni parola
            <input bind:value={password} type="password" autocomplete="new-password" minlength="10" required />
            <small>En az 10 karakter.</small>
          </label>
          <label>
            Yeni parola (yeniden)
            <input bind:value={again} type="password" autocomplete="new-password" minlength="10" required />
          </label>
          {#if error}<p class="error" role="alert">{error}</p>{/if}
          <button class="primary" disabled={busy}>Parolayı kaydet ve gir</button>
        </form>
      {:else}
        <p class="muted">Bağlantı açılıyor…</p>
      {/if}
    {:else if screen === "cihaz"}
      {#if linked === true}
        <h1>Bilgisayar bağlandı</h1>
        <p>Masaüstündeki Dactile'a dönebilirsin; sitedeki oyunların orada da görünecek.</p>
        <p class="muted">Bağlı bilgisayarları Ayarlar &gt; Hesabım'dan kaldırabilirsin.</p>
      {:else if linked === false}
        <h1>Bağlanmadı</h1>
        <p>Bu bilgisayar hesabına bağlanmadı.</p>
        <a class="button" href="/">Dactile'a git</a>
      {:else if desk}
        <h1>Bu bilgisayar bağlansın mı?</h1>
        <p class="lead"><b>{desk.name || "Adı olmayan bir bilgisayar"}</b> hesabına bağlanmak istiyor.</p>
        <p class="code" aria-label="Kod">{desk.code.slice(0, 4)}-{desk.code.slice(4)}</p>
        <p class="muted">
          Bu kod masaüstündeki Dactile'da görünenle aynı değilse ya da bağlanmayı sen istemediysen Hayır de. Bağlanan bilgisayar
          oyunlarını açıp yazabilir.
        </p>
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        <div class="row">
          <button class="primary" disabled={busy} onclick={() => answerDesk(true)}>Evet, bağla</button>
          <button disabled={busy} onclick={() => answerDesk(false)}>Hayır</button>
        </div>
      {:else}
        <p class="muted">Açılıyor…</p>
      {/if}
    {:else if screen === "dogrula"}
      {#if checkedMail}
        <h1>Adresin doğrulandı</h1>
        <p>{checkedMail} artık hesabının adresi.</p>
        <a class="button primary" href="/">Dactile'a git</a>
      {:else}
        <p class="muted">Doğrulanıyor…</p>
      {/if}
    {/if}
  </section>

  {#if screen === "giris" && !gone}
    <p class="foot">Hesabın yok mu? Dactile yalnızca davetle açılır: seni çağıracak kişiye yaz.</p>
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    min-height: 100vh;
    background:
      radial-gradient(1200px 500px at 50% -10%, color-mix(in srgb, var(--claret) 14%, transparent), transparent 70%),
      var(--chrome);
    color: var(--chrome-fg);
    font-family: var(--sans);
  }
  main {
    max-width: 400px;
    margin: 0 auto;
    padding: 9vh 16px 40px;
  }
  .brand {
    padding: 18px 24px 14px;
    font-family: var(--mono);
    background: #151515;
    border-radius: 14px 14px 0 0;
  }
  .brand .name {
    color: #d8cfc0;
    font-weight: 700;
    font-size: 21px;
    white-space: nowrap;
  }
  .caret {
    display: inline-block;
    width: 0.6em;
    height: 1.05em;
    margin-left: 0.35em;
    vertical-align: -0.15em;
    background: #c4505a;
    animation: blink 1.2s steps(1) infinite;
  }
  .brand .what {
    margin-top: 4px;
    color: #8f877b;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    width: 19ch;
    animation: type 0.55s steps(19) 0.15s both;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
  @keyframes type {
    from {
      width: 0;
    }
  }
  /* The cursor blinks all the same. */
  @media (prefers-reduced-motion: reduce) {
    .brand .what {
      animation: none;
    }
  }
  .card {
    padding: 24px 24px 20px;
    background: var(--page);
    border: 1px solid var(--chrome-line);
    border-top: 3px solid var(--claret);
    border-radius: 0 0 14px 14px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.12);
  }
  h1 {
    font-family: var(--serif);
    font-weight: 600;
    font-size: 26px;
    margin: 0 0 16px;
    color: var(--fg);
  }
  .code {
    margin: 4px 0 10px;
    color: var(--accent-ink);
    font: 600 26px var(--mono);
    letter-spacing: 0.12em;
    text-align: center;
  }
  .lead {
    margin: -6px 0 16px;
    line-height: 1.45;
  }
  form {
    display: grid;
    gap: 14px;
  }
  label {
    display: grid;
    gap: 5px;
    font-size: 13px;
    color: var(--chrome-muted);
  }
  input {
    font: inherit;
    font-size: 15px;
    color: var(--fg);
    background: color-mix(in srgb, var(--page) 85%, #fff);
    border: 1px solid var(--chrome-line);
    border-radius: 8px;
    padding: 9px 12px;
    transition:
      border-color 0.12s,
      box-shadow 0.12s;
  }
  input:focus {
    outline: none;
    border-color: var(--accent-ink);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-ink) 18%, transparent);
  }
  small {
    font-size: 12px;
    color: var(--muted);
  }
  small.bad {
    color: var(--claret-deep);
  }
  button,
  .button {
    font: inherit;
    font-size: 15px;
    padding: 10px 14px;
    border-radius: 8px;
    border: 1px solid var(--chrome-line);
    background: var(--chrome-2);
    color: var(--chrome-fg);
    cursor: pointer;
    text-align: center;
    text-decoration: none;
  }
  .primary {
    background: var(--claret);
    border-color: var(--claret);
    color: #fff;
    font-weight: 600;
  }
  .primary:hover:not(:disabled) {
    background: var(--claret-deep);
  }
  button:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .google {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    width: 100%;
    box-sizing: border-box;
    padding: 10px 14px;
    border: 1px solid var(--chrome-line);
    border-radius: 8px;
    background: color-mix(in srgb, var(--page) 85%, #fff);
    color: var(--fg);
    text-decoration: none;
    font-size: 15px;
  }
  .g {
    font-weight: 700;
    color: #4285f4;
  }
  .or {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 14px 0;
    color: var(--faint);
    font-size: 12px;
  }
  .or::before,
  .or::after {
    content: "";
    flex: 1;
    border-top: 1px solid var(--chrome-line);
  }
  .link {
    background: none;
    border: 0;
    padding: 0;
    margin-top: 14px;
    color: var(--accent-ink);
    font-size: 14px;
    text-decoration: underline;
  }
  .row {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .muted {
    color: var(--muted);
  }
  .error {
    margin: 0;
    padding: 8px 12px;
    color: var(--accent-ink);
    font-size: 14px;
    background: color-mix(in srgb, var(--claret) 10%, transparent);
    border-left: 3px solid var(--claret);
    border-radius: 6px;
  }
  .foot {
    margin: 16px 2px 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--chrome-muted);
  }
</style>
