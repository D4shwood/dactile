// The writer's ribbon: the same tabs, groups and buttons as the terminal
// app's (tui/src/tui/writer/ribbon.rs). An action the desktop app can't do
// yet is shown, greyed, so the ribbon reads the same in both.

export type Act =
  | { do: "key"; id: string }
  | { do: "insert"; snippet: string; block: boolean }
  | { do: "soon" };

export interface Btn {
  tr: string;
  en: string;
  icon: string;
  key: string;
  act: Act;
}

export interface Group {
  tr: string;
  en: string;
  btns: Btn[];
}

export interface Tab {
  tr: string;
  en: string;
  tip: [string, string];
  groups: Group[];
}

const k = (id: string): Act => ({ do: "key", id });
const ins = (snippet: string, block = true): Act => ({ do: "insert", snippet, block });
const soon: Act = { do: "soon" };
const b = (tr: string, en: string, icon: string, key: string, act: Act): Btn => ({ tr, en, icon, key, act });
const g = (tr: string, en: string, btns: Btn[]): Group => ({ tr, en, btns });

export const FILE_TAB = 0;

export const TABS: Tab[] = [
  { tr: "Dosya", en: "File", tip: ["D", "F"], groups: [] },
  {
    tr: "Giriş",
    en: "Home",
    tip: ["G", "H"],
    groups: [
      g("Kayıt", "Save", [
        b("Kaydet", "Save", "▣", "Ctrl+S", k("save")),
        b("PDF derle", "Build PDF", "⎙", "F5", k("build")),
      ]),
      g("Geri al", "Undo", [
        b("Geri al", "Undo", "↶", "Ctrl+Z", k("undo")),
        b("Yinele", "Redo", "↷", "Ctrl+Y", k("redo")),
      ]),
      g("Pano", "Clipboard", [
        b("Yapıştır", "Paste", "⎗", "Ctrl+V", k("paste")),
        b("Hepsini seç", "Select all", "▤", "Ctrl+A", k("select-all")),
        b("Kes", "Cut", "✂", "Ctrl+X", k("cut")),
        b("Kopyala", "Copy", "⎘", "Ctrl+C", k("copy")),
      ]),
      g("Düzenleme", "Editing", [
        b("Bul", "Find", "⌕", "Ctrl+F", k("find")),
        b("Yazım", "Spelling", "✓", "Alt+S", soon),
        b("Ara", "Search", "⌘", "Ctrl+K", soon),
        b("Komutlar", "Commands", "≡", "Ctrl+P", k("commands")),
      ]),
    ],
  },
  {
    tr: "Ekle",
    en: "Insert",
    tip: ["E", "N"],
    groups: [
      g("Yapı", "Structure", [
        b("Perde", "Act", "▬", "#perde", ins("#perde ‸")),
        b("Sahne", "Scene", "§", "#sahne", ins("#sahne ‸")),
        b("Önsahne", "Prologue", "◇", "#önsahne", ins("#önsahne ‸")),
      ]),
      g("Kişi", "People", [
        b("Karakter", "Character", "☺", "", soon),
        b("Kişiler", "Cast", "⁂", "Ctrl+O", soon),
      ]),
      g("Yönerge", "Directions", [
        b("Eylem", "Action", "⁋", "", ins("\\action{‸}")),
        b("Sahne yönergesi", "Stage direction", "⌖", "", ins("\\stagedir{‸}")),
        b("Replik içi", "Inside a line", "❨", "((", ins("\\ic{‸}", false)),
        b("Es", "Pause", "⋯", "", ins("\\beat‸")),
      ]),
      g("Bölüm", "Sections", [
        b("Önsöz", "Preface", "¶", "#önsöz", ins("#önsöz\n\n‸")),
        b("Rejisöre notlar", "Director's notes", "✎", "#notlar", ins("#notlar\n\n‸")),
      ]),
    ],
  },
  {
    tr: "Görünüm",
    en: "View",
    tip: ["R", "W"],
    groups: [
      g("Görünümler", "Views", [
        b("Sayfa", "Page", "▯", "F7", k("page")),
        b("Kaynak", "Source", "⟨⟩", "F2", k("source")),
        b("PDF", "PDF", "⎙", "F12", k("pdf-panel")),
        b("Notlar", "Notes", "✎", "Ctrl+N", k("notes")),
      ]),
      g("Yakınlık", "Zoom", [
        b("Büyüt", "Zoom in", "+", "Ctrl++", k("zoom-in")),
        b("Küçült", "Zoom out", "−", "Ctrl+−", k("zoom-out")),
        b("%100", "100%", "○", "Ctrl+0", k("zoom-reset")),
      ]),
      g("Odak", "Focus", [
        b("Daktilo", "Typewriter", "⌨", "F3", k("typewriter")),
        b("Sahne odağı", "Scene focus", "◉", "F4", soon),
        b("Soldur", "Dim", "◐", "F8", soon),
      ]),
      g("Görünüş", "Look", [
        b("Tema", "Theme", "◈", "F6", k("theme")),
        b("Şeridi katla", "Fold ribbon", "⌃", "Ctrl+F1", k("fold")),
      ]),
    ],
  },
  {
    tr: "Oyun",
    en: "Play",
    tip: ["O", "P"],
    groups: [
      g("Kişiler", "Cast", [
        b("Kişiler", "Cast", "⁂", "Ctrl+O", soon),
        b("Yeni karakter", "New character", "☺", "", soon),
      ]),
      g("Sahneler", "Scenes", [
        b("Pano", "Board", "▦", "Ctrl+G", k("board")),
        b("Özet", "Summary", "≣", "Alt+E", soon),
        b("Böl", "Split", "⫶", "Alt+B", soon),
        b("Birleştir", "Join", "⋈", "Alt+J", soon),
      ]),
      g("Prova", "Rehearsal", [
        b("Dağıtım", "Casting", "☍", "F9", soon),
        b("Ezber", "Memorize", "◌", "", soon),
      ]),
      g("Kapak", "Cover", [b("Oyun bilgisi", "Cover details", "ⓘ", "Alt+İ", k("cover"))]),
    ],
  },
  {
    tr: "Notlar",
    en: "Notes",
    tip: ["N", "O"],
    groups: [
      g("Notlar", "Notes", [
        b("Panel", "Panel", "✎", "Ctrl+N", k("notes")),
        b("Buraya not", "Note here", "✚", "Alt+N", k("note-here")),
      ]),
      g("Al", "Take", [
        b("Panodan", "Clipboard", "⎗", "", k("notes-paste")),
        b("Dosyadan", "From a file", "⇲", "", k("notes-file")),
      ]),
      g("Çıktı", "Output", [b("Notlu PDF", "PDF with notes", "⎙", "", soon)]),
    ],
  },
  {
    tr: "Yardım",
    en: "Help",
    tip: ["Y", "Y"],
    groups: [
      g("Yardım", "Help", [
        b("Tuşlar", "Keys", "⌨", "F1", k("keys")),
        b("Komutlar", "Commands", "≡", "Ctrl+P", k("commands")),
        b("Sürüm", "Version", "ⓘ", "", k("about")),
      ]),
    ],
  },
];

