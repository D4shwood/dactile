//! What the commands of a play do, for the Help tab's side panel. The class
//! guide in docs/ tells it at length; this is the short version.

use crate::lang;

pub struct Entry {
    pub group: Group,
    /// As it is written in a play.
    pub name: &'static str,
    pub tr: &'static str,
    pub en: &'static str,
    /// A line to copy; empty when the name says it all.
    pub example: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Parts,
    Cast,
    Lines,
    Directions,
    Cover,
}

impl Group {
    pub const ALL: [Group; 5] = [
        Group::Parts,
        Group::Cast,
        Group::Lines,
        Group::Directions,
        Group::Cover,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Group::Parts => lang::l("Bölümler", "Parts"),
            Group::Cast => lang::l("Kişiler", "The cast"),
            Group::Lines => lang::l("Replikler", "Lines"),
            Group::Directions => lang::l("Yönergeler", "Directions"),
            Group::Cover => lang::l("Kapak ve görünüm", "Cover and look"),
        }
    }
}

impl Entry {
    pub fn what(&self) -> &'static str {
        if lang::is_tr() { self.tr } else { self.en }
    }
}

const fn e(
    group: Group,
    name: &'static str,
    tr: &'static str,
    en: &'static str,
    example: &'static str,
) -> Entry {
    Entry {
        group,
        name,
        tr,
        en,
        example,
    }
}

use Group::*;

