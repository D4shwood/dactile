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

window.addEventListener("dactile-signed-out", () => askPassword());

if (await signedIn()) await import("../../src/main");
else askPassword();
