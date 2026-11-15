// The page's start: signed in, the interface comes; signed out, a password
// first. Signed out later (the server restarted), the page asks again.

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

/// Where one comes in with a link the owner sends, there is nothing to type;
/// with Google too, where the site has it.
function askForLink(google: boolean) {
  const withGoogle = google
    ? `<p style="margin:0 0 1.5rem"><a href="/giris/google"
         style="display:inline-block;padding:.6rem 1.2rem;border:1px solid #999;border-radius:4px;color:inherit;text-decoration:none">Google ile gir</a></p>
       <p style="color:#666;margin:0">Ya da sana gönderilen bağlantıyı aç.</p>`
    : `<p style="font-size:1.1rem;margin:0 0 1rem">Girmek için sana gönderilen bağlantıyı aç.</p>
       <p style="color:#666;margin:0">Bağlantın yoksa ya da süresi dolduysa site sahibinden yenisini iste.</p>`;
  document.body.innerHTML = `
    <div style="max-width:22rem;margin:20vh auto;padding:0 1rem;font-family:system-ui,sans-serif">
      <h1 style="font-family:serif;letter-spacing:.3em;margin:0 0 1.5rem">DACTILE</h1>
      ${withGoogle}
    </div>`;
}

async function ask() {
  const door = await fetch("/api/door")
    .then((r) => r.json())
    .catch(() => null);
  if (door?.kind === "link") askForLink(door.google === true);
  else askPassword();
}

window.addEventListener("dactile-signed-out", () => void ask());

if (await signedIn()) await import("../../src/main");
else await ask();
