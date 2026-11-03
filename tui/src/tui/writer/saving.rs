//! Saving, building, and what goes on in the background.

use super::*;

impl Writer {
    pub fn tick(&mut self) {
        if let Some(rx) = &self.speller_rx
            && let Ok(result) = rx.try_recv()
        {
            self.speller_rx = None;
            match result {
                Ok(sp) => {
                    self.speller = Some(sp);
                    self.spell_cache.clear();
                }
                Err(e) => self.say(
                    &lang::f(
                        "Sözlük yüklenemedi: {}",
                        "Couldn't load the dictionary: {}",
                        &[&format!("{e:#}")],
                    ),
                    true,
                ),
            }
        }
        if let Some((rx, since)) = &self.aside {
            let since = *since;
            match rx.try_recv() {
                Ok(result) => {
                    self.aside = None;
                    self.aside_done(result);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.aside = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    let secs = since.elapsed().as_secs();
                    self.message = Some((
                        lang::f("Derleniyor… {} sn", "Building… {} s", &[&secs]),
                        false,
                    ));
                }
            }
        }
        if let Some((rx, since)) = &self.building {
            let since = *since;
            match rx.try_recv() {
                Ok(result) => {
                    self.building = None;
                    self.built(result);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.building = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // The message stays while it runs.
                    let secs = since.elapsed().as_secs();
                    self.message = Some((
                        lang::f("PDF derleniyor… {} sn", "Building the PDF… {} s", &[&secs]),
                        false,
                    ));
                    self.message_at = Instant::now();
                }
            }
        }
        self.tick_notes();
        if self.checked.elapsed() >= CHECK_EVERY {
            self.checked = Instant::now();
            self.check_disk();
            self.check_notes_disk();
        }
        let due = self.buf.dirty
            && self
                .buf
                .changed_at
                .is_some_and(|t| t.elapsed() >= AUTOSAVE_AFTER);
        if due && !self.conflict && !self.read_only {
            self.save(false);
        }
    }

    /// Picks up changes made to the file elsewhere (another editor, a sync).
    pub(super) fn check_disk(&mut self) {
        let now = file::modified(&self.path);
        if now.is_none() || now == self.disk_time {
            return;
        }
        if self.buf.dirty {
            self.conflict = true;
            self.say(
                lang::l(
                    "Dosya başka yerde değişti. ^S senin halini yazar; ^Q'dan önce karar ver.",
                    "The file changed elsewhere. ^S writes yours over it; decide before ^Q.",
                ),
                true,
            );
            return;
        }
        if let Ok(text) = file::read(&self.path)
            && text != self.buf.text
        {
            self.buf.reload(text);
            self.raw_here = false;
            self.relayout();
            self.snap_back();
            self.say(
                lang::l(
                    "Dosya başka yerde değişti; yeniden yüklendi.",
                    "The file changed elsewhere; reloaded.",
                ),
                false,
            );
        }
        self.disk_time = now;
    }

    pub(super) fn save(&mut self, by_hand: bool) {
        if self.read_only {
            return;
        }
        if !self.buf.dirty && !by_hand {
            return;
        }
        // Changed elsewhere since the last look: looked at now, not written
        // over unseen (autosave runs between two looks too).
        let now = file::modified(&self.path);
        if !self.conflict && now.is_some() && now != self.disk_time {
            self.check_disk();
            return;
        }
        self.fix_quietly();
        let theirs = self.keep_theirs();
        match file::write_atomic(&self.path, &self.buf.text) {
            Ok(()) => {
                self.buf.dirty = false;
                self.conflict = false;
                self.disk_time = file::modified(&self.path);
                let _ = file::snapshot(&self.history, &self.buf.text, false);
                self.notes_follow();
                // An .aux there already gets the casts: a run by hand is
                // right in one pass too. Not while XeLaTeX writes it.
                if self.building.is_none() {
                    let _ = build::seed(&self.path, &build::casts(&self.doc.outline), false);
                }
                if theirs {
                    self.say(
                        lang::l(
                            "Kaydedildi; dışarıdaki hali geçmişte kaldı.",
                            "Saved; the version from elsewhere is in the history.",
                        ),
                        false,
                    );
                } else if by_hand {
                    self.say(lang::l("Kaydedildi.", "Saved."), false);
                }
            }
            Err(e) => self.say(
                &lang::f(
                    "Kaydedilemedi: {}",
                    "Couldn't save: {}",
                    &[&format!("{e:#}")],
                ),
                true,
            ),
        }
    }