export interface Entry {
  tr: string;
  en: string;
  key: string;
  act: Act;
}

/// The page Dosya opens.
export const BACKSTAGE: Entry[] = [
  { tr: "Bilgi", en: "Info", key: "", act: k("info") },
  { tr: "Kaydet", en: "Save", key: "Ctrl+S", act: k("save") },
  { tr: "PDF derle", en: "Build the PDF", key: "F5", act: k("build") },
  { tr: "Şablon olarak kaydet", en: "Save as a template", key: "", act: k("save-template") },
  { tr: "Dağıtım ve oyuncu kopyaları", en: "Casting and actors' copies", key: "F9", act: soon },
  { tr: "Notlu PDF kopyası", en: "A PDF copy with notes", key: "", act: soon },
  { tr: "Kapat", en: "Close", key: "Ctrl+W", act: k("close") },
];

/// Key tips: a letter for each button of a tab, the first free one of its
/// name, as the terminal app gives them.
export function tips(tab: Tab, tr: boolean): string[] {
  const used = new Set<string>();
  const out: string[] = [];
  for (const grp of tab.groups)
    for (const btn of grp.btns) {
      const name = (tr ? btn.tr : btn.en).toLocaleUpperCase(tr ? "tr" : "en");
      let c = [...name].find((ch) => /\p{L}/u.test(ch) && !used.has(ch));
      if (!c) c = [..."ABCDEFGHIJKLMNOPRSTUVYZ123456789"].find((ch) => !used.has(ch))!;
      used.add(c);
      out.push(c);
    }
  return out;
}
