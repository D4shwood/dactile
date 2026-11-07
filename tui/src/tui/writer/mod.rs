//! The writing screen: the play as it will read, edited in place.

mod cast;
mod casting;
mod draw;
mod find;
mod forms;
mod mouse;
mod notes;
mod palette;
mod ribbon;
mod saving;
mod scenes;
mod settings;
mod spelling;
mod submit;
#[cfg(test)]
mod tests;
mod typing;

use notes::{NOTES_W, NoteMark, NotesView};
use palette::palette_items;
pub use palette::{PAct, PItem};

use super::anim;
use super::board;
use super::keys;
use super::memorize::{self, Memo};
use super::panels::{self, CastPanel, Entry, ScenesPanel};
use super::popup::{self, Field, Form, FormKind, Menu, Verbs};
use super::roles::{self, RolesPanel};
use super::theme::{self, BEIGE, CLARET, FAINT, MUTED, WARN};
use anyhow::Result;
use dactile::buffer::{self, Buffer};
use dactile::build::{self, Built, Failed};
use dactile::cast::{self as dcast, Spec};
use dactile::check::{self, Problem};
use dactile::complete::{self, Action};
use dactile::file::{self, LockState};
use dactile::lex::Kind;
use dactile::outline;
use dactile::render::{self, Doc, Layout, Options, Stop};
use dactile::sidecar::{Casting, Sidecar};
use dactile::source;
use dactile::spell::{self, Speller};
use dactile::{info, lang, library, shelf, structure, timing};
use ratatui::Frame;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant, SystemTime};

/// How long a window takes to open, and how long a message stays before
/// it fades.
const POPUP_OPENS: Duration = Duration::from_millis(160);
const MESSAGE_STAYS: Duration = Duration::from_secs(4);
const MESSAGE_FADES: Duration = Duration::from_millis(900);
/// How fast a new heading types itself.
const REVEAL_PER_CHAR: Duration = Duration::from_millis(28);

/// Saved this long after the last keystroke.
const AUTOSAVE_AFTER: Duration = Duration::from_secs(2);
/// How often the file on disk is checked for changes from elsewhere.
const CHECK_EVERY: Duration = Duration::from_secs(2);
/// Columns left of the text on the page: the draft line numbers.
const MARGIN: u16 = 7;
/// Air right of the text on the page.
const PAGE_RIGHT: u16 = 3;
/// The widest the text gets: about a typed page.
const PAGE_TEXT: u16 = 66;
/// Two Ctrl+G within this: the board instead of the strip.
const TWICE: Duration = Duration::from_millis(400);

/// Where the page sits on the screen.
struct Sheet {
    x: u16,
    width: u16,
    /// The text's first column.
    text_x: u16,
    text_w: u16,
}