    /// Before ours goes over a file changed elsewhere: theirs, kept in the
    /// history. Whether there was one.
    fn keep_theirs(&self) -> bool {
        self.conflict
            && file::read(&self.path).is_ok_and(|t| file::snapshot(&self.history, &t, true).is_ok())
    }

    /// ^S in a play another Dactile holds: once says what it means, twice
    /// takes the lock, with the play as it is on disk.
    pub(super) fn take_lock(&mut self, confirmed: bool) {
        if !confirmed {
            let who = file::holder(&self.path).unwrap_or_default();
            self.armed = Some(Armed::TakeLock);
            self.say(
                &lang::f(
                    "Oyun başka bir Dactile'da açık ({}). ^S yine kilidi alır; ikisi birden yazarsa biri kaybolur.",
                    "The play is open in another Dactile ({}). ^S again takes the lock; if both write, one is lost.",
                    &[&who],
                ),
                true,
            );
            return;
        }
        match file::take_over(&self.path) {
            Ok(lock) => {
                self._lock = Some(lock);
                self.read_only = false;
                if let Ok(text) = file::read(&self.path)
                    && text != self.buf.text
                {
                    self.buf.reload(text);
                    self.relayout();
                    self.snap_back();
                }
                self.disk_time = file::modified(&self.path);
                self.say(
                    lang::l("Kilit artık sende.", "The lock is yours now."),
                    false,
                );
            }
            Err(e) => self.say(
                &lang::f(
                    "Kilit alınamadı: {}",
                    "Couldn't take the lock: {}",
                    &[&format!("{e:#}")],
                ),
                true,
            ),
        }
    }

