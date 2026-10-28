//! What would stop XeLaTeX, found before it runs: a brace left open or one
//! too many, a command nobody defined, and the characters TeX reads as
//! something else (`&`, `$`, `^`, a `_` outside a line, a `%` before a
//! number). Typing and pasting in the writer escape these already; they come
//! from the source view and from edits made elsewhere. The ones with an
//! obvious fix are fixed quietly.

use crate::lex::Kind;
use crate::render::Doc;
use crate::tex::{self, Edit};
use crate::{class, lang};
use std::collections::HashSet;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq)]
pub enum What {
    /// A `{` never closed.
    Unclosed,
    /// A `}` that closes nothing.
    ExtraClose,
    /// `\name` nobody defined; `alias` when it is a character's alias with
    /// letters run on (`\sein` for `\se`+`in`).
    Unknown { name: String, alias: Option<String> },
    /// `&`, `$`, `^` or `_` as TeX reads them, or `%` before a number
    /// (`%50`), which hides the rest of the line.
    Special(char),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub span: Range<usize>,
    pub what: What,
}

impl Problem {
    /// The quiet fix, if there is an obvious one.
    pub fn fix(&self) -> Option<Edit> {
        let at = self.span.start;
        match &self.what {
            What::Special('^') => Some(Edit::new(self.span.clone(), "\\^{}")),
            What::Special(_) => Some(Edit::new(at..at, "\\")),
            _ => None,
        }
    }

    /// One line for the status line.
    pub fn message(&self) -> String {
        match &self.what {
            What::Unclosed => lang::l(
                "Açılan { kapanmamış: PDF burada durur.",
                "A { never closed: the PDF stops here.",
            )
            .into(),
            What::ExtraClose => lang::l(
                "Fazladan }: hiçbir şeyi kapatmıyor.",
                "An extra }: it closes nothing.",
            )
            .into(),
            What::Unknown {
                name,
                alias: Some(a),
            } => lang::f(
                "\\{} tanımlı değil: \\{} ve ek mi? \\{}{}{} yaz.",
                "\\{} isn't defined: \\{} and a suffix? Write \\{}{}{}.",
                &[name, a, a, &"{}", &&name[a.len()..]],
            ),
            What::Unknown { name, alias: None } => lang::f(
                "\\{} tanımlı değil: PDF burada durur.",
                "\\{} isn't defined: the PDF stops here.",
                &[name],
            ),
            What::Special(c) => lang::f(
                "Kaçırılmamış {}: kayıtta düzelir.",
                "An unescaped {}: fixed on save.",
                &[c],
            ),
        }
    }
}

/// The problems in the play's body, in order.
pub fn problems(text: &str, doc: &Doc, known: &HashSet<String>) -> Vec<Problem> {
    let aliases: Vec<&str> = doc
        .cast
        .characters
        .iter()
        .map(|c| c.alias.as_str())
        .collect();
    let mut out = Vec::new();
    for b in &doc.blocks[doc.body.clone()] {
        let dialogue = match &b.kind {
            Kind::Comment | Kind::Blank => continue,
            // `#` headings are read by the class to the line's end.
            Kind::Act { hash: true, .. }
            | Kind::Scene { hash: true, .. }
            | Kind::Section { hash: true, .. } => continue,
            Kind::Line { .. } | Kind::Do { .. } => true,
            _ => false,
        };
        let code = &text[b.span.clone()];
        // Tables and other environments use & and $ on purpose.
        let env = code.contains("\\begin{");
        scan(
            text,
            b.span.clone(),
            dialogue,
            env,
            known,
            &aliases,
            &mut out,
        );
    }
    out
}