pub struct Writer {
    path: PathBuf,
    buf: Buffer,
    doc: Doc,
    lay: Layout,
    width: u16,
    height: u16,
    /// The block at the cursor is shown as raw source.
    raw_here: bool,
    raw_all: bool,
    preamble_open: bool,
    scroll: usize,
    /// The scroll as drawn: it glides to `scroll` when animations are on.
    shown: usize,
    glide: f32,
    animations: bool,
    /// A heading just written: it types itself out from then.
    reveal: Option<(usize, Instant)>,
    /// When the open popup opened, and which kind it is.
    popup_born: Instant,
    popup_kind: Option<std::mem::Discriminant<Popup>>,
    /// When the message on the status line came.
    message_at: Instant,
    /// The column Up/Down try to keep.
    goal_col: Option<u16>,
    _lock: Option<file::Lock>,
    read_only: bool,
    history: PathBuf,
    /// The file's modification time after our last read or write.
    disk_time: Option<SystemTime>,
    checked: Instant,
    /// The file changed on disk while we had unsaved changes: autosave waits.
    conflict: bool,
    /// A key that asks to be pressed again (^Q over a conflict, ^S to take
    /// the lock): the next key, if not the same, forgets it.
    armed: Option<Armed>,
    message: Option<(String, bool)>,
    clipboard: Option<arboard::Clipboard>,
    /// Fallback when there's no system clipboard.
    copied: String,
    popup: Popup,
    /// The cursor's line stays in the middle of the screen.
    typewriter: bool,
    /// Only the cursor's scene; the others fold to one line.
    focus: bool,
    /// Everything but the cursor's block in the background.
    dim: bool,
    /// paper, sepia, night, contrast or dark.
    theme: String,
    /// Page view: A4-shaped sheets broken where the PDF breaks.
    pages: bool,
    /// `NAME:` just became a line with its space: the next space is dropped.
    eat_space: bool,
    /// The keyboard the help screen draws.
    keyboard: String,
    /// The margin map's rows on screen and the block each one opens.
    map_hits: Vec<(u16, usize)>,
    /// The ribbon's tabs and buttons on screen, for the mouse; the tab
    /// shown, and whether only the tabs show.
    ribbon_hits: Vec<(Rect, ribbon::Hit)>,
    ribbon_tab: usize,
    ribbon_folded: bool,
    /// Acts and scenes marked to hide or throw away, by where they start.
    /// Any edit clears them.
    marks: std::collections::BTreeSet<usize>,
    /// The play's trash, as last read.
    trash: Vec<shelf::Trashed>,
    /// Suggestions only with ^K, not on every backslash.
    menu_manual: bool,
    default_wpm: u32,
    /// The top of the text area on screen, for the mouse.
    text_top: u16,
    sidecar: Sidecar,
    speller: Option<Speller>,
    speller_rx: Option<Receiver<Result<Speller>>>,
    /// A PDF build running in the background, and since when.
    building: Option<(Receiver<Result<Built, Failed>>, Instant)>,
    /// A table or actor copies being built: what came out (a file or a
    /// folder) and how many PDFs.
    aside: Option<(Receiver<Aside>, Instant)>,
    /// The last of those, for `o` on F9's screen.
    aside_out: Option<PathBuf>,
    /// What would stop XeLaTeX, underlined faintly.
    problems: Vec<Problem>,
    /// Where F5 last stopped for a problem: F5 there again builds anyway.
    stopped_at: Option<usize>,
    /// The line of the play the last build stopped on, for Alt+G.
    failed_line: Option<usize>,
    /// Words the play knows: names and words added by hand, lower case.
    known: HashSet<String>,
    spell_cache: HashMap<String, bool>,
    /// Misspelt words on screen, by byte range.
    misspelt: Vec<std::ops::Range<usize>>,
    /// F9's figures and a role's cues, worked out once per version of the
    /// text (`buf.rev`) instead of on every frame.
    roles_data: Option<(u64, u32, roles::Data)>,
    /// The blocks laid out last time, for the next layout to take over.
    lay_cache: render::Cache,
    cues: Option<(u64, String, Vec<dactile::memorize::Cue>)>,
    /// The strip beside the writing (^G), when there is room for it.
    strip: bool,
    /// The last ^G, to tell a double one.
    strip_key: Option<Instant>,
    /// The board shows cards rather than the list.
    board_cards: bool,
    /// F9's screen, where it was left.
    roles: RolesPanel,
    /// Shift+F8: this role's lines lit, the rest faint (an alias).
    role_view: Option<String>,
    /// The play as it was before this session's first move, split or
    /// merge, kept as a draft.
    kept: Option<PathBuf>,
    /// The notes on the play (`oyun.notlar.md`) and their panel (^N).
    notes: dactile::notes::Notes,
    notes_view: NotesView,
    notes_scroll: usize,
    note_marks: Vec<NoteMark>,
    /// Who one's own notes are from: the name in the settings.
    me: String,
    pub quit: bool,
}

/// The scenes a character speaks in, to give some to a new character.
pub struct SplitPanel {
    pub from: String,
    pub spec: Spec,
    /// Indexes into the outline's scenes.
    pub scenes: Vec<usize>,
    pub picked: std::collections::BTreeSet<usize>,
    pub sel: usize,
}

/// The view toggles, kept in the settings between sessions.
pub struct View {
    pub typewriter: bool,
    pub focus: bool,
    pub dim: bool,
    pub theme: String,
    pub pages: bool,
    pub animations: bool,
    pub ribbon_folded: bool,
}

/// What a build beside the play made: where, and how many PDFs.
type Aside = Result<(PathBuf, usize), Failed>;

/// What a second press of the same key does.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Armed {
    /// ^Q while the file changed elsewhere: quit, writing ours over it.
    Quit,
    /// ^S while another Dactile holds the play: take the lock.
    TakeLock,
    /// x on a note: delete it.
    DeleteNote,
}

