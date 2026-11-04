// What the page held when the writer went away, by the play it was opened
// with: a hot reload while developing (vite remounts the writer with the
// same `opened`) picks the text up where it was instead of as it was opened,
// so an autosave never writes the older text back. A play opened again is a
// new `opened` and starts from the disk.

import type { Opened } from "./api";

export const kept = new WeakMap<Opened, string>();
