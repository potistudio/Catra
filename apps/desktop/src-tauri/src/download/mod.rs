use crate::activity_log::emit_activity_log;
use crate::library::{import_file, DuplicateResolver, LibraryState};
use std::collections::{HashSet, VecDeque};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::SystemTime;
use tauri::{AppHandle, Emitter, Manager};

enum DownloadJob {
    Track(String),
    Playlist(String),
}

struct DownloadQueue {
    jobs: Mutex<VecDeque<DownloadJob>>,
    condvar: Condvar,
    worker_started: Mutex<bool>,
}

impl DownloadQueue {
    fn new() -> Self {
        Self {
            jobs: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            worker_started: Mutex::new(false),
        }
    }

    fn global() -> &'static DownloadQueue {
        static QUEUE: OnceLock<DownloadQueue> = OnceLock::new();
        QUEUE.get_or_init(DownloadQueue::new)
    }

    fn enqueue(&self, job: DownloadJob) {
        {
            let mut jobs = self.jobs.lock().unwrap();
            jobs.push_back(job);
        }
        self.condvar.notify_one();
    }

    fn ensure_worker(&self, app: AppHandle) {
        let mut started = self.worker_started.lock().unwrap();
        if *started {
            return;
        }

        *started = true;
        let queue = Self::global();
        std::thread::spawn(move || queue.run_worker(app));
    }

    fn run_worker(&self, app: AppHandle) {
        loop {
            let job = {
                let mut jobs = self.jobs.lock().unwrap();
                while jobs.is_empty() {
                    jobs = self.condvar.wait(jobs).unwrap();
                }
                jobs.pop_front().unwrap()
            };

            match job {
                DownloadJob::Track(url) => process_track_download(&app, &url),
                DownloadJob::Playlist(url) => process_playlist_download(&app, &url),
            }
        }
    }
}

pub fn download_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("downloads");

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn list_dir_files(dir: &Path) -> Result<HashSet<PathBuf>, String> {
    let mut files = HashSet::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            files.insert(path);
        }
    }
    Ok(files)
}

fn decode_process_output(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }

    #[cfg(windows)]
    {
        let (decoded, _, had_errors) = encoding_rs::SHIFT_JIS.decode(bytes);
        if !had_errors {
            return decoded.into_owned();
        }
    }

    String::from_utf8_lossy(bytes).into_owned()
}

fn find_new_files(dir: &Path, before: &HashSet<PathBuf>) -> Result<Vec<PathBuf>, String> {
    let mut new_files = Vec::new();

    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() && !before.contains(&path) {
            new_files.push(path);
        }
    }

    new_files.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });

    Ok(new_files)
}

fn resolve_downloaded_paths(
    output_dir: &Path,
    before: &HashSet<PathBuf>,
    stdout: &[u8],
) -> Result<Vec<PathBuf>, String> {
    let text = decode_process_output(stdout);
    let mut paths = Vec::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let path = PathBuf::from(line);
        if path.is_file() {
            paths.push(path);
        }
    }

    if !paths.is_empty() {
        return Ok(paths);
    }

    let new_files = find_new_files(output_dir, before)?;
    if new_files.is_empty() {
        return Err("ダウンロードされたファイルが見つかりませんでした".to_string());
    }

    Ok(new_files)
}

fn resolve_downloaded_path(
    output_dir: &Path,
    before: &HashSet<PathBuf>,
    stdout: &[u8],
) -> Result<PathBuf, String> {
    resolve_downloaded_paths(output_dir, before, stdout)?
        .into_iter()
        .next_back()
        .ok_or_else(|| "ダウンロードされたファイルが見つかりませんでした".to_string())
}

fn run_yt_dlp(
    app: &AppHandle,
    url: &str,
    allow_playlist: bool,
) -> Result<(Vec<u8>, HashSet<PathBuf>, PathBuf), String> {
    let output_dir = download_dir(app)?;
    let output_template = output_dir.join("%(uploader)s - %(title)s.%(ext)s");
    let files_before = list_dir_files(&output_dir)?;

    emit_activity_log(
        app,
        "info",
        format!("yt-dlp を実行: {url}"),
        Some(format!("出力先: {}", output_dir.to_string_lossy())),
    );

    let mut args = vec![
        "-x".to_string(),
        "--audio-format".to_string(),
        "mp3".to_string(),
        "--embed-thumbnail".to_string(),
        "--add-metadata".to_string(),
        "--encoding".to_string(),
        "utf-8".to_string(),
        "-o".to_string(),
        output_template.to_string_lossy().to_string(),
        "--print".to_string(),
        "after_move:filepath".to_string(),
    ];

    if !allow_playlist {
        args.push("--no-playlist".to_string());
    }

    args.push(url.to_string());

    let mut child = Command::new("yt-dlp")
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("yt-dlp を実行できませんでした: {e}"))?;

    if let Some(stderr) = child.stderr.take() {
        let app_stderr = app.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    emit_activity_log(&app_stderr, "info", trimmed, None);
                }
            }
        });
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("yt-dlp の完了待ちに失敗しました: {e}"))?;

    if !output.status.success() {
        let stderr = decode_process_output(&output.stderr);
        let message = if stderr.trim().is_empty() {
            format!("yt-dlp が終了コード {} で失敗しました", output.status)
        } else {
            stderr.trim().to_string()
        };
        return Err(message);
    }

    Ok((output.stdout, files_before, output_dir))
}

