//! A small tree of an XML part of a .docx or .odt: a play's document is a
//! few megabytes at most, and a tree keeps the readers plain.

use anyhow::{Context, Result};
use quick_xml::Reader;
use quick_xml::events::Event;

#[derive(Debug, Default)]
pub struct Element {
    /// With its prefix: `w:p`, `text:span`.
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub kids: Vec<Kid>,
}

#[derive(Debug)]
pub enum Kid {
    El(Element),
    Text(String),
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.kids.iter().filter_map(|k| match k {
            Kid::El(e) => Some(e),
            Kid::Text(_) => None,
        })
    }

    pub fn child(&self, name: &str) -> Option<&Element> {
        self.elements().find(|e| e.name == name)
    }

    /// The first element named `name` at any depth, this one included.
    pub fn find(&self, name: &str) -> Option<&Element> {
        if self.name == name {
            return Some(self);
        }
        self.elements().find_map(|e| e.find(name))
    }

    /// All the text inside, in order.
    pub fn text(&self) -> String {
        let mut s = String::new();
        for k in &self.kids {
            match k {
                Kid::Text(t) => s.push_str(t),
                Kid::El(e) => s.push_str(&e.text()),
            }
        }
        s
    }
}

/// The document's root element.
pub fn parse(xml: &str) -> Result<Element> {
    let mut reader = Reader::from_str(xml);
    let mut stack = vec![Element::default()];
    loop {
        match reader
            .read_event()
            .context(crate::lang::l("XML okunamadı", "reading XML"))?
        {
            Event::Start(e) => stack.push(element(&e)?),
            Event::Empty(e) => {
                let el = element(&e)?;
                push(&mut stack, Kid::El(el));
            }
            Event::End(_) => {
                if stack.len() > 1 {
                    let el = stack.pop().unwrap_or_default();
                    push(&mut stack, Kid::El(el));
                }
            }
            Event::Text(t) => push(&mut stack, Kid::Text(t.xml10_content().into_owned())),
            Event::CData(t) => push(&mut stack, Kid::Text(t.xml10_content().into_owned())),
            Event::GeneralRef(r) => {
                let c = match r.resolve_char_ref().ok().flatten() {
                    Some(c) => Some(c),
                    None => match r.as_ref() {
                        "amp" => Some('&'),
                        "lt" => Some('<'),
                        "gt" => Some('>'),
                        "quot" => Some('"'),
                        "apos" => Some('\''),
                        _ => None,
                    },
                };
                if let Some(c) = c {
                    push(&mut stack, Kid::Text(c.to_string()));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    while stack.len() > 1 {
        let el = stack.pop().unwrap_or_default();
        push(&mut stack, Kid::El(el));
    }
    let root = stack.pop().unwrap_or_default();
    root.kids
        .into_iter()
        .find_map(|k| match k {
            Kid::El(e) => Some(e),
            Kid::Text(_) => None,
        })
        .context(crate::lang::l("boş bir XML parçası", "an empty XML part"))
}

fn element(e: &quick_xml::events::BytesStart) -> Result<Element> {
    let name = e.name().as_ref().to_string();
    let mut attrs = Vec::new();
    for a in e.attributes().with_checks(false) {
        let a = a.context(crate::lang::l(
            "bir XML özniteliği okunamadı",
            "reading an XML attribute",
        ))?;
        let key = a.key.as_ref().to_string();
        let value = a
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .context(crate::lang::l(
                "bir XML özniteliği okunamadı",
                "reading an XML attribute",
            ))?;
        attrs.push((key, value.into_owned()));
    }
    Ok(Element {
        name,
        attrs,
        kids: Vec::new(),
    })
}

/// Text next to text is one piece: an entity splits a word in the events.
fn push(stack: &mut [Element], kid: Kid) {
    let Some(top) = stack.last_mut() else { return };
    if let (Kid::Text(t), Some(Kid::Text(last))) = (&kid, top.kids.last_mut()) {
        last.push_str(t);
        return;
    }
    top.kids.push(kid);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_tree() {
        let root = parse(
            "<?xml version=\"1.0\"?><a x=\"1 &amp; 2\"><b>Kar &amp; Masal&#x131;</b><c/>son</a>",
        )
        .unwrap();
        assert_eq!(root.name, "a");
        assert_eq!(root.attr("x"), Some("1 & 2"));
        assert_eq!(root.child("b").unwrap().text(), "Kar & Masalı");
        assert!(root.find("c").is_some());
        assert_eq!(root.text(), "Kar & Masalıson");
    }
}
