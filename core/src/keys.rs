//! The keys, one list for both apps: what each key does and what it is for.
//! Dactile and Dactile Light draw the same coloured keyboard from it.

use crate::lang;

/// Which app a key belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Only {
    Both,
    Desktop,
    Light,
}

impl Only {
    pub fn fits(self, app: Only) -> bool {
        self == Only::Both || self == app
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cat {
    File,
    Edit,
    Panel,
    View,
    Write,
}

impl Cat {
    pub const ALL: [Cat; 5] = [Cat::Write, Cat::Panel, Cat::View, Cat::Edit, Cat::File];

    pub fn name(self) -> &'static str {
        match self {
            Cat::File => lang::l("dosya", "file"),
            Cat::Edit => lang::l("düzenleme", "editing"),
            Cat::Panel => lang::l("pencereler", "panels"),
            Cat::View => lang::l("görünüm", "view"),
            Cat::Write => lang::l("yazarken", "writing"),
        }
    }
}

/// A key and what it does.
pub struct Bind {
    /// The key cap: "O", "F3", "\\", "Tab".
    pub cap: &'static str,
    pub ctrl: bool,
    pub alt: bool,
    pub cat: Cat,
    pub tr: &'static str,
    pub en: &'static str,
    pub only: Only,
}

const fn b(
    cap: &'static str,
    ctrl: bool,
    alt: bool,
    cat: Cat,
    tr: &'static str,
    en: &'static str,
) -> Bind {
    Bind {
        cap,
        ctrl,
        alt,
        cat,
        tr,
        en,
        only: Only::Both,
    }
}

const fn desk(mut x: Bind) -> Bind {
    x.only = Only::Desktop;
    x
}

const fn term(mut x: Bind) -> Bind {
    x.only = Only::Light;
    x
}

/// Every key of the writer, the one list the keyboard and the short list
/// both read.
pub const BINDS: &[Bind] = &[
    b(
        "\\",
        false,
        false,
        Cat::Write,
        "öneri menüsü: karakterler, komutlar",
        "suggestions: characters, commands",
    ),
    b(
        "#",
        false,
        false,
        Cat::Write,
        "satır başında: #sahne, #önsahne, #perde, #önsöz, #notlar",
        "at a line's start: #sahne, #prologue, #perde, #önsöz, #notlar",
    ),
    b(
        ":",
        false,
        false,
        Cat::Write,
        "SELİN: yazınca SELİN'in repliği olur",
        "typing SELİN: makes it SELİN's line",
    ),
    b(
        "(",
        false,
        false,
        Cat::Write,
        "(( replik içinde yönerge açar: (güler)",
        "(( opens a direction inside a line: (laughs)",
    ),
    b(
        "Tab",
        false,
        false,
        Cat::Write,
        "boş satırda sıradaki konuşanı yazar",
        "on an empty line, writes who speaks next",
    ),
    b(
        "⌫",
        false,
        false,
        Cat::Write,
        "bir makronun içine girince satırın kaynağı açılır",
        "into a macro: the line shows its source",
    ),
    b(
        "P",
        true,
        false,
        Cat::Panel,
        "komut paleti: her şeyi adıyla bul ve yap",
        "command palette: find anything by name and do it",
    ),
    b(
        "O",
        true,
        false,
        Cat::Panel,
        "kişiler: düzenle, kart, sıra, yeni, sil",
        "the cast: edit, card, order, new, delete",
    ),
    b(
        "N",
        true,
        false,
        Cat::Panel,
        "notlar yanda: AI'ın, atölyenin, senin; g bu kısmı, U hepsini uygular",
        "notes beside: an AI's, a workshop's, yours; g applies a part, U all",
    ),
    b(
        "N",
        false,
        true,
        Cat::Panel,
        "seçili söze ya da imleçteki kelimeye not",
        "a note on the selection or the word at the cursor",
    ),
    term(b(
        "G",
        true,
        false,
        Cat::Panel,
        "sahne şeridi yazının yanında; iki kez: pano (liste, kartlar, taşı)",
        "the scene strip beside the writing; twice: the board (list, cards, move)",
    )),
    b(
        "E",
        false,
        true,
        Cat::Panel,
        "imlecin sahnesinin özeti (PDF'e girmez)",
        "the cursor's scene's summary (never printed)",
    ),
    b(
        "B",
        false,
        true,
        Cat::Edit,
        "sahneyi imlecin satırından ikiye böl",
        "split the scene in two at the cursor's line",
    ),
    b(
        "J",
        false,
        true,
        Cat::Edit,
        "sahneyi sonrakiyle birleştir",
        "join the scene with the next one",
    ),
    b(
        "K",
        true,
        false,
        Cat::Panel,
        "karakter ya da komut ara",
        "search characters and commands",
    ),
    b(
        "F",
        true,
        false,
        Cat::Panel,
        "bul ve değiştir",
        "find and replace",
    ),
    // Alt with the key that types "i": İ on a Turkish keyboard, I on a US one.
    b(
        "İ",
        false,
        true,
        Cat::Panel,
        "oyun bilgisi: başlık, yazar, taslak/final",
        "cover details: title, author, draft/final",
    ),
    b(
        "S",
        false,
        true,
        Cat::Panel,
        "imleçteki kelime için yazım önerileri",
        "spelling suggestions for the word at the cursor",
    ),
    b(
        "P",
        false,
        true,
        Cat::Panel,
        "kişi tanımlarını ve oyun bilgisini kaynakta aç",
        "show the cast and cover lines of the source",
    ),
    b("F1", false, false, Cat::View, "bu ekran", "this screen"),
    b(
        "F2",
        false,
        false,
        Cat::View,
        "bütün oyun kaynak olarak",
        "the whole play as source",
    ),
    b(
        "F3",
        false,
        false,
        Cat::View,
        "daktilo: imleç satırı ortada",
        "typewriter: the cursor's line in the middle",
    ),
    b(
        "F4",
        false,
        false,
        Cat::View,
        "sahne odağı: diğer sahneler kapanır",
        "scene focus: the other scenes fold",
    ),
    b(
        "F5",
        false,
        false,
        Cat::File,
        "PDF'i derle (arka planda, tek geçiş)",
        "build the PDF (in the background, one pass)",
    ),
    term(b(
        "O",
        false,
        true,
        Cat::File,
        "PDF'i görüntüleyicide aç",
        "open the PDF in the viewer",
    )),
    term(b(
        "L",
        false,
        true,
        Cat::File,
        "son derlemenin logunu aç",
        "open the last build's log",
    )),
    term(b(
        "F8",
        false,
        false,
        Cat::View,
        "imleç dışını soldur; Shift+F8: imleçteki rolün replikleri parlak, kalanı soluk",
        "dim all but the cursor's line; Shift+F8: the cursor's role lit, the rest faint",
    )),
    term(b(
        "F10",
        false,
        false,
        Cat::File,
        "şerit: sekmelerde ve düğmelerde harfler, ya da oklar; ^F1 katlar; fareyle de",
        "the ribbon: letters on tabs and buttons, or the arrows; ^F1 folds it; by mouse too",
    )),
    b(
        "F9",
        false,
        false,
        Cat::Panel,
        "dağıtım: oyuncular, sahne/rol tablosu, sayılar",
        "casting: actors, the scene/role table, numbers",
    ),
    b(
        "F6",
        false,
        false,
        Cat::View,
        "tema: kâğıt, sepya, gece, yüksek kontrast, koyu",
        "theme: paper, sepia, night, high contrast, dark",
    ),
    b(
        "F7",
        false,
        false,
        Cat::View,
        "sayfa görünümü: A4 yapraklar, sayfa numaraları",
        "page view: A4 sheets, page numbers",
    ),
    b("Z", true, false, Cat::Edit, "geri al", "undo"),
    b("Y", true, false, Cat::Edit, "yinele", "redo"),
    b("C", true, false, Cat::Edit, "kopyala", "copy"),
    b("X", true, false, Cat::Edit, "kes", "cut"),
    b(
        "V",
        true,
        false,
        Cat::Edit,
        "yapıştır; satır başında 'AD: …' satırları replik olur",
        "paste; at a line's start 'NAME: …' lines become lines",
    ),
    b(
        "A",
        true,
        false,
        Cat::Edit,
        "oyunun hepsini seç",
        "select the whole play",
    ),
    b(
        "S",
        true,
        false,
        Cat::File,
        "kaydet (zaten kendiliğinden kaydeder)",
        "save (it saves by itself anyway)",
    ),
    term(b("Q", true, false, Cat::File, "çık", "quit")),
    desk(b(
        "G",
        true,
        false,
        Cat::Panel,
        "pano: sahneleri sürükle bırak, kartlar",
        "the board: drag the scenes, cards",
    )),
    desk(b(
        "Alt",
        false,
        false,
        Cat::File,
        "şeridin harfleri: Alt E A bir eylem ekler",
        "the ribbon's letters: Alt N A adds an action",
    )),
    desk(b(
        "F8",
        false,
        false,
        Cat::View,
        "imleçteki blok dışını soldur",
        "dim all but the cursor's block",
    )),
    desk(b(
        "F12",
        false,
        false,
        Cat::Panel,
        "PDF paneli yanda",
        "the PDF panel beside",
    )),
    term(b(
        "F12",
        false,
        false,
        Cat::File,
        "PDF'i görüntüleyicide aç",
        "open the PDF in a viewer",
    )),
    term(b(
        "G",
        false,
        true,
        Cat::File,
        "derlemenin hata verdiği satıra git",
        "go to the line the build failed on",
    )),
    desk(b(
        "'",
        true,
        false,
        Cat::Panel,
        "imlecin yerini PDF'te göster",
        "show the cursor's place in the PDF",
    )),
    desk(b(
        "W",
        true,
        false,
        Cat::File,
        "oyunu kapat",
        "close the play",
    )),
    desk(b(
        "B",
        true,
        false,
        Cat::Write,
        "seçili yazıyı kalın yap",
        "make the selection bold",
    )),
    desk(b(
        "I",
        true,
        false,
        Cat::Write,
        "seçili yazıyı italik yap",
        "make the selection italic",
    )),
    desk(b(
        "U",
        true,
        false,
        Cat::Write,
        "seçili yazının altını çiz",
        "underline the selection",
    )),
    desk(b("+", true, false, Cat::View, "sayfayı büyüt", "zoom in")),
    desk(b("−", true, false, Cat::View, "sayfayı küçült", "zoom out")),
    desk(b(
        "0",
        true,
        false,
        Cat::View,
        "sayfa boyutunu sıfırla",
        "reset the zoom",
    )),
    b(
        "F1",
        true,
        false,
        Cat::View,
        "şeridi katla ya da aç",
        "fold or unfold the ribbon",
    ),
];

impl Bind {
    /// The cap on `layout`'s keyboard.
    pub fn cap(&self, layout: &str) -> &'static str {
        if layout == "us" && self.cap == "İ" {
            "I"
        } else {
            self.cap
        }
    }

