//! Dactile's desktop app: a Svelte front end over the `dactile` library.
//! Each command here passes a call from the window on to `play` or `view`.

pub mod play;
pub mod view;

use dactile::config::Config;
use play::{BuildResult, Card, Change, Home, Open, Opened, Preview, Saved, Settings};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

#[derive(Default)]
struct App {
    cfg: Config,
    open: Option<Open>,
    /// The command line's play was asked for.
    started: bool,
}

type Shared<'a> = State<'a, Mutex<App>>;
type Res<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

#[tauri::command]
fn home(app: Shared) -> Home {
    let mut a = app.lock().unwrap();
    a.cfg = play::load_config();
    play::home(&a.cfg)
}

#[tauri::command]
fn settings(app: Shared) -> Settings {
    play::settings(&app.lock().unwrap().cfg)
}

#[tauri::command]
fn set_setting(app: Shared, key: String, value: serde_json::Value) -> Res<()> {
    play::set(&mut app.lock().unwrap().cfg, &key, &value).map_err(err)
}

#[tauri::command]
fn preview(path: String) -> Res<Preview> {
    play::preview(&PathBuf::from(path)).map_err(err)
}

#[tauri::command]
fn create(app: Shared, template: usize, title: String) -> Res<String> {
    let a = app.lock().unwrap();
    play::create(&a.cfg, template, &title)
        .map(|p| p.display().to_string())
        .map_err(err)
}

#[tauri::command]
fn example(app: Shared) -> Res<String> {
    play::example(&app.lock().unwrap().cfg)
        .map(|p| p.display().to_string())
        .map_err(err)
}

#[tauri::command]
fn open(app: Shared, path: String) -> Res<Opened> {
    let mut a = app.lock().unwrap();
    // The play open before goes first, and its lock with it.
    a.open = None;
    let (open, opened) = play::open(&mut a.cfg, &PathBuf::from(path)).map_err(err)?;
    a.open = Some(open);
    Ok(opened)
}

#[tauri::command]
fn close(app: Shared) {
    app.lock().unwrap().open = None;
}

#[tauri::command]
fn save(app: Shared, text: String, force: bool) -> Res<Saved> {
    let mut a = app.lock().unwrap();
    let open = a.open.as_mut().ok_or("no play open")?;
    open.save(&text, force).map_err(err)
}

#[tauri::command]
fn save_template(text: String, name: String) -> Res<String> {
    dactile::templates::save(&text, &name)
        .map(|p| p.display().to_string())
        .map_err(err)
}

#[tauri::command]
fn view(text: String) -> view::View {
    view::view(&text)
}

#[tauri::command]
fn board(text: String) -> Vec<Card> {
    play::board(&text)
}

#[tauri::command]
fn relocate(text: String, card: usize, to: usize) -> Option<Change> {
    play::relocate(&text, card, to)
}

/// XeLaTeX takes seconds: off the main thread.
#[tauri::command]
async fn build(app: Shared<'_>) -> Res<BuildResult> {
    let path = {
        let a = app.lock().unwrap();
        a.open.as_ref().ok_or("no play open")?.path.clone()
    };
    tauri::async_runtime::spawn_blocking(move || play::build_pdf(&path))
        .await
        .map_err(|e| e.to_string())
}

/// The play named on the command line (`dactile-desktop oyun.tex`), once.
#[tauri::command]
fn startup(app: Shared) -> Option<String> {
    let mut a = app.lock().unwrap();
    if a.started {
        return None;
    }
    a.started = true;
    std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .find(|p| p.extension().is_some_and(|e| e == "tex"))
        .and_then(|p| p.canonicalize().ok())
        .map(|p| p.display().to_string())
}

/// An error in the window, written where `npm run tauri dev` shows it.
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

/// In a debug build with `DACTILE_DEV_EVAL=file`: JavaScript written to that
/// file runs in the window, and the file goes. For trying the app out from
/// a script, with no hands on it.
fn dev_eval(app: tauri::AppHandle) {
    use tauri::Manager;
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
    tauri::Builder::default()
        .manage(Mutex::new(App::default()))
        .setup(|app| {
            dev_eval(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            home,
            settings,
            set_setting,
            preview,
            create,
            example,
            open,
            close,
            save,
            save_template,
            view,
            board,
            relocate,
            build,
            pick,
            startup,
            log,
            pdf,
        ])
        .run(tauri::generate_context!())
        .expect("the window could not open");
}