    /// F5: the PDF, built in the background from the saved play.
    pub(super) fn build(&mut self) {
        if self.building.is_some() {
            return;
        }
        if !self.read_only {
            self.fix_quietly();
        }
        self.save(false);
        if let Some(p) = check::stops(&self.problems) {
            let at = p.span.start;
            if self.stopped_at != Some(at) {
                // Not built: the cursor goes there; F5 again builds anyway.
                let msg = p.message();
                self.stopped_at = Some(at);
                self.buf.cursor = at;
                self.buf.anchor = None;
                self.raw_here = true;
                self.relayout();
                self.snap_back();
                self.keep_visible();
                self.say(
                    &lang::f("{} (F5 yine de derler)", "{} (F5 builds anyway)", &[&msg]),
                    true,
                );
                return;
            }
        }
        self.stopped_at = None;
        if self.buf.dirty && !self.read_only {
            return;
        }
        let (tx, rx) = channel();
        let path = self.path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(build::build(&path));
        });
        self.building = Some((rx, Instant::now()));
        self.say(lang::l("PDF derleniyor…", "Building the PDF…"), false);
    }

    /// Builds something made from the play in the background.
    pub(super) fn start_aside(&mut self, job: impl FnOnce() -> Aside + Send + 'static) {
        if self.aside.is_some() {
            return;
        }
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(job());
        });
        self.aside = Some((rx, Instant::now()));
        self.say(lang::l("Derleniyor…", "Building…"), false);
    }

    pub(super) fn aside_done(&mut self, result: Aside) {
        let notes = std::mem::take(&mut self.notes_view.building);
        match result {
            Ok((out, _)) if notes => self.say(
                &lang::f(
                    "Notlu kopya hazır: {} (notlarda o açar)",
                    "The copy with notes is ready: {} (o in the notes opens it)",
                    &[&dactile::config::tilde(&out)],
                ),
                false,
            ),
            Ok((out, n)) => {
                let msg = if n == 1 {
                    lang::f(
                        "Hazır: {} (F9'da o açar)",
                        "Ready: {} (o on F9 opens it)",
                        &[&dactile::config::tilde(&out)],
                    )
                } else {
                    lang::f(
                        "{} PDF hazır: {} (F9'da o açar)",
                        "{} PDFs ready: {} (o on F9 opens it)",
                        &[&n, &dactile::config::tilde(&out)],
                    )
                };
                self.aside_out = Some(out);
                self.say(&msg, false);
            }
            Err(e) => self.say(&lang::f("Olmadı: {}", "Failed: {}", &[&e]), true),
        }
    }

    /// A copy for each actor of the casting in use, and for each role nobody
    /// is cast in, all at once.
    pub(super) fn actor_copies(&mut self, kind: dactile::casting::CopyKind) {
        use dactile::casting;
        let text = &self.buf.text;
        let stage = dactile::stage::read(text, &self.doc, &[]);
        let table = casting::table(text, &self.doc, &stage);
        let mut all = casting::copies(&self.doc, &self.sidecar, &table);
        for c in &mut all {
            c.kind = kind;
        }
        if all.is_empty() {
            self.say(lang::l("Sahnede kimse yok.", "Nobody is on stage."), true);
            return;
        }
        let aux = build::casts(&self.doc.outline);
        let dir = casting::copies_dir(&self.path);
        let jobs: Vec<(String, PathBuf)> = all
            .iter()
            .map(|c| {
                (
                    casting::copy_tex(text, &self.doc, &stage, &table, self.wpm(), c),
                    dir.join(c.file_name(&self.doc, &self.sidecar)),
                )
            })
            .collect();
        let play = self.path.clone();
        self.start_aside(move || {
            for (tex, out) in &jobs {
                build::build_aside(&play, tex, &aux, out)?;
            }
            Ok((dir, jobs.len()))
        });
    }

    pub(super) fn table_pdf(&mut self) {
        let data = roles::Data::of(&self.buf.text, &self.doc, self.wpm());
        let tex =
            dactile::casting::table_tex(&self.doc, &self.buf.text, &self.sidecar, &data.table);
        let out = dactile::casting::table_path(&self.path, "pdf");
        let play = self.path.clone();
        self.start_aside(move || build::build_aside(&play, &tex, "", &out).map(|b| (b.pdf, 1)));
    }

    pub(super) fn built(&mut self, result: Result<Built, Failed>) {
        match result {
            Ok(b) => {
                let pages = b.pages.map_or(String::new(), |n| {
                    lang::f(", {} sayfa", ", {} pages", &[&n])
                });
                self.say(
                    &lang::f(
                        "PDF hazır{} ({} sn). Alt+O ile aç.",
                        "PDF ready{} ({} s). Alt+O opens it.",
                        &[&pages, &format!("{:.1}", b.took.as_secs_f32())],
                    ),
                    false,
                );
            }
            Err(e) => self.say(
                &lang::f(
                    "PDF olmadı: {} (Alt+L log)",
                    "No PDF: {} (Alt+L: the log)",
                    &[&e],
                ),
                true,
            ),
        }
    }

    /// The view toggles, for the settings.
    pub fn view(&self) -> View {
        View {
            typewriter: self.typewriter,
            focus: self.focus,
            dim: self.dim,
            theme: self.theme.clone(),
            animations: self.animations,
            pages: self.pages,
            ribbon_folded: self.ribbon_folded,
        }
    }

    /// On quit: save what's left and keep a version.
    pub fn close(&mut self) -> Result<()> {
        if self.buf.dirty && !self.read_only {
            if file::modified(&self.path) != self.disk_time {
                self.conflict = true;
            }
            self.keep_theirs();
            file::write_atomic(&self.path, &self.buf.text)?;
            self.buf.dirty = false;
        }
        let _ = file::snapshot(&self.history, &self.buf.text, true);
        Ok(())
    }
}
