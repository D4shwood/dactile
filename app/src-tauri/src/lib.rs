//! Dactile's desktop app: a Svelte front end over `dactile_backend`. The
//! window's commands go through `call` to the backend's table; here are only
//! those a desktop has and a web page doesn't: its file dialogs, its
//! programs, its printer, its screen.

use dactile_backend::{Ctx, Res, err, importing, play};
use std::path::PathBuf;
use tauri::{Manager, State};

type Shared<'a> = State<'a, Ctx>;

/// A command of the backend's, by its name: off the main thread, as some of
/// them run XeLaTeX or read the plays.
#[tauri::command]
async fn call(
    handle: tauri::AppHandle,
    name: String,
    args: serde_json::Value,
) -> Res<serde_json::Value> {
    tauri::async_runtime::spawn_blocking(move || {
        dactile_backend::dispatch(handle.state::<Ctx>().inner(), &name, args)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reads a document (a PDF takes a while) and keeps the draft for the
/// preview; the OCR's progress goes to the window as it comes.
#[tauri::command]
async fn import_read(
    app: Shared<'_>,
    handle: tauri::AppHandle,
    path: String,
    ocr: bool,
) -> Res<importing::Shown> {
    use tauri::Emitter;
    let lang = play::lang_of(&app.lock().unwrap().cfg);
    let file = PathBuf::from(path);
    let from = file.clone();
    let draft = tauri::async_runtime::spawn_blocking(move || {
        importing::read(&from, lang, ocr, &|done, total| {
            let _ = handle.emit("ocr", (done, total));
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    let shown = importing::shown(&file, &draft);
    app.lock().unwrap().importing = Some((file, draft));
    Ok(shown)
}

/// The play named on the command line (`dactile oyun.tex`), once.
#[tauri::command]
fn startup(app: Shared) -> Option<String> {
    let mut a = app.lock().unwrap();
    if a.started {
        return None;
    }
    a.started = true;
    // An AppImage runs from inside itself, and says where it was started.
    let here = std::env::var_os("OWD")
        .map(PathBuf::from)
        .unwrap_or_default();
    std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .find(|p| p.extension().is_some_and(|e| e == "tex"))
        .and_then(|p| here.join(p).canonicalize().ok())
        .map(|p| p.display().to_string())
}

/// A file's text, for notes from a file.
#[tauri::command]
async fn pick_text() -> Option<String> {
    let f = rfd::AsyncFileDialog::new()
        .add_filter("Metin", &["md", "txt"])
        .pick_file()
        .await?;
    std::fs::read_to_string(f.path()).ok()
}

#[tauri::command]
async fn pick_import() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .add_filter(
            dactile::lang::l("Oyun belgeleri", "Play documents"),
            &dactile::import::EXTENSIONS,
        )
        .pick_file()
        .await
        .map(|f| f.path().display().to_string())
}

/// A file or a folder, in the desktop's own program for it.
#[tauri::command]
fn launch(path: String) -> Res<()> {
    dactile::build::launch(std::path::Path::new(&path)).map_err(err)
}

/// A file selected in the desktop's file manager. It may wait for an answer
/// over D-Bus: off the main thread.
#[tauri::command]
async fn reveal(path: String) -> Res<()> {
    tauri::async_runtime::spawn_blocking(move || {
        dactile::build::reveal(std::path::Path::new(&path)).map_err(err)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn log(message: String) {
    eprintln!("[window] {message}");
}

/// A play picked in the system's file dialog.
#[tauri::command]
async fn pick() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .add_filter("Oyun", &["tex"])
        .pick_file()
        .await
        .map(|f| f.path().display().to_string())
}

/// The PDF's bytes, for the preview.
#[tauri::command]
fn pdf(path: String) -> Res<tauri::ipc::Response> {
    std::fs::read(path)
        .map(tauri::ipc::Response::new)
        .map_err(|e| e.to_string())
}

/// The window over the whole screen, for the PDF viewer.
#[tauri::command]
fn full_screen(window: tauri::WebviewWindow, on: bool) -> Res<()> {
    window.set_fullscreen(on).map_err(|e| e.to_string())
}

/// Pages of a PDF to the default printer. `lp` may take a moment: off the
/// main thread.
#[tauri::command]
async fn pdf_print(path: String, first: usize, last: usize, copies: usize) -> Res<()> {
    tauri::async_runtime::spawn_blocking(move || {
        dactile::build::print(std::path::Path::new(&path), first, last, copies).map_err(err)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// A web address from a PDF's link, in the desktop's browser.
#[tauri::command]
fn open_url(url: String) -> Res<()> {
    dactile::build::open_url(&url).map_err(err)
}

/// In a debug build with `DACTILE_DEV_EVAL=file`: JavaScript written to that
/// file runs in the window, and the file goes. For trying the app out from
/// a script, with no hands on it.
fn dev_eval(app: tauri::AppHandle) {
    let Some(path) = std::env::var_os("DACTILE_DEV_EVAL").filter(|_| cfg!(debug_assertions)) else {
        return;
    };
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let Ok(js) = std::fs::read_to_string(&path) else {
                continue;
            };
            let _ = std::fs::remove_file(&path);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.eval(js);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Wayland desktops match a window to its menu entry by this name alone:
    // left at the binary's name, the window gets the generic W icon.
    #[cfg(target_os = "linux")]
    glib::set_prgname(Some("io.github.d4shwood.dactile"));
    tauri::Builder::default()
        .manage(Ctx::default())
        .setup(|app| {
            dactile_backend::spell::load(app.state::<Ctx>().spell.clone());
            dev_eval(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            call,
            import_read,
            startup,
            pick_text,
            pick_import,
            launch,
            reveal,
            log,
            pick,
            pdf,
            full_screen,
            pdf_print,
            open_url,
        ])
        .run(tauri::generate_context!())
        .expect("the window could not open");
}