pub const ENTRIES: &[Entry] = &[
    e(
        Parts,
        "#perde",
        "Numaralı bir perde açar. Hiç yazılmazsa oyun tek perdedir.",
        "Opens a numbered act (#act). Without any, the play has one act.",
        "#perde Kar Yağarken",
    ),
    e(
        Parts,
        "#sahne",
        "Numaralı bir sahne açar; altına o sahnede olanlar kendiliğinden yazılır.",
        "Opens a numbered scene (#scene); who is in it is written beneath by itself.",
        "#sahne Değirmen — Akşam",
    ),
    e(
        Parts,
        "#önsahne",
        "Numarasız ön sahne. Sonraki sahne yine 1 olur.",
        "A prologue without a number (#prologue). The next scene is still 1.",
        "#önsahne Yol",
    ),
    e(
        Parts,
        "#önsöz",
        "Kendi sayfasında önsöz.",
        "A preface on its own page (#preface).",
        "",
    ),
    e(
        Parts,
        "#notlar",
        "Kendi sayfasında yönetmene notlar.",
        "Notes for the director on their own page (#notes).",
        "",
    ),
    e(
        Cast,
        "\\karakter",
        "Bir kişi tanımlar: adı, kısaltması, tanımı. Kısaltma \\se, \\sedo gibi komutları verir.",
        "Defines a character: name, short name, description. The short name gives \\se, \\sedo and so on.",
        "\\karakter[Köy öğretmeni]{SELİN}{se}",
    ),
    e(
        Cast,
        "\\sirkarakter",
        "Kişiler sayfasında görünmeyen kişi; replikleri normal yazılır. İngilizcesi \\secretcharacter.",
        "A character kept off the cast page; their lines work as usual. Also \\secretcharacter.",
        "\\sirkarakter{YABANCI}{ya}",
    ),
    e(
        Cast,
        "\\kisiler",
        "Kişiler sayfasını basar ve sayfayı kapatır. \\kisiler* yazılırsa oyun aynı sayfada başlar.",
        "Prints the cast page and ends it. With \\kisiler* the play starts on the same page.",
        "",
    ),
    e(
        Cast,
        "\\dramatispersonae",
        "Kişiler başlığını oyunun dili ne olursa olsun Latince yapar.",
        "Makes the cast page's title Latin, whatever the play's language.",
        "",
    ),
    e(
        Cast,
        "\\digerkisiler",
        "Kişiler sayfasına tek tek tanımlanmayan kişileri yazar. İngilizcesi \\othercast.",
        "Lists on the cast page those not defined one by one. Also \\othercast.",
        "\\digerkisiler{Köylüler}",
    ),
    e(
        Lines,
        "SELİN:",
        "Satır başında yazınca SELİN'in repliği olur.",
        "At a line's start it becomes SELİN's line.",
        "SELİN: Kar dinmedi.",
    ),
    e(
        Lines,
        "\\se",
        "Satır başında kişinin repliği, replik içinde kişinin adı. Kısaltma \\karakter'den gelir.",
        "At a line's start the character's line, inside a line their name. The short name comes from \\karakter.",
        "\\se Kar dinmedi.",
    ),
    e(
        Lines,
        "\\sedo",
        "Kişinin yaptığı bir yönerge; kaynak satırının sonunda biter. Zincirlenir: \\sedo\\ozdo.",
        "A direction for what the character does; it ends with its source line. They chain: \\sedo\\ozdo.",
        "\\sedo Kapıya bakar.",
    ),
    e(
        Lines,
        "_kelime_",
        "Replik içinde altı çizili yazar.",
        "Underlines inside a line.",
        "Bu _kez_ gitme.",
    ),
    e(
        Lines,
        "%",
        "Satır sonunda sonraki satırı buna ekler.",
        "At a line's end it joins the next line to this one.",
        "",
    ),
    e(
        Directions,
        "\\ic",
        "Replik içinde parantezli, italik yönerge.",
        "A direction inside a line, in brackets and italics.",
        "\\ic{gülerek}",
    ),
    e(
        Directions,
        "\\action",
        "Replikler arasında eylem paragrafı.",
        "An action paragraph between lines.",
        "\\action{Işıklar yavaşça açılır.}",
    ),
    e(
        Directions,
        "\\stagedir",
        "Ortalanmış sahne yönergesi.",
        "A centred stage direction.",
        "\\stagedir{Karlı bir dağ köyü.}",
    ),
    e(
        Directions,
        "\\middir",
        "Sağa yaslı italik yönerge.",
        "An italic direction on the right.",
        "\\middir{Kapı çalar.}",
    ),
    e(
        Directions,
        "\\beat",
        "\"(Sus.)\" yazar.",
        "Writes a pause.",
        "",
    ),
    e(
        Directions,
        "\\girer",
        "Giriş yönergesi: (SELİN ile OZAN girerler.)",
        "An entrance: SELİN and OZAN enter.",
        "\\girer{se,oz}",
    ),
    e(
        Directions,
        "\\cikar",
        "Çıkış yönergesi: (SELİN çıkar.)",
        "An exit: SELİN leaves.",
        "\\cikar{se}",
    ),
    e(
        Directions,
        "\\dipnot",
        "Dipnot; replikte ve yönergede çalışır.",
        "A footnote, in lines and directions alike.",
        "\\dipnot{Eski bir türkü.}",
    ),
    e(
        Cover,
        "\\title",
        "Oyunun adı.",
        "The play's title.",
        "\\title{Kar Masalı}",
    ),
    e(
        Cover,
        "\\author",
        "Yazarın adı.",
        "The author's name.",
        "\\author{Deniz Yılmaz}",
    ),
    e(
        Cover,
        "\\editedby",
        "Kapakta yazarın altına \"Düzenleyen: …\" yazar.",
        "Writes an editor's name under the author on the cover.",
        "\\editedby{Ada Kaya}",
    ),
    e(
        Cover,
        "\\draft",
        "Taslak numarası; kapakta TASLAK olarak görünür.",
        "The draft number, shown on the cover.",
        "\\draft{0.1}",
    ),
    e(
        Cover,
        "\\copyrightnotice",
        "Kapağa telif satırı. \\address, \\email, \\telephone da aynı yolla çalışır; boş olan basılmaz.",
        "A copyright line on the cover. \\address, \\email and \\telephone work alike; empty ones aren't printed.",
        "\\copyrightnotice{© 2026 Deniz Yılmaz}",
    ),
    e(
        Cover,
        "\\coverstyle",
        "Kapaktaki bir öğenin hizası, boyutu, stili ve boşlukları.",
        "A cover part's alignment, size, style and spacing.",
        "\\coverstyle{title}{align=left, size=Huge}",
    ),
    e(
        Cover,
        "\\caststyle",
        "Kişiler sayfasındaki bir öğenin hizası, boyutu, stili ve boşlukları.",
        "A cast page part's alignment, size, style and spacing.",
        "\\caststyle{title}{align=center, size=Large}",
    ),
    e(
        Cover,
        "\\scenetitleflush",
        "Sahne başlıklarını sola ya da sağa yaslar.",
        "Sets scene headings left or right.",
        "\\scenetitleflush[left]",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_group_has_commands() {
        for g in Group::ALL {
            assert!(ENTRIES.iter().any(|e| e.group == g));
        }
    }
}