fn scan(
    text: &str,
    span: Range<usize>,
    dialogue: bool,
    env: bool,
    known: &HashSet<String>,
    aliases: &[&str],
    out: &mut Vec<Problem>,
) {
    let mut open: Vec<usize> = Vec::new();
    let mut i = span.start;
    let mut line_start = span.start;
    while i < span.end {
        let c = text[i..].chars().next().unwrap();
        let next = text[i + c.len_utf8()..span.end].chars().next();
        match c {
            '\n' => line_start = i + 1,
            '\\' => {
                let rest = &text[i + 1..span.end];
                let len: usize = rest
                    .chars()
                    .take_while(|&c| tex::is_letter(c))
                    .map(char::len_utf8)
                    .sum();
                if len == 0 {
                    // A control symbol: \%, \\, \{ …
                    i += 1 + rest.chars().next().map_or(0, char::len_utf8);
                    continue;
                }
                let name = &rest[..len];
                if !known.contains(name) {
                    let alias = aliases
                        .iter()
                        .filter(|a| a.len() < name.len() && name.starts_with(**a))
                        .max_by_key(|a| a.len())
                        .map(|a| a.to_string());
                    out.push(Problem {
                        span: i..i + 1 + len,
                        what: What::Unknown {
                            name: name.to_string(),
                            alias,
                        },
                    });
                }
                i += 1 + len;
                continue;
            }
            '%' => {
                let before = text[line_start..i].trim();
                if next.is_some_and(|n| n.is_ascii_digit()) && !before.is_empty() {
                    out.push(Problem {
                        span: i..i + 1,
                        what: What::Special('%'),
                    });
                } else {
                    // A comment: to the line's end.
                    i = text[i..span.end].find('\n').map_or(span.end, |n| i + n);
                    continue;
                }
            }
            '{' => open.push(i),
            '}' => {
                if open.pop().is_none() {
                    out.push(Problem {
                        span: i..i + 1,
                        what: What::ExtraClose,
                    });
                }
            }
            '&' | '$' if !env => out.push(Problem {
                span: i..i + 1,
                what: What::Special(c),
            }),
            '^' if !env && next != Some('^') => out.push(Problem {
                span: i..i + 1,
                what: What::Special('^'),
            }),
            '_' if !dialogue && !env => out.push(Problem {
                span: i..i + 1,
                what: What::Special('_'),
            }),
            _ => {}
        }
        i += c.len_utf8();
    }
    if let Some(&at) = open.first() {
        // To the line's end, so it shows under what follows the brace.
        let end = text[at..span.end].find('\n').map_or(span.end, |n| at + n);
        out.push(Problem {
            span: at..end.max(at + 1),
            what: What::Unclosed,
        });
    }
}

/// Every command the play can use: the class's, the cast's, what the
/// preamble defines, and LaTeX's own that plays use.
pub fn known(text: &str, doc: &Doc) -> HashSet<String> {
    let mut out: HashSet<String> = LATEX.iter().map(|s| s.to_string()).collect();
    for (_, body) in class::FILES {
        defined(body, &mut out);
    }
    defined(&text[doc.parts.preamble.clone()], &mut out);
    for c in &doc.cast.characters {
        for s in ["", "do", "font"] {
            out.insert(format!("{}{s}", c.alias));
        }
    }
    out
}

/// Names after `\newcommand`, `\def`, `\NewDocumentCommand` and the like.
fn defined(src: &str, out: &mut HashSet<String>) {
    const DEFINERS: &[&str] = &[
        "newcommand",
        "renewcommand",
        "providecommand",
        "NewDocumentCommand",
        "RenewDocumentCommand",
        "ProvideDocumentCommand",
        "DeclareDocumentCommand",
        "NewCommandCopy",
        "DeclareRobustCommand",
        "newlength",
        "newif",
        "def",
        "gdef",
        "edef",
        "xdef",
        "let",
        "newfontfamily",
        "cs_new_protected:Npn",
        "cs_new:Npn",
    ];
    let mut rest = src;
    while let Some(i) = rest.find('\\') {
        rest = &rest[i + 1..];
        let len = rest
            .find(|c: char| !(c.is_alphabetic() || c == '_' || c == ':'))
            .unwrap_or(rest.len());
        let word = &rest[..len];
        if DEFINERS.contains(&word) {
            let after = rest[len..].trim_start_matches([' ', '\t', '\n', '{', '*']);
            if let Some(name) = after.strip_prefix('\\') {
                let n: String = name.chars().take_while(|&c| tex::is_letter(c)).collect();
                // \newif\iffoo gives \foofalse and \footrue too.
                if let Some(base) = n.strip_prefix("if").filter(|_| word == "newif") {
                    out.insert(format!("{base}true"));
                    out.insert(format!("{base}false"));
                }
                if !n.is_empty() {
                    out.insert(n);
                }
            }
        }
    }
}

