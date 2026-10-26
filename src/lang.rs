//! The interface language: Turkish or English, chosen in the settings or
//! from the play (`auto`). Every string the app shows passes through `l`.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Tr,
    En,
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(l: Lang) {
    CURRENT.store(if l == Lang::En { 1 } else { 0 }, Ordering::Relaxed);
}

pub fn get() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == 1 {
        Lang::En
    } else {
        Lang::Tr
    }
}

pub fn is_tr() -> bool {
    get() == Lang::Tr
}

/// The Turkish or the English text, by the current language.
pub fn l(tr: &'static str, en: &'static str) -> &'static str {
    if is_tr() { tr } else { en }
}

/// `l` for a pattern: each `{}` takes the next argument.
pub fn f(tr: &str, en: &str, args: &[&dyn std::fmt::Display]) -> String {
    let pattern = if is_tr() { tr } else { en };
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut args = args.iter();
    let mut rest = pattern;
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        if let Some(a) = args.next() {
            out.push_str(&a.to_string());
        }
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

/// "tr", "en" or "auto" from the settings, resolved: auto follows the play
/// (Turkish letters in it) or, without one, the system's language.
pub fn resolve(setting: &str, play: Option<&str>) -> Lang {
    match setting {
        "tr" => Lang::Tr,
        "en" => Lang::En,
        _ => match play {
            Some(text) => of_text(text),
            None => system(),
        },
    }
}

/// Turkish when the text has Turkish letters, English otherwise.
pub fn of_text(text: &str) -> Lang {
    let turkish = text
        .chars()
        .filter(|c| "çğıöşüÇĞİÖŞÜ".contains(*c))
        .take(3)
        .count();
    if turkish >= 3 {
        Lang::Tr
    } else if text.trim().is_empty() {
        system()
    } else {
        Lang::En
    }
}

/// The system's language from LANG / LC_ALL / LC_MESSAGES.
pub fn system() -> Lang {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(v) = std::env::var(var)
            && !v.is_empty()
        {
            return if v.starts_with("tr") {
                Lang::Tr
            } else {
                Lang::En
            };
        }
    }
    Lang::Tr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_and_detection() {
        set(Lang::Tr);
        assert_eq!(
            f("{} eklendi: \\{}", "{} added: \\{}", &[&"SELİN", &"se"]),
            "SELİN eklendi: \\se"
        );
        assert_eq!(l("kaydet", "save"), "kaydet");
        assert_eq!(of_text("Şu ağaçlığın gölgesi"), Lang::Tr);
        assert_eq!(of_text("The quick brown fox"), Lang::En);
        assert_eq!(resolve("en", Some("Şu ağaç")), Lang::En);
    }
}