enum Popup {
    None,
    /// After a backslash: what can follow it.
    Menu(Menu),
    /// After `\xdo `: how directions usually start.
    Verbs(Verbs),
    /// ^K: the same list, searched by name.
    Search(Menu, String),
    Form(Form),
    /// ^O: the cast.
    Cast(CastPanel),
    /// ^G: the acts and scenes, and the trash.
    Scenes(ScenesPanel),
    /// Splitting a character: which of its scenes the new one takes.
    Split(SplitPanel),
    /// ^P: everything there is to do, by name; the query and the choice.
    Palette(String, usize),
    /// F1: the keyboard, with the key last pressed or pointed at.
    Help(keys::Help),
    /// Alt+S: spelling suggestions for the word at `range`.
    Suggest(std::ops::Range<usize>, Vec<String>, usize),
    /// F9: the casting, the scene/role table, the numbers (`self.roles`).
    Roles,
    /// Learning a role by heart.
    Memorize(Memo),
    /// F10: the keys on the ribbon.
    Ribbon(ribbon::Nav),
    /// Dosya: its page, the entry chosen.
    Backstage(usize),
    /// The play's own settings: its pace (0) and its words after.
    PlaySettings(usize),
}

impl Writer {
    pub fn open(path: &Path, cfg: &dactile::config::Config) -> Result<Writer> {
        let text = file::read(path)?;
        lang::set(lang::resolve(&cfg.lang, Some(&text)));
        let (lock, read_only, message) = match file::lock(path)? {
            LockState::Ours(l) => (Some(l), false, None),
            LockState::Taken(who) => (
                None,
                true,
                Some((
                    lang::f(
                        "Başka bir Dactile bu oyunu açmış ({}); salt okunur açıldı.",
                        "Another Dactile has this play open ({}); opened read-only.",
                        &[&who],
                    ),
                    true,
                )),
            ),
        };
        // Notes that don't read are put aside, not saved over.
        let (sidecar, message) = match Sidecar::load(path) {
            Ok(s) => (s, message),
            Err(e) => {
                let kept = dactile::sidecar::put_aside(path);
                let detail = match kept {
                    Ok(p) => dactile::config::tilde(&p),
                    Err(e) => format!("{e:#}"),
                };
                (
                    Sidecar::default(),
                    Some((
                        lang::f(
                            "Notlar okunamadı ({}); kenara kondu: {}",
                            "The notes didn't read ({}); put aside: {}",
                            &[&format!("{:#}", e.root_cause()), &detail],
                        ),
                        true,
                    )),
                )
            }
        };
        let history = file::history_dir(path);
        // The play as it was before this session, whatever happens next.
        let _ = file::snapshot(&history, &text, true);
        let doc = Doc::new(&text);
        let mut buf = Buffer::new(text);
        // Start at the first line of the body that has something on it.
        buf.cursor = doc.blocks[doc.body.clone()]
            .iter()
            .find(|b| !matches!(b.kind, Kind::Blank))
            .map_or(doc.parts.body.start, |b| b.span.start);
        let mut w = Writer {
            path: path.to_path_buf(),
            buf,
            doc,
            lay: Layout::default(),
            width: 80,
            height: 24,
            raw_here: false,
            raw_all: false,
            preamble_open: false,
            scroll: 0,
            shown: 0,
            glide: 0.0,
            animations: cfg.animations,
            reveal: None,
            popup_born: Instant::now(),
            popup_kind: None,
            message_at: Instant::now(),
            goal_col: None,
            _lock: lock,
            read_only,
            history,
            disk_time: file::modified(path),
            checked: Instant::now(),
            conflict: false,
            armed: None,
            message,
            clipboard: arboard::Clipboard::new().ok(),
            copied: String::new(),
            popup: Popup::None,
            typewriter: cfg.typewriter,
            focus: cfg.focus,
            dim: cfg.dim,
            theme: cfg.theme().to_string(),
            pages: cfg.pages,
            eat_space: false,
            keyboard: cfg.keyboard.clone(),
            map_hits: Vec::new(),
            ribbon_hits: Vec::new(),
            ribbon_tab: 1,
            ribbon_folded: cfg.ribbon_folded,
            marks: Default::default(),
            trash: Vec::new(),
            menu_manual: cfg.menu == "manual",
            default_wpm: cfg.wpm,
            text_top: 1,
            sidecar,
            speller: None,
            speller_rx: None,
            building: None,
            aside: None,
            aside_out: None,
            problems: Vec::new(),
            stopped_at: None,
            failed_line: None,
            known: HashSet::new(),
            spell_cache: HashMap::new(),
            misspelt: Vec::new(),
            roles_data: None,
            lay_cache: render::Cache::default(),
            cues: None,
            strip: false,
            strip_key: None,
            board_cards: false,
            roles: RolesPanel::default(),
            role_view: None,
            kept: None,
            notes: Default::default(),
            notes_view: NotesView::default(),
            notes_scroll: 0,
            note_marks: Vec::new(),
            me: cfg.author.clone(),
            quit: false,
        };
        w.load_notes();
        // The dictionary takes a moment to load: in the background.
        if let Some((aff, dic)) = spell::find() {
            let (tx, rx) = channel();
            std::thread::spawn(move || {
                let _ = tx.send(Speller::load(&aff, &dic));
            });
            w.speller_rx = Some(rx);
        }
        w.refresh_known();
        w.relayout();
        w.snap_forward();
        Ok(w)
    }