    pub fn what(&self) -> &'static str {
        if lang::is_tr() { self.tr } else { self.en }
    }

    pub fn combo(&self, layout: &str) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.alt {
            s.push_str("Alt+");
        }
        s.push_str(self.cap(layout));
        s
    }
}

/// The keys drawn under the letters, for an app.
pub fn special(app: Only) -> Vec<&'static str> {
    let mut caps = SPECIAL.to_vec();
    if app == Only::Desktop {
        caps.extend(["+", "−", "0", "'"]);
    }
    caps
}

impl Cat {
    /// The word the desktop app colours it by.
    pub fn id(self) -> &'static str {
        match self {
            Cat::File => "file",
            Cat::Edit => "edit",
            Cat::Panel => "panel",
            Cat::View => "view",
            Cat::Write => "write",
        }
    }
}

pub fn rows(layout: &str) -> [&'static [&'static str]; 3] {
    match layout {
        "f" => [
            &["F", "G", "Ğ", "I", "O", "D", "R", "N", "H", "P", "Q", "W"],
            &["U", "İ", "E", "A", "Ü", "T", "K", "M", "L", "Y", "Ş", "X"],
            &["J", "Ö", "V", "C", "Ç", "Z", "S", "B"],
        ],
        "us" => [
            &["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
            &["A", "S", "D", "F", "G", "H", "J", "K", "L"],
            &["Z", "X", "C", "V", "B", "N", "M"],
        ],
        _ => [
            &["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "Ğ", "Ü"],
            &["A", "S", "D", "F", "G", "H", "J", "K", "L", "Ş", "İ"],
            &["Z", "X", "C", "V", "B", "N", "M", "Ö", "Ç"],
        ],
    }
}

pub const FKEYS: [&str; 12] = [
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
];
const SPECIAL: [&str; 10] = ["Ctrl", "Alt", "Tab", "\\", "#", "(", ":", "⌫", "⏎", "Esc"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_key_means_two_things_in_one_app() {
        for app in [Only::Desktop, Only::Light] {
            let mine: Vec<&Bind> = BINDS.iter().filter(|b| b.only.fits(app)).collect();
            for (i, a) in mine.iter().enumerate() {
                for b in &mine[i + 1..] {
                    assert_ne!(a.combo("q"), b.combo("q"), "{app:?}");
                }
            }
        }
    }

    #[test]
    fn every_key_is_on_the_keyboard() {
        for app in [Only::Desktop, Only::Light] {
            for layout in ["q", "f", "us"] {
                let [a, b, c] = rows(layout);
                let special = special(app);
                for k in BINDS.iter().filter(|b| b.only.fits(app)) {
                    let cap = k.cap(layout);
                    assert!(
                        FKEYS.contains(&cap)
                            || [a, b, c].iter().any(|r| r.contains(&cap))
                            || special.contains(&cap),
                        "{cap} on {layout}"
                    );
                }
            }
        }
    }
}
