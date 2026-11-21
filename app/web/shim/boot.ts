// The page's start: signed in, the interface comes; signed out, the door:
// the gate's sign-in page (web/door/), or a password where one server runs
// for one person. Signed out later, the page asks again.

import { post } from "./core";

async function signedIn() {
  const r = await fetch("/api/version");
  return r.ok;
}

function askPassword(wrong = false) {
  document.body.innerHTML = `
    <form id="login" style="max-width:20rem;margin:20vh auto;font-family:system-ui,sans-serif;display:grid;gap:.75rem">
      <h1 style="font-family:serif;letter-spacing:.3em;margin:0 0 .5rem">DACTILE</h1>
      <label>Parola <input name="password" type="password" autocomplete="current-password" autofocus required
        style="width:100%;padding:.5rem;font:inherit;box-sizing:border-box"></label>
      <button style="padding:.5rem;font:inherit">Gir</button>
      <p id="wrong" style="color:#a33;margin:0" ${wrong ? "" : "hidden"}>Parola yanlış.</p>
    </form>`;
  const form = document.getElementById("login") as HTMLFormElement;
  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const password = new FormData(form).get("password");
    try {
      await post("/api/login", { password });
      location.reload();
    } catch {
      askPassword(true);
    }
  });
}

/// The links mail brings: an invitation, a new password, an address; and
/// the page a desktop opens to be linked. They open the door's page whether or not this browser is signed in.
const DOOR = /^\/(davet|parola|giris|dogrula)(\/[0-9a-f]+)?$|^\/cihaz\/[A-Za-z0-9-]+$/;

const door = fetch("/api/door")
  .then((r) => r.json())
  .catch(() => null);

/// The trial site says so on every page, the door's and the app's. Kept
/// outside <body>, which the door and the password page write over; low on
/// the page, clear of the title and the ribbon, above the status bar.
void door.then((d) => {
  if (d?.stage !== "pbe") return;
  const mark = document.createElement("div");
  mark.textContent = "DENEME SİTESİ · yazdıkların silinebilir";
  mark.setAttribute("role", "note");
  mark.style.cssText =
    "position:fixed;bottom:30px;left:50%;transform:translateX(-50%);z-index:99999;pointer-events:none;" +
    "background:#8a2a2a;color:#fff;font:600 11px/1 system-ui,sans-serif;letter-spacing:.08em;" +
    "padding:4px 12px;border-radius:6px;opacity:.85";
  document.documentElement.appendChild(mark);
});

async function ask() {
  const d = await door;
  if (d?.kind === "accounts") {
    const { open } = await import("../door/main");
    open({ google: d.google === true, mail: d.mail === true });
  } else askPassword();
}

window.addEventListener("dactile-signed-out", () => void ask());

if (!DOOR.test(location.pathname) && (await signedIn())) await import("../../src/main");
else await ask();