    // ------------------------------------------------------------ model

    fn block(&self) -> Option<usize> {
        self.doc.block_at(self.buf.cursor)
    }

    fn opts(&self) -> Options<'_> {
        Options {
            width: self.text_width(),
            raw_block: if self.raw_here { self.block() } else { None },
            raw_all: self.raw_all,
            preamble_open: self.preamble_open,
            page_rows: self.pages.then(|| self.page_rows()),
            focus_scene: if self.focus {
                self.block().and_then(|b| self.doc.scene_of(b))
            } else {
                None
            },
            summaries: Some(&self.sidecar.summaries),
        }
    }

    fn text_width(&self) -> u16 {
        self.sheet().text_w
    }

    /// A page of `PAGE_TEXT` columns in the middle, or the whole screen when
    /// that doesn't fit.
    fn sheet(&self) -> Sheet {
        let full = MARGIN + PAGE_TEXT + PAGE_RIGHT;
        let room = self.width
            - if self.strip_shown() {
                board::STRIP_W
            } else if self.notes_beside() {
                NOTES_W
            } else {
                0
            };
        if room >= full + 4 {
            let x = (room - full) / 2;
            Sheet {
                x,
                width: full,
                text_x: x + MARGIN,
                text_w: PAGE_TEXT,
            }
        } else {
            Sheet {
                x: 0,
                width: self.width,
                text_x: MARGIN,
                text_w: self.width.saturating_sub(MARGIN + 1).max(20),
            }
        }
    }

    /// Whether the strip is on and the paper and it fit side by side.
    fn strip_shown(&self) -> bool {
        self.strip
            && !self.notes_view.open
            && self.width >= MARGIN + PAGE_TEXT + PAGE_RIGHT + 4 + board::STRIP_W
    }

    /// Rows of text on a sheet in page view: an A4 sheet as wide as the
    /// page on screen (terminal cells are about twice as tall as wide), less
    /// its margins.
    fn page_rows(&self) -> u16 {
        let tall = (self.sheet().width as f32 * 1.414 / 2.0).round() as u16;
        tall.saturating_sub((render::PAGE_TOP + render::PAGE_BOTTOM) as u16)
            .max(12)
    }

    fn palette(&self) -> &'static theme::Palette {
        theme::palette(&self.theme)
    }

    /// A page of its own colour on the dark around it; the dark theme is
    /// all one.
    fn on_paper(&self) -> bool {
        self.theme != "dark"
    }

    /// Rebuilds the model and the layout from the text.
    fn relayout(&mut self) {
        self.doc = Doc::new(&self.buf.text);
        let mut cache = std::mem::take(&mut self.lay_cache);
        self.lay = render::layout_cached(&self.buf.text, &self.doc, self.opts(), &mut cache);
        self.lay_cache = cache;
        let known = check::known(&self.buf.text, &self.doc);
        self.problems = check::problems(&self.buf.text, &self.doc, &known);
    }

    /// The problem the cursor is on, if any.
    fn problem_here(&self) -> Option<&Problem> {
        let at = self.buf.cursor;
        self.problems
            .iter()
            .find(|p| p.span.start <= at && at <= p.span.end)
    }

    /// The quiet fixes, but not in the block being written as source.
    fn fix_quietly(&mut self) {
        let keep = (self.raw_here && !self.raw_all)
            .then(|| self.block().map(|b| self.doc.blocks[b].span.clone()))
            .flatten();
        let edits = check::fixes(&self.problems, keep);
        if edits.is_empty() {
            return;
        }
        let at = self.buf.cursor;
        let shift: usize = edits
            .iter()
            .filter(|e| e.range.start < at)
            .map(|e| e.text.len() - e.range.len())
            .sum();
        self.buf.apply(&edits, at + shift);
        self.relayout();
    }

    /// Lays out again after an edit; the cursor lands on a stop.
    fn edited(&mut self) {
        self.marks.clear();
        self.relayout();
        self.snap_back();
    }

    fn cursor_stop(&self) -> Option<Stop> {
        self.lay.stop_at(self.buf.cursor)
    }

    /// Moves the cursor onto the stop at or before it.
    fn snap_back(&mut self) {
        if !self.lay.is_stop(self.buf.cursor)
            && let Some(s) = self.lay.stop_at(self.buf.cursor)
        {
            self.buf.cursor = s.byte;
        }
    }

    fn snap_forward(&mut self) {
        if !self.lay.is_stop(self.buf.cursor)
            && let Some(s) = self
                .lay
                .next_stop(self.buf.cursor)
                .or_else(|| self.lay.stop_at(self.buf.cursor))
        {
            self.buf.cursor = s.byte;
        }
    }

    /// Moves to `byte`; leaving the block closes its raw view.
    fn go(&mut self, byte: usize, select: bool) {
        if select {
            self.buf.anchor.get_or_insert(self.buf.cursor);
        } else {
            self.buf.anchor = None;
        }
        let before = self.block();
        let scene = before.and_then(|b| self.doc.scene_of(b));
        self.buf.cursor = byte;
        let moved = self.block() != before;
        if self.raw_here && moved {
            self.raw_here = false;
            self.relayout();
            self.snap_back();
        } else if self.focus && moved && self.block().and_then(|b| self.doc.scene_of(b)) != scene {
            // Into a folded scene: it opens, the one left folds.
            self.relayout();
            self.snap_forward();
        }
    }

    fn in_folded_preamble(&self) -> bool {
        !self.preamble_open && !self.raw_all && self.buf.cursor < self.doc.parts.body.start
    }

    fn editable(&mut self) -> bool {
        if self.read_only {
            self.say(
                lang::l(
                    "Salt okunur: oyun başka bir Dactile'da açık (^S iki kez: kilidi al).",
                    "Read-only: the play is open in another Dactile (^S twice takes the lock).",
                ),
                true,
            );
            return false;
        }
        if self.in_folded_preamble() {
            self.say(
                lang::l(
                    "Kişiler ve oyun bilgisi kapalı: Alt+P ile aç.",
                    "The cast and cover lines are folded: Alt+P opens them.",
                ),
                false,
            );
            return false;
        }
        true
    }

    /// A warning on the status line, from outside the writer.
    pub fn warn(&mut self, s: &str) {
        self.say(s, true);
    }

    fn say(&mut self, s: &str, warn: bool) {
        self.message = Some((s.to_string(), warn));
        self.message_at = Instant::now();
    }

    /// Whether something on screen is still moving: the loop draws faster.
    pub fn animating(&self) -> bool {
        self.animations
            && (self.shown != self.scroll
                || self.reveal.is_some()
                || self.popup_born.elapsed() < POPUP_OPENS
                || (self.message.is_some()
                    && (MESSAGE_STAYS..MESSAGE_STAYS + MESSAGE_FADES)
                        .contains(&self.message_at.elapsed())))
    }

    /// Moves the drawn scroll a step toward the real one.
    fn glide_scroll(&mut self) {
        if !self.animations {
            self.shown = self.scroll;
            self.glide = self.scroll as f32;
            return;
        }
        let target = self.scroll as f32;
        // Far jumps cut most of the way at once, then ease in.
        if (target - self.glide).abs() > self.text_height() as f32 * 3.0 {
            self.glide = target - (target - self.glide).signum() * self.text_height() as f32;
        }
        self.glide += (target - self.glide) * 0.3;
        if (target - self.glide).abs() < 0.5 {
            self.glide = target;
        }
        self.shown = self.glide.round() as usize;
    }

    fn raw_now(&self) -> bool {
        self.raw_here || self.raw_all
    }
}
