//! Drawing the writer.

use super::*;

impl Writer {
    pub(super) fn keep_visible(&mut self) {
        let Some(cur) = self.cursor_stop() else {
            return;
        };
        let h = self.text_height();
        if self.typewriter {
            self.scroll = cur.row.saturating_sub(h / 2);
            return;
        }
        let pad = 3.min(h / 3);
        if cur.row < self.scroll + pad {
            self.scroll = cur.row.saturating_sub(pad);
        } else if cur.row + pad >= self.scroll + h {
            self.scroll = (cur.row + pad + 1).saturating_sub(h);
        }
    }

    pub(super) fn text_height(&self) -> usize {
        self.height.saturating_sub(3).max(1) as usize
    }

    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        let kind =
            (!matches!(self.popup, Popup::None)).then(|| std::mem::discriminant(&self.popup));
        if kind != self.popup_kind {
            self.popup_kind = kind;
            self.popup_born = Instant::now();
        }
        // A heading being typed out is done once all of it shows.
        if let Some((_, born)) = self.reveal
            && born.elapsed() > REVEAL_PER_CHAR * 120
        {
            self.reveal = None;
        }
        if area.width != self.width || area.height != self.height {
            self.width = area.width;
            self.height = area.height;
            self.relayout();
            self.snap_back();
            self.keep_visible();
        }
        f.render_widget(
            ratatui::widgets::Block::default().style(theme::base()),
            area,
        );
        self.draw_header(f, Rect { height: 1, ..area });
        let body = Rect {
            y: area.y + 1,
            height: self.text_height() as u16,
            ..area
        };
        self.text_top = body.y;
        self.glide_scroll();
        self.draw_sheet(f, body);
        self.draw_map(f, body);
        self.mark_misspelt();
        self.mark_notes();
        self.draw_text(f, body);
        if self.strip_shown() {
            let units = shelf::units(&self.buf.text, &self.doc);
            let items = board::items(
                &self.buf.text,
                &self.doc,
                &units,
                &self.sidecar.summaries,
                self.wpm(),
            );
            let w = board::STRIP_W;
            board::draw_strip(
                f,
                Rect {
                    x: area.x + area.width - w,
                    width: w,
                    ..body
                },
                &items,
                self.buf.cursor,
            );
        }
        if self.notes_view.open {
            self.draw_notes(f, body);
        }
        let status = Rect {
            y: area.y + area.height.saturating_sub(2),
            height: 1,
            ..area
        };
        self.draw_status(f, status);
        let keys = Rect {
            y: area.y + area.height.saturating_sub(1),
            height: 1,
            ..area
        };
        self.draw_keys(f, keys);
        let at = self
            .cursor_stop()
            .filter(|c| c.row >= self.shown)
            .map(|c| {
                Position::new(
                    body.x + self.sheet().text_x + c.col,
                    body.y + (c.row - self.shown) as u16,
                )
            })
            .unwrap_or(Position::new(body.x, body.y));
        // What was under the window, for it to unroll over.
        let before = (self.animations && kind.is_some() && self.popup_born.elapsed() < POPUP_OPENS)
            .then(|| f.buffer_mut().clone());
        match &self.popup {
            Popup::None => {}
            Popup::Menu(m) => {
                let q = self.buf.text.get(m.from..self.buf.cursor).unwrap_or("");
                popup::draw_menu(f, body, at, m, q);
            }
            Popup::Verbs(v) => popup::draw_verbs(f, body, at, v),
            Popup::Search(m, q) => popup::draw_search(f, area, m, q),
            Popup::Form(form) => popup::draw_form(f, area, form),
            Popup::Cast(p) => {
                panels::draw_cast(f, area, &self.doc, &self.buf.text, &self.sidecar, p)
            }
            Popup::Memorize(m) => {
                let rev = self.buf.rev;
                if !matches!(&self.cues, Some((r, role, _)) if *r == rev && *role == m.role) {
                    let cues = dactile::memorize::cues(&self.buf.text, &self.doc, &m.role);
                    self.cues = Some((rev, m.role.clone(), cues));
                }
                let cues = self.cues.as_ref().map_or(&[][..], |c| &c.2);
                memorize::draw(f, area, cues, m, self.sidecar.actor(&m.role));
            }
            Popup::Roles => {
                let (rev, wpm) = (self.buf.rev, self.wpm());
                if !matches!(&self.roles_data, Some((r, w, _)) if *r == rev && *w == wpm) {
                    let data = roles::Data::of(&self.buf.text, &self.doc, wpm);
                    self.roles_data = Some((rev, wpm, data));
                }
                let Some((_, _, data)) = &self.roles_data else {
                    unreachable!()
                };
                roles::draw(
                    f,
                    area,
                    &self.doc,
                    &self.sidecar,
                    data,
                    self.wpm(),
                    &self.roles,
                );
                if self.message.is_some() {
                    self.draw_status(f, status);
                }
            }
            Popup::Scenes(p) => {
                let units = shelf::units(&self.buf.text, &self.doc);
                let items = board::items(
                    &self.buf.text,
                    &self.doc,
                    &units,
                    &self.sidecar.summaries,
                    self.wpm(),
                );
                board::draw(f, area, &self.doc, &items, &self.marks, self.trash.len(), p);
                if p.trash {
                    panels::draw_trash(f, area, &self.trash, p);
                }
                // Its messages, over the board.
                if self.message.is_some() {
                    self.draw_status(f, status);
                }
            }
            // Drawn below: it keeps where its caps are, for the mouse.
            Popup::Help(_) => {}
            Popup::Palette(q, sel) => {
                let items = palette_items(q);
                panels::draw_palette(f, area, q, &items, *sel);
            }
            Popup::Split(p) => panels::draw_split(
                f,
                area,
                &self.doc,
                &p.from,
                &p.spec.script,
                &p.scenes,
                &p.picked,
                p.sel,
            ),
            Popup::Suggest(range, items, sel) => {
                let word = self.buf.text.get(range.clone()).unwrap_or("");
                panels::draw_suggest(f, body, at, word, items, *sel);
            }
        }
        if let Popup::Help(h) = &mut self.popup {
            keys::draw_help(f, area, &self.keyboard, h);
        }
        if let Some(before) = before {
            anim::unroll(f.buffer_mut(), &before, self.popup_born, POPUP_OPENS);
        }
    }

    pub(super) fn draw_header(&self, f: &mut Frame, area: Rect) {
        let title = self.doc.title(&self.buf.text).unwrap_or_else(|| {
            self.path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        });
        let state = if self.read_only {
            Span::styled(
                lang::l("salt okunur ", "read-only "),
                Style::default().fg(WARN),
            )
        } else if self.conflict {
            Span::styled(
                lang::l("dışarıda değişti ", "changed elsewhere "),
                Style::default().fg(WARN),
            )
        } else if self.buf.dirty {
            Span::styled("• ", Style::default().fg(MUTED))
        } else {
            Span::styled(lang::l("kaydedildi ", "saved "), Style::default().fg(FAINT))
        };
        let left = Line::from(vec![
            Span::styled(" dactile", Style::default().fg(MUTED)),
            Span::styled(" — ", Style::default().fg(FAINT)),
            Span::styled(
                title,
                Style::default().fg(BEIGE).add_modifier(Modifier::BOLD),
            ),
        ]);
        f.render_widget(Paragraph::new(left).style(theme::base()), area);
        let w = state.content.chars().count() as u16;
        let right = Rect {
            x: area.x + area.width.saturating_sub(w),
            width: w.min(area.width),
            ..area
        };
        f.render_widget(
            Paragraph::new(Line::from(state)).style(theme::base()),
            right,
        );
    }

    /// The play's headings in the margin left of the page, where there is
    /// room: prefaces, acts and their scenes, the cursor's one marked.
    pub(super) fn draw_map(&mut self, f: &mut Frame, area: Rect) {
        self.map_hits.clear();
        let sheet = self.sheet();
        if sheet.x < 22 || self.strip_shown() {
            return;
        }
        let o = &self.doc.outline;
        let base = self.doc.body.start;
        // (where it starts, depth, label, hidden)
        let mut entries: Vec<(usize, u8, String, bool)> = Vec::new();
        for sct in &o.sections {
            let name = if sct.title.trim().is_empty() {
                match sct.which {
                    dactile::lex::Section::Preface => lang::l("Önsöz", "Preface"),
                    dactile::lex::Section::Notes => lang::l("Rejisöre notlar", "Director's notes"),
                }
                .to_string()
            } else {
                sct.title.trim().to_string()
            };
            entries.push((self.doc.blocks[sct.block + base].span.start, 0, name, false));
        }
        for u in shelf::units(&self.buf.text, &self.doc) {
            let depth = u8::from(u.kind == shelf::UnitKind::Scene);
            let name = if u.hidden {
                format!("{} ({})", u.name(), lang::l("gizli", "hidden"))
            } else {
                u.name()
            };
            entries.push((u.span.start, depth, name, u.hidden));
        }
        entries.sort_by_key(|e| e.0);
        if entries.is_empty() {
            return;
        }
        let here = self.buf.cursor;
        let current = entries.iter().rposition(|e| e.0 <= here);
        let w = (sheet.x - 4) as usize;
        let rows = area.height.saturating_sub(2) as usize;
        let start = current
            .map_or(0, |c| c.saturating_sub(rows / 2))
            .min(entries.len().saturating_sub(rows));
        let x = area.x + 2;
        for (n, (at, depth, label, hidden)) in entries.iter().enumerate().skip(start).take(rows) {
            let y = area.y + 1 + (n - start) as u16;
            let is_here = Some(n) == current;
            let indent = if *depth == 0 { "" } else { "  " };
            let marked = self.marks.contains(at);
            let mark = match (marked, is_here) {
                (true, _) => "■ ",
                (_, true) => "▸ ",
                _ => "  ",
            };
            let mut style = if is_here {
                Style::default().fg(CLARET)
            } else if *depth == 0 && !hidden {
                Style::default().fg(MUTED)
            } else {
                Style::default().fg(FAINT)
            };
            if *hidden {
                style = style.add_modifier(Modifier::ITALIC);
            }
            let text = popup::fit(&format!("{indent}{label}"), w.saturating_sub(2));
            let line = Line::from(vec![
                Span::styled(
                    mark,
                    Style::default().fg(if marked { WARN } else { CLARET }),
                ),
                Span::styled(text, style),
            ]);
            f.render_widget(
                Paragraph::new(line).style(theme::base()),
                Rect {
                    x,
                    y,
                    width: w as u16,
                    height: 1,
                },
            );
            self.map_hits.push((y, *at));
        }
    }

    /// The desk and the page on it.
    pub(super) fn draw_sheet(&self, f: &mut Frame, area: Rect) {
        let sheet = self.sheet();
        let p = self.palette();
        let buf = f.buffer_mut();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let on_page = x >= sheet.x && x < sheet.x + sheet.width;
                let shadow = self.on_paper() && x == sheet.x + sheet.width && y > area.y;
                let bg = if on_page {
                    p.bg
                } else if shadow {
                    theme::SHADOW
                } else {
                    theme::BG
                };
                buf[(x, y)].set_char(' ').set_style(Style::default().bg(bg));
            }
        }
    }

    pub(super) fn draw_text(&self, f: &mut Frame, area: Rect) {
        let sel = self.buf.selection();
        let sheet = self.sheet();
        let p = self.palette();
        let left = area.x + sheet.text_x;
        let right = area.x + (sheet.x + sheet.width).min(area.width);
        // Panels cover the text; forms and search type somewhere else.
        let covered = matches!(
            self.popup,
            Popup::Cast(_)
                | Popup::Scenes(_)
                | Popup::Roles
                | Popup::Memorize(_)
                | Popup::Split(_)
                | Popup::Palette(..)
                | Popup::Help(_)
                | Popup::Form(_)
                | Popup::Search(..)
        );
        let cursor = self.cursor_stop().filter(|c| {
            !covered && c.row >= self.shown && c.row < self.shown + area.height as usize
        });
        // A heading just written types itself out.
        let typing = self.reveal.and_then(|(at, born)| {
            let b = (self.doc.body.clone()).find(|&b| {
                self.doc.blocks[b].span.start >= at
                    && matches!(
                        self.doc.blocks[b].kind,
                        Kind::Act { .. } | Kind::Scene { .. } | Kind::Section { .. }
                    )
            })?;
            let n = (born.elapsed().as_millis() / REVEAL_PER_CHAR.as_millis()) as usize;
            Some((b, n))
        });
        let mut shown_chars = 0usize;
        // Notes whose margin dot is drawn: on the first row of their place.
        let mut dotted: Vec<usize> = Vec::new();
        let buf = f.buffer_mut();
        for (i, row) in self
            .lay
            .rows
            .iter()
            .skip(self.shown)
            .take(area.height as usize)
            .enumerate()
        {
            let y = area.y + i as u16;
            if row.gap {
                // Between two pages: the desk shows through.
                let bg = if self.on_paper() {
                    theme::BG
                } else {
                    theme::SHADOW
                };
                for x in
                    area.x + sheet.x..(area.x + sheet.x + sheet.width + 1).min(area.x + area.width)
                {
                    buf[(x, y)].set_char(' ').set_style(Style::default().bg(bg));
                }
                continue;
            }
            let here = row.block.is_some() && row.block == self.block();
            if let Some(n) = row.number {
                let label = format!("{n:>4}");
                let style = Style::default()
                    .bg(p.bg)
                    .fg(if here { p.muted } else { p.faint });
                let x0 = left.saturating_sub(MARGIN) + 1;
                for (k, ch) in label.chars().enumerate() {
                    let x = x0 + k as u16;
                    if x < right {
                        buf[(x, y)].set_char(ch).set_style(style);
                    }
                }
            }
            for c in &row.cells {
                let x = left + c.col;
                if x >= right {
                    continue;
                }
                if let Some((b, n)) = typing
                    && row.block == Some(b)
                {
                    if shown_chars >= n {
                        continue;
                    }
                    shown_chars += 1;
                }
                let mut style = theme::on(c.style, p);
                if (self.dim && !here) || row.block.is_some_and(|b| !self.role_lit(b)) {
                    style = style.fg(p.faint);
                }
                if let Some(b) = c.byte
                    && self.misspelt.iter().any(|r| r.contains(&b))
                {
                    style = style
                        .add_modifier(Modifier::UNDERLINED)
                        .underline_color(p.warn);
                } else if let Some(b) = c.byte
                    && let Some(pr) = self.problems.iter().find(|r| r.span.contains(&b))
                {
                    // A brace astray spoils its block: plainly marked.
                    let brace = matches!(
                        pr.what,
                        check::What::Unclosed | check::What::ExtraClose
                    );
                    style = style
                        .add_modifier(Modifier::UNDERLINED)
                        .underline_color(if brace { p.warn } else { p.faint });
                }
                // A note's place: underlined in its author's colour; the
                // note chosen in the panel lit.
                if let Some(b) = c.byte
                    && let Some(m) = self
                        .note_marks
                        .iter()
                        .filter(|m| m.range.contains(&b) && (m.quoted || m.chosen))
                        .max_by_key(|m| m.chosen)
                {
                    if m.chosen {
                        style = style.bg(p.selection);
                    }
                    if !self.misspelt.iter().any(|r| r.contains(&b)) {
                        style = style
                            .add_modifier(Modifier::UNDERLINED)
                            .underline_color(m.color);
                    }
                }
                if let (Some(s), Some(b)) = (&sel, c.byte)
                    && s.contains(&b)
                {
                    style = style.bg(p.selection);
                }
                buf[(x, y)].set_char(c.ch).set_style(style);
            }
            // A dot in the margin where a note starts.
            let bytes = row.cells.iter().filter_map(|c| c.byte);
            if let (Some(lo), Some(hi)) = (bytes.clone().min(), bytes.max())
                && let Some((k, m)) = self
                    .note_marks
                    .iter()
                    .enumerate()
                    .filter(|(k, m)| {
                        !dotted.contains(k) && m.range.start <= hi && lo <= m.range.end
                    })
                    .max_by_key(|(_, m)| m.chosen)
            {
                dotted.push(k);
                let x = left.saturating_sub(2);
                if x < right {
                    let ch = if m.chosen { '▸' } else { '•' };
                    buf[(x, y)]
                        .set_char(ch)
                        .set_style(Style::default().bg(p.bg).fg(m.color));
                }
            }
        }
        // Who speaks next, faint, on the empty line.
        if let (Some(a), Some(cur)) = (self.ghost(), cursor)
            && let Some(c) = self.doc.cast.get(&a)
        {
            let y = area.y + (cur.row - self.shown) as u16;
            let hint = format!("{}   Tab", c.script);
            let style = Style::default()
                .bg(p.bg)
                .fg(p.faint)
                .add_modifier(Modifier::ITALIC);
            for (k, ch) in hint.chars().enumerate() {
                let x = left + cur.col + k as u16;
                if x < right {
                    buf[(x, y)].set_char(ch).set_style(style);
                }
            }
        }
        // The cursor, unless a full panel covers the text. On paper the
        // terminal's own cursor can vanish, so the page draws its own.
        if let Some(cur) = cursor {
            let x = (left + cur.col).min(right.saturating_sub(1));
            let y = area.y + (cur.row - self.shown) as u16;
            if self.on_paper() {
                let cell = &mut buf[(x, y)];
                cell.set_style(Style::default().fg(p.bg).bg(p.fg));
            } else {
                f.set_cursor_position(Position::new(x, y));
            }
        }
    }

    pub(super) fn draw_status(&self, f: &mut Frame, area: Rect) {
        // A message stays a while, then fades into the status line.
        let age = self.message_at.elapsed();
        let gone = self.animations && age >= MESSAGE_STAYS + MESSAGE_FADES;
        if let Some((msg, warn)) = &self.message
            && !gone
        {
            let mut color = if *warn { WARN } else { MUTED };
            if self.animations && age > MESSAGE_STAYS {
                let t = (age - MESSAGE_STAYS).as_secs_f32() / MESSAGE_FADES.as_secs_f32();
                color = anim::mix(color, theme::BG, t);
            }
            let style = Style::default().fg(color);
            f.render_widget(
                Paragraph::new(format!(" {msg}")).style(theme::base().patch(style)),
                area,
            );
            return;
        }
        if let Some(pr) = self.problem_here() {
            f.render_widget(
                Paragraph::new(format!(" {}", pr.message()))
                    .style(theme::base().patch(Style::default().fg(MUTED))),
                area,
            );
            return;
        }
        let mut parts: Vec<Span> = Vec::new();
        let block = self.block();
        if let Some(a) = block.and_then(|b| self.doc.act_of(b)) {
            parts.push(Span::styled(
                format!(
                    " {}. {}",
                    outline::roman(self.doc.outline.acts[a].number),
                    lang::l("Perde", "Act")
                ),
                Style::default().fg(CLARET),
            ));
        }
        if let Some(s) = block.and_then(|b| self.doc.scene_of(b)) {
            let s = &self.doc.outline.scenes[s];
            parts.push(Span::styled(
                format!(" · {}", s.title()),
                Style::default().fg(CLARET),
            ));
            parts.push(Span::styled(
                format!(
                    " · {} {} · ≈ {}",
                    s.words,
                    lang::l("kelime", "words"),
                    timing::format(timing::scene_secs(s, self.wpm()))
                ),
                Style::default().fg(MUTED),
            ));
        }
        if let Some(who) = block
            .and_then(|b| self.doc.speaker(b))
            .and_then(|a| self.doc.cast.get(a))
        {
            parts.push(Span::styled(
                format!(" · {}", who.script),
                Style::default().fg(BEIGE),
            ));
        }
        if self.raw_now() {
            parts.push(Span::styled(
                lang::l("  HAM", "  RAW"),
                Style::default().fg(WARN).add_modifier(Modifier::BOLD),
            ));
        }
        if parts.is_empty() {
            parts.push(Span::styled(" ", Style::default()));
        }
        f.render_widget(Paragraph::new(Line::from(parts)).style(theme::base()), area);
        // The view's modes on the right, while they are on.
        let mut tags: Vec<&str> = Vec::new();
        if self.typewriter {
            tags.push(lang::l("daktilo", "typewriter"));
        }
        if self.focus {
            tags.push(lang::l("odak", "focus"));
        }
        if self.dim {
            tags.push(lang::l("soluk", "dim"));
        }
        let role_tag;
        if let Some(c) = self.role_view.as_deref().and_then(|a| self.doc.cast.get(a)) {
            role_tag = lang::f("rol: {}", "role: {}", &[&c.script]);
            tags.push(&role_tag);
        }
        let page;
        if self.pages && !self.lay.pages.is_empty() {
            let row = self.cursor_stop().map_or(0, |c| c.row);
            let n = self.lay.pages.partition_point(|&p| p <= row);
            page = lang::f("sayfa {}/{}", "page {}/{}", &[&n, &self.lay.pages.len()]);
            tags.push(&page);
        }
        let misspelt = self.misspelt.len();
        let spell = (misspelt > 0).then(|| lang::f("{} yazım", "{} spelling", &[&misspelt]));
        let mut right: Vec<Span> = Vec::new();
        if let Some(sp) = spell {
            right.push(Span::styled(sp, Style::default().fg(WARN)));
            right.push(Span::raw("  "));
        }
        for t in tags {
            right.push(Span::styled(
                format!(" {t} "),
                Style::default().fg(BEIGE).bg(theme::SELECTION),
            ));
            right.push(Span::raw(" "));
        }
        let w: usize = right.iter().map(|s| s.content.chars().count()).sum();
        if w > 0 && (w as u16) < area.width / 2 {
            let r = Rect {
                x: area.x + area.width - w as u16,
                width: w as u16,
                ..area
            };
            f.render_widget(Paragraph::new(Line::from(right)).style(theme::base()), r);
        }
    }

    pub(super) fn draw_keys(&self, f: &mut Frame, area: Rect) {
        // (key, what, how much it matters): the least needed go first when
        // the screen is narrow.
        let keys = [
            ("F1", lang::l("yardım", "help"), 9),
            ("^P", lang::l("komutlar", "commands"), 8),
            ("\\", lang::l("öneri", "suggest"), 7),
            ("^O", lang::l("kişiler", "cast"), 8),
            ("^G", lang::l("pano", "board"), 6),
            ("^N", lang::l("notlar", "notes"), 6),
            ("^F", lang::l("bul", "find"), 4),
            ("^K", lang::l("ara", "search"), 3),
            ("^Z", lang::l("geri al", "undo"), 2),
            ("F6", lang::l("tema", "theme"), 1),
            ("F7", lang::l("sayfalar", "pages"), 1),
            ("^S", lang::l("kaydet", "save"), 5),
            ("^Q", lang::l("çık", "quit"), 10),
        ];
        let len = |k: &str, w: &str| k.chars().count() + w.chars().count() + 4;
        let mut shown: Vec<_> = keys.to_vec();
        while shown.iter().map(|(k, w, _)| len(k, w)).sum::<usize>() + 1 > area.width as usize {
            let Some(least) = shown
                .iter()
                .enumerate()
                .min_by_key(|(_, k)| k.2)
                .map(|(i, _)| i)
            else {
                break;
            };
            shown.remove(least);
        }
        let mut spans = vec![Span::raw(" ")];
        for (k, what, _) in shown {
            spans.push(Span::styled(k, Style::default().fg(MUTED)));
            spans.push(Span::styled(
                format!(" {what}   "),
                Style::default().fg(FAINT),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(spans)).style(theme::base()), area);
    }
}