pub fn download_track(app: &AppHandle, url: &str) -> Result<PathBuf, String> {
    let (stdout, files_before, output_dir) = run_yt_dlp(app, url, false)?;
    let path = resolve_downloaded_path(&output_dir, &files_before, &stdout)?;
    emit_activity_log(
        app,
        "success",
        format!(
            "ダウンロード完了: {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        ),
        Some(path.to_string_lossy().to_string()),
    );
    Ok(path)
}

pub fn download_playlist(app: &AppHandle, url: &str) -> Result<Vec<PathBuf>, String> {
    let (stdout, files_before, output_dir) = run_yt_dlp(app, url, true)?;
    let paths = resolve_downloaded_paths(&output_dir, &files_before, &stdout)?;
    emit_activity_log(
        app,
        "success",
        format!("プレイリストのダウンロード完了: {} 曲", paths.len()),
        Some(url.to_string()),
    );
    Ok(paths)
}

fn import_downloaded_path(app: &AppHandle, path: &Path) -> Result<(), String> {
    emit_activity_log(app, "info", "ライブラリへインポート中...", None);
    let state = app.state::<LibraryState>();
    let resolver = app.state::<DuplicateResolver>();
    import_file(app, &state, &resolver, path)?;
    emit_activity_log(
        app,
        "success",
        "ライブラリへ追加しました",
        Some(path.to_string_lossy().to_string()),
    );
    let _ = app.emit(
        "library-updated",
        path.to_string_lossy().to_string(),
    );
    Ok(())
}

fn process_track_download(app: &AppHandle, url: &str) {
    emit_activity_log(app, "info", format!("ダウンロードを開始: {url}"), None);

    match download_track(app, url).and_then(|path| {
        import_downloaded_path(app, &path).map_err(|error| {
            format!("インポートに失敗しました: {error}")
        })
    }) {
        Ok(()) => {}
        Err(error) => {
            emit_activity_log(app, "error", "ダウンロードに失敗しました", Some(error.clone()));
            let _ = app.emit("download-error", error);
        }
    }
}

fn process_playlist_download(app: &AppHandle, url: &str) {
    emit_activity_log(
        app,
        "info",
        format!("プレイリストの一括ダウンロードを開始: {url}"),
        None,
    );

    let paths = match download_playlist(app, url) {
        Ok(paths) => paths,
        Err(error) => {
            emit_activity_log(app, "error", "プレイリストのダウンロードに失敗しました", Some(error.clone()));
            let _ = app.emit("download-error", error);
            return;
        }
    };

    let total = paths.len();
    let mut imported = 0usize;
    let mut failed = 0usize;

    for (index, path) in paths.into_iter().enumerate() {
        emit_activity_log(
            app,
            "info",
            format!("インポート中 ({}/{})", index + 1, total),
            Some(path.to_string_lossy().to_string()),
        );

        match import_downloaded_path(app, &path) {
            Ok(()) => imported += 1,
            Err(error) => {
                failed += 1;
                emit_activity_log(
                    app,
                    "error",
                    format!("インポートに失敗: {}", path.file_name().unwrap_or_default().to_string_lossy()),
                    Some(error),
                );
            }
        }
    }

    emit_activity_log(
        app,
        "success",
        format!("一括ダウンロード完了: 成功 {imported} 曲、失敗 {failed} 曲"),
        Some(url.to_string()),
    );
}

fn enqueue_job(app: AppHandle, job: DownloadJob) {
    let queue = DownloadQueue::global();
    queue.ensure_worker(app);
    queue.enqueue(job);
}

pub fn download_and_import(app: AppHandle, url: String) {
    enqueue_job(app, DownloadJob::Track(url));
}

pub fn download_playlist_and_import(app: AppHandle, url: String) {
    enqueue_job(app, DownloadJob::Playlist(url));
}
