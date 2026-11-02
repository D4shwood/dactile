//! Plain text and Markdown, and the joining of lines that only wrapped
//! (which a PDF's text needs too).

use super::{Format, Para, Run, Source, shape};

pub fn read(text: &str, markdown: bool) -> Source {
    let lines: Vec<String> = text
        .replace("\r\n", "\n")
        .split('\n')
        .map(|l| {
            if markdown {
                l.strip_prefix("> ").unwrap_or(l).to_string()
            } else {
                l.to_string()
            }
        })
        .collect();
    let paras = if markdown {
        markdown_paras(&lines)
    } else {
        unwrap(&lines, false)
            .iter()
            .map(|p| Para::plain(p))
            .collect()
    };
    Source {
        format: if markdown {
            Format::Markdown
        } else {
            Format::Text
        },
        paras,
        title: None,
        author: None,
    }
}

/// Markdown: `# Heading` lines, and the rest as paragraphs with their
/// `*italics*` and `**bold**`.
fn markdown_paras(lines: &[String]) -> Vec<Para> {
    let mut out = Vec::new();
    let mut chunk: Vec<String> = Vec::new();
    let flush = |chunk: &mut Vec<String>, out: &mut Vec<Para>| {
        for p in unwrap(chunk, true) {
            out.push(Para {
                runs: inline(&p),
                ..Para::default()
            });
        }
        chunk.clear();
    };
    for l in lines {
        let t = l.trim_start();
        let hashes = t.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) && t[hashes..].starts_with(' ') {
            flush(&mut chunk, &mut out);
            out.push(Para {
                runs: inline(t[hashes..].trim().trim_end_matches('#').trim()),
                heading: true,
                ..Para::default()
            });
        } else {
            chunk.push(l.clone());
        }
    }
    flush(&mut chunk, &mut out);
    out
}

/// `*x*` and `_x_` in italics, `**x**` and `__x__` in bold; `\*` is a star.
fn inline(s: &str) -> Vec<Run> {
    let chars: Vec<char> = s.chars().collect();
    let mut runs = Vec::new();
    let (mut italic, mut bold) = (false, false);
    let mut text = String::new();
    let mut i = 0;
    let flush = |text: &mut String, runs: &mut Vec<Run>, italic: bool, bold: bool| {
        if !text.is_empty() {
            runs.push(Run {
                text: std::mem::take(text),
                italic,
                bold,
            });
        }
    };
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() && "*_\\#".contains(chars[i + 1]) {
            text.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == '*' || c == '_' {
            // An underscore inside a word is a letter.
            let inside = c == '_'
                && i > 0
                && chars[i - 1].is_alphanumeric()
                && chars.get(i + 1).is_some_and(|n| n.is_alphanumeric());
            if !inside {
                let double = chars.get(i + 1) == Some(&c);
                flush(&mut text, &mut runs, italic, bold);
                if double {
                    bold = !bold;
                    i += 2;
                } else {
                    italic = !italic;
                    i += 1;
                }
                continue;
            }
        }
        text.push(c);
        i += 1;
    }
    flush(&mut text, &mut runs, italic, bold);
    runs
}

/// Lines into paragraphs: a blank line or a line that starts something
/// (a name, a heading, a parenthesis) starts one. In between, a line the
/// page width broke is joined to the one before (a word hyphenated across
/// them made whole), a short one kept on a line of its own, as in a poem.
/// Where no line wraps (some run on for hundreds of characters) every line
/// is a paragraph; `soft` (Markdown) joins every line, but where one ends
/// in two spaces or a backslash.
pub(super) fn unwrap(lines: &[String], soft: bool) -> Vec<String> {
    let mut lens: Vec<usize> = lines
        .iter()
        .map(|l| l.trim_end().chars().count())
        .filter(|&n| n > 0)
        .collect();
    lens.sort_unstable();
    let width = lens.get(lens.len() * 9 / 10).copied().unwrap_or(0);
    // Lines all short (a list, a poem) did not wrap; nor did very long ones.
    let wrapped = !soft && (40..=110).contains(&width);
    let mut out: Vec<String> = Vec::new();
    let mut cur: Option<String> = None;
    let mut prev_len = 0;
    let mut prev_hard = false;
    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            out.extend(cur.take());
            continue;
        }
        let len = line.trim_end().chars().count();
        let hard = soft && (line.ends_with("  ") || line.trim_end().ends_with('\\'));
        let t = if soft {
            t.trim_end_matches('\\').trim_end()
        } else {
            t
        };
        match cur.as_mut() {
            Some(p) if !shape::starts_item(t) && !shape::is_heading(p) && (soft || wrapped) => {
                let join = if soft {
                    !prev_hard
                } else {
                    prev_len * 10 >= width * 7
                };
                if join {
                    let hyphen = p
                        .strip_suffix('-')
                        .is_some_and(|b| b.ends_with(char::is_alphabetic))
                        && t.starts_with(char::is_lowercase);
                    if hyphen {
                        p.pop();
                    } else {
                        p.push(' ');
                    }
                } else {
                    p.push('\n');
                }
                p.push_str(t);
            }
            _ => {
                out.extend(cur.take());
                cur = Some(t.to_string());
            }
        }
        prev_len = len;
        prev_hard = hard;
    }
    out.extend(cur);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        s.split('\n').map(str::to_string).collect()
    }

    #[test]
    fn wrapped_lines_join_and_poems_stay() {
        let text = "SELİN: Bu cümle sayfanın sonuna kadar gidiyor ve orada kı-\n\
                    rılıyor, ama aslında tek bir paragraf olarak okunmalı.\n\
                    OZAN: Kısa.\n\
                    Bir dize,\n\
                    bir dize daha.\n\
                    \n\
                    (Kapı açılır, uzun bir sessizlik olur ve herkes birbirine\n\
                    bakar.)";
        let p = unwrap(&lines(text), false);
        assert_eq!(
            p,
            [
                "SELİN: Bu cümle sayfanın sonuna kadar gidiyor ve orada kırılıyor, ama aslında tek bir paragraf olarak okunmalı.",
                "OZAN: Kısa.\nBir dize,\nbir dize daha.",
                "(Kapı açılır, uzun bir sessizlik olur ve herkes birbirine bakar.)",
            ]
        );
    }

    #[test]
    fn markdown_styles() {
        let src = read(
            "# Kar Masalı\n\nSELİN: Gel *(gülerek)* buraya,  \nhemen.\n**OZAN**: Geliyorum\\*.",
            true,
        );
        assert!(src.paras[0].heading);
        assert_eq!(src.paras[0].text(), "Kar Masalı");
        assert_eq!(src.paras[1].text(), "SELİN: Gel (gülerek) buraya,\nhemen.");
        assert!(src.paras[1].runs[1].italic);
        assert!(src.paras[2].runs[0].bold);
        assert_eq!(src.paras[2].text(), "OZAN: Geliyorum*.");
    }
}
