// The door's page, mounted where the app would be: for someone not signed
// in, and for the links mail brings (an invitation, a new password, an
// address to check).
import { mount } from "svelte";
import "../../src/lib/theme.css";
import Door from "./Door.svelte";

export function open(way: { google: boolean; mail: boolean }) {
  document.body.innerHTML = '<div id="door"></div>';
  mount(Door, { target: document.getElementById("door")!, props: { way } });
}