/// LaTeX's commands that a play's body may use.
const LATEX: &[&str] = &[
    // The page and its parts.
    "maketitle",
    "title",
    "author",
    "date",
    "and",
    "thanks",
    "today",
    "tableofcontents",
    "clearpage",
    "cleardoublepage",
    "newpage",
    "pagebreak",
    "nopagebreak",
    "linebreak",
    "nolinebreak",
    "newline",
    "enlargethispage",
    "thispagestyle",
    "pagestyle",
    "pagenumbering",
    "chapter",
    "section",
    "subsection",
    "subsubsection",
    "paragraph",
    "addcontentsline",
    "label",
    "ref",
    "pageref",
    "footnote",
    "footnotemark",
    "footnotetext",
    "input",
    "include",
    "begin",
    "end",
    "item",
    "caption",
    "hline",
    "cline",
    "multicolumn",
    "includegraphics",
    "url",
    "href",
    // Spacing.
    "par",
    "noindent",
    "indent",
    "vspace",
    "hspace",
    "vskip",
    "hskip",
    "bigskip",
    "medskip",
    "smallskip",
    "vfill",
    "hfill",
    "quad",
    "qquad",
    "enspace",
    "thinspace",
    "negthinspace",
    "null",
    "relax",
    "centering",
    "raggedright",
    "raggedleft",
    "nobreak",
    "allowbreak",
    "break",
    "sloppy",
    "fussy",
    "mbox",
    "makebox",
    "parbox",
    "hbox",
    "vbox",
    "phantom",
    "hphantom",
    "vphantom",
    "strut",
    "protect",
    "setlength",
    "addtolength",
    "setcounter",
    "addtocounter",
    "stepcounter",
    "refstepcounter",
    "arabic",
    "roman",
    "Roman",
    "alph",
    "Alph",
    "value",
    "the",
    "thepage",
    "textwidth",
    "linewidth",
    "baselineskip",
    "parindent",
    "parskip",
    "fill",
    "stretch",
    // Type.
    "textit",
    "textbf",
    "textsc",
    "textsl",
    "textup",
    "textmd",
    "textrm",
    "textsf",
    "texttt",
    "textnormal",
    "emph",
    "underline",
    "textsuperscript",
    "textsubscript",
    "itshape",
    "bfseries",
    "scshape",
    "slshape",
    "upshape",
    "mdseries",
    "rmfamily",
    "sffamily",
    "ttfamily",
    "normalfont",
    "em",
    "it",
    "bf",
    "sc",
    "rm",
    "sf",
    "tt",
    "tiny",
    "scriptsize",
    "footnotesize",
    "small",
    "normalsize",
    "large",
    "Large",
    "LARGE",
    "huge",
    "Huge",
    "color",
    "textcolor",
    "fontspec",
    "selectfont",
    "fontsize",
    "selectlanguage",
    "foreignlanguage",
    "textturkish",
    "textenglish",
    "textrussian",
    "textgreek",
    "textfrench",
    "textgerman",
    // Characters.
    "ldots",
    "dots",
    "textellipsis",
    "LaTeX",
    "TeX",
    "XeLaTeX",
    "S",
    "P",
    "copyright",
    "textcopyright",
    "textregistered",
    "texttrademark",
    "textendash",
    "textemdash",
    "textasciitilde",
    "textasciicircum",
    "textbackslash",
    "textbar",
    "textless",
    "textgreater",
    "textbullet",
    "textdegree",
    "textperiodcentered",
    "textquoteleft",
    "textquoteright",
    "textquotedblleft",
    "textquotedblright",
    "guillemotleft",
    "guillemotright",
    "guilsinglleft",
    "guilsinglright",
    "quotedblbase",
    "quotesinglbase",
    "lq",
    "rq",
    "i",
    "j",
    "o",
    "O",
    "l",
    "L",
    "ss",
    "ae",
    "AE",
    "oe",
    "OE",
    "aa",
    "AA",
    "c",
    "u",
    "v",
    "H",
    "r",
    "d",
    "b",
    "t",
    "k",
    "dag",
    "ddag",
    "pounds",
    "euro",
    "textdollar",
    "textsection",
    "textparagraph",
    "slash",
    "space",
    // Conditionals the preamble's own commands may leave in the body.
    "else",
    "fi",
    "ifx",
    "ifthenelse",
    // The class's language and font helpers, defined by name elsewhere.
    "ru",
    "en",
    "de",
    "fr",
    "el",
];

/// The quiet fixes for `problems`, skipping those in `keep` (the block
/// being written in the source view).
pub fn fixes(problems: &[Problem], keep: Option<Range<usize>>) -> Vec<Edit> {
    problems
        .iter()
        .filter(|p| keep.as_ref().is_none_or(|k| !k.contains(&p.span.start)))
        .filter_map(Problem::fix)
        .collect()
}

/// Whether one of them stops the build: any but the quiet ones.
pub fn stops(problems: &[Problem]) -> Option<&Problem> {
    problems.iter().find(|p| p.fix().is_none())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAY: &str = include_str!("../examples/ornek-oyun.tex");

    fn check(t: &str) -> Vec<(String, What)> {
        let doc = Doc::new(t);
        problems(t, &doc, &known(t, &doc))
            .into_iter()
            .map(|p| (t[p.span].to_string(), p.what))
            .collect()
    }

    fn play(body: &str) -> String {
        PLAY.replace("\\end{document}", &format!("{body}\n\\end{{document}}"))
    }

    #[test]
    fn the_example_is_clean() {
        assert_eq!(check(PLAY), vec![]);
    }

    #[test]
    fn what_would_stop_xelatex() {
        let got = check(&play("\n\\se Bir \\ic{gülerek kaçar.\n\n\\action{Sonra}\n"));
        assert_eq!(got, vec![("{gülerek kaçar.".into(), What::Unclosed)]);
        let got = check(&play("\n\\action{Kapı}}\n"));
        assert_eq!(got, vec![("}".into(), What::ExtraClose)]);
        let got = check(&play("\n\\oz Gel \\sein yanına. \\foo da.\n"));
        assert_eq!(
            got,
            vec![
                (
                    "\\sein".into(),
                    What::Unknown {
                        name: "sein".into(),
                        alias: Some("se".into())
                    }
                ),
                (
                    "\\foo".into(),
                    What::Unknown {
                        name: "foo".into(),
                        alias: None
                    }
                ),
            ]
        );
    }

    #[test]
    fn quiet_fixes() {
        let t = play(
            "\n\\action{a_b} % bir not\n\\se Ali & Veli %50 indirim, 5$ ve x^2.\n\\se _altı_ çizili\n",
        );
        let doc = Doc::new(&t);
        let p = problems(&t, &doc, &known(&t, &doc));
        let whats: Vec<&What> = p.iter().map(|p| &p.what).collect();
        assert_eq!(
            whats,
            vec![
                &What::Special('_'),
                &What::Special('&'),
                &What::Special('%'),
                &What::Special('$'),
                &What::Special('^'),
            ]
        );
        assert!(stops(&p).is_none());
        let fixed = tex::apply(&t, &fixes(&p, None));
        assert!(fixed.contains(
            "\\action{a\\_b} % bir not\n\\se Ali \\& Veli \\%50 indirim, 5\\$ ve x\\^{}2."
        ));
        assert_eq!(check(&fixed), vec![]);
    }

    #[test]
    fn the_preamble_defines_commands() {
        let t = PLAY.replace(
            "\\begin{document}",
            "\\newcommand{\\sis}{Sis}\n\\def\\yagmur{x}\n\\begin{document}",
        );
        let t = t.replace(
            "\\end{document}",
            "\\action{\\sis ve \\yagmur}\n\\end{document}",
        );
        assert_eq!(check(&t), vec![]);
    }
}
