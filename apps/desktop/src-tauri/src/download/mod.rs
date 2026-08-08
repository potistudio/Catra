use crate::activity_log::emit_activity_log;
use crate::library::{import_file, DuplicateResolver, LibraryState};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadPhase {
    Idle,
    Queued,
    Downloading,
    Importing,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressPayload {
    pub status: String,
    pub phase: DownloadPhase,
    pub percent: Option<f64>,
    pub current: Option<usize>,
    pub total: Option<usize>,
    pub queue_position: Option<usize>,
    pub message: Option<String>,
}

struct JobCompletion {
    success: bool,
    error: Option<String>,
    finished_at: Instant,
}

struct ProgressState {
    active_url: Option<String>,
    active_phase: DownloadPhase,
    percent: Option<f64>,
    current: Option<usize>,
    total: Option<usize>,
    message: Option<String>,
    queued_urls: Vec<String>,
    completions: HashMap<String, JobCompletion>,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            active_url: None,
            active_phase: DownloadPhase::Idle,
            percent: None,
            current: None,
            total: None,
            message: None,
            queued_urls: Vec::new(),
            completions: HashMap::new(),
        }
    }
}

struct ProgressTracker {
    state: Mutex<ProgressState>,
}

impl ProgressTracker {
    fn global() -> &'static ProgressTracker {
        static TRACKER: OnceLock<ProgressTracker> = OnceLock::new();
        TRACKER.get_or_init(|| ProgressTracker {
            state: Mutex::new(ProgressState::default()),
        })
    }

    fn prune_completions(state: &mut ProgressState) {
        let ttl = Duration::from_secs(30);
        state
            .completions
            .retain(|_, completion| completion.finished_at.elapsed() < ttl);
    }

    fn emit_progress(app: &AppHandle, state: &ProgressState) {
        let payload = DownloadProgressPayload {
            status: match state.active_phase {
                DownloadPhase::Idle => "idle".to_string(),
                DownloadPhase::Queued => "queued".to_string(),
                DownloadPhase::Downloading => "downloading".to_string(),
                DownloadPhase::Importing => "importing".to_string(),
            },
            phase: state.active_phase,
            percent: state.percent,
            current: state.current,
            total: state.total,
            queue_position: None,
            message: state.message.clone(),
        };
        let _ = app.emit("download-progress", payload);
    }

    fn set_queued_urls(&self, app: &AppHandle, urls: Vec<String>) {
        let mut state = self.state.lock().unwrap();
        state.queued_urls = urls;
        Self::prune_completions(&mut state);
        Self::emit_progress(app, &state);
    }

    fn start_job(&self, app: &AppHandle, url: &str, is_playlist: bool) {
        let mut state = self.state.lock().unwrap();
        state.active_url = Some(normalize_download_url(url));
        state.active_phase = DownloadPhase::Downloading;
        state.percent = Some(0.0);
        state.current = None;
        state.total = None;
        state.message = Some(if is_playlist {
            "プレイリストをダウンロード中...".to_string()
        } else {
            "ダウンロード中...".to_string()
        });
        state
            .queued_urls
            .retain(|queued_url| queued_url != &normalize_download_url(url));
        Self::prune_completions(&mut state);
        Self::emit_progress(app, &state);
    }

    fn update_from_yt_dlp_line(&self, app: &AppHandle, line: &str) {
        let mut state = self.state.lock().unwrap();
        if state.active_phase != DownloadPhase::Downloading {
            return;
        }

        if let Some((current, total)) = parse_playlist_item_line(line) {
            state.current = Some(current);
            state.total = Some(total);
            state.message = Some(format!("{current}/{total} 曲をダウンロード中"));
        }

        if let Some(percent) = parse_download_percent_line(line) {
            state.percent = Some(percent);
            if let (Some(current), Some(total)) = (state.current, state.total) {
                state.message = Some(format!(
                    "{current}/{total} 曲 ({percent:.0}%)"
                ));
            } else {
                state.message = Some(format!("ダウンロード中 {percent:.0}%"));
            }
        }

        Self::emit_progress(app, &state);
    }

    fn set_importing(
        &self,
        app: &AppHandle,
        message: &str,
        current: Option<usize>,
        total: Option<usize>,
    ) {
        let mut state = self.state.lock().unwrap();
        state.active_phase = DownloadPhase::Importing;
        state.percent = None;
        state.current = current;
        state.total = total;
        state.message = Some(message.to_string());
        Self::emit_progress(app, &state);
    }

    fn finish_job(&self, app: &AppHandle, url: &str, success: bool, error: Option<String>) {
        let normalized = normalize_download_url(url);
        let mut state = self.state.lock().unwrap();
        state.completions.insert(
            normalized,
            JobCompletion {
                success,
                error: error.clone(),
                finished_at: Instant::now(),
            },
        );
        state.active_url = None;
        state.active_phase = DownloadPhase::Idle;
        state.percent = None;
        state.current = None;
        state.total = None;
        state.message = None;
        Self::prune_completions(&mut state);
        Self::emit_progress(app, &state);
    }

    fn snapshot_for_url(&self, url: &str) -> DownloadProgressPayload {
        let normalized = normalize_download_url(url);
        let state = self.state.lock().unwrap();

        if let Some(completion) = state.completions.get(&normalized) {
            return DownloadProgressPayload {
                status: if completion.success {
                    "completed".to_string()
                } else {
                    "error".to_string()
                },
                phase: DownloadPhase::Idle,
                percent: if completion.success {
                    Some(100.0)
                } else {
                    None
                },
                current: None,
                total: None,
                queue_position: None,
                message: completion.error.clone().or_else(|| {
                    if completion.success {
                        Some("完了".to_string())
                    } else {
                        None
                    }
                }),
            };
        }

        if state.active_url.as_deref() == Some(normalized.as_str()) {
            return DownloadProgressPayload {
                status: match state.active_phase {
                    DownloadPhase::Idle => "idle".to_string(),
                    DownloadPhase::Queued => "queued".to_string(),
                    DownloadPhase::Downloading => "downloading".to_string(),
                    DownloadPhase::Importing => "importing".to_string(),
                },
                phase: state.active_phase,
                percent: state.percent,
                current: state.current,
                total: state.total,
                queue_position: None,
                message: state.message.clone(),
            };
        }

        if let Some(position) = state
            .queued_urls
            .iter()
            .position(|queued_url| queued_url == &normalized)
        {
            return DownloadProgressPayload {
                status: "queued".to_string(),
                phase: DownloadPhase::Queued,
                percent: None,
                current: None,
                total: None,
                queue_position: Some(position + 1),
                message: Some(format!("待機中 ({}/{})", position + 1, state.queued_urls.len())),
            };
        }

        DownloadProgressPayload {
            status: "idle".to_string(),
            phase: DownloadPhase::Idle,
            percent: None,
            current: None,
            total: None,
            queue_position: None,
            message: None,
        }
    }
}

fn parse_download_percent_line(line: &str) -> Option<f64> {
    let trimmed = line.trim();
    if !trimmed.starts_with("[download]") {
        return None;
    }

    let after_tag = trimmed.strip_prefix("[download]")?.trim_start();
    if after_tag.starts_with("Downloading item") {
        return None;
    }

    let percent_index = after_tag.find('%')?;
    let before_percent = after_tag[..percent_index].trim();
    let value = before_percent.split_whitespace().last()?;
    value.parse().ok()
}

fn parse_playlist_item_line(line: &str) -> Option<(usize, usize)> {
    let rest = line
        .trim()
        .strip_prefix("[download] Downloading item ")?
        .trim();
    let mut parts = rest.split_whitespace();
    let current = parts.next()?.parse().ok()?;
    if parts.next()? != "of" {
        return None;
    }
    let total = parts.next()?.parse().ok()?;
    Some((current, total))
}

pub fn normalize_download_url(url: &str) -> String {
    let trimmed = url.trim();
    let without_fragment = trimmed.split('#').next().unwrap_or(trimmed);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    without_query.trim_end_matches('/').to_string()
}

pub fn get_download_progress(url: &str) -> DownloadProgressPayload {
    ProgressTracker::global().snapshot_for_url(url)
}

pub fn get_active_download_progress() -> DownloadProgressPayload {
    let tracker = ProgressTracker::global();
    let state = tracker.state.lock().unwrap();

    if let Some(url) = state.active_url.clone() {
        drop(state);
        return tracker.snapshot_for_url(&url);
    }

    if let Some(url) = state.queued_urls.first().cloned() {
        drop(state);
        return tracker.snapshot_for_url(&url);
    }

    DownloadProgressPayload {
        status: "idle".to_string(),
        phase: DownloadPhase::Idle,
        percent: None,
        current: None,
        total: None,
        queue_position: None,
        message: None,
    }
}

fn is_yt_dlp_progress_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("[download]")
        || trimmed.starts_with("[ExtractAudio]")
        || trimmed.starts_with("[Metadata]")
}

fn process_yt_dlp_stderr_line(app: &AppHandle, line: &str) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }

    ProgressTracker::global().update_from_yt_dlp_line(app, trimmed);
    if !is_yt_dlp_progress_line(trimmed) {
        emit_activity_log(app, "info", trimmed, None);
    }
}

fn read_yt_dlp_stderr(app: AppHandle, stderr: impl Read + Send + 'static) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 4096];

        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(count) => {
                    for &byte in &chunk[..count] {
                        if byte == b'\n' || byte == b'\r' {
                            if !buffer.is_empty() {
                                let line = decode_process_output(&buffer);
                                process_yt_dlp_stderr_line(&app, &line);
                                buffer.clear();
                            }
                        } else {
                            buffer.push(byte);
                        }
                    }
                }
                Err(_) => break,
            }
        }

        if !buffer.is_empty() {
            let line = decode_process_output(&buffer);
            process_yt_dlp_stderr_line(&app, &line);
        }
    });
}

fn sync_queue_snapshot(app: &AppHandle, jobs: &VecDeque<DownloadJob>) {
    let urls = jobs
        .iter()
        .map(|job| match job {
            DownloadJob::Track(url) | DownloadJob::Playlist(url) => normalize_download_url(url),
        })
        .collect();
    ProgressTracker::global().set_queued_urls(app, urls);
}

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

    fn enqueue(&self, job: DownloadJob, app: &AppHandle) {
        {
            let mut jobs = self.jobs.lock().unwrap();
            jobs.push_back(job);
            sync_queue_snapshot(app, &jobs);
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
                let job = jobs.pop_front().unwrap();
                sync_queue_snapshot(&app, &jobs);
                job
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
        "--newline".to_string(),
        "--progress-delta".to_string(),
        "1".to_string(),
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
        read_yt_dlp_stderr(app.clone(), stderr);
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
    ProgressTracker::global().set_importing(app, "ライブラリへインポート中...", None, None);
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
    ProgressTracker::global().start_job(app, url, false);
    emit_activity_log(app, "info", format!("ダウンロードを開始: {url}"), None);

    match download_track(app, url).and_then(|path| {
        import_downloaded_path(app, &path).map_err(|error| {
            format!("インポートに失敗しました: {error}")
        })
    }) {
        Ok(()) => {
            ProgressTracker::global().finish_job(app, url, true, None);
        }
        Err(error) => {
            emit_activity_log(app, "error", "ダウンロードに失敗しました", Some(error.clone()));
            let _ = app.emit("download-error", error.clone());
            ProgressTracker::global().finish_job(app, url, false, Some(error));
        }
    }
}

fn process_playlist_download(app: &AppHandle, url: &str) {
    ProgressTracker::global().start_job(app, url, true);
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
            let _ = app.emit("download-error", error.clone());
            ProgressTracker::global().finish_job(app, url, false, Some(error));
            return;
        }
    };

    let total = paths.len();
    let mut imported = 0usize;
    let mut failed = 0usize;

    for (index, path) in paths.into_iter().enumerate() {
        ProgressTracker::global().set_importing(
            app,
            &format!("インポート中 ({}/{})", index + 1, total),
            Some(index + 1),
            Some(total),
        );
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
    ProgressTracker::global().finish_job(
        app,
        url,
        failed == 0,
        if failed == 0 {
            None
        } else {
            Some(format!("成功 {imported} 曲、失敗 {failed} 曲"))
        },
    );
}

fn enqueue_job(app: AppHandle, job: DownloadJob) {
    let queue = DownloadQueue::global();
    queue.ensure_worker(app.clone());
    queue.enqueue(job, &app);
}

pub fn download_and_import(app: AppHandle, url: String) {
    enqueue_job(app, DownloadJob::Track(normalize_download_url(&url)));
}

pub fn download_playlist_and_import(app: AppHandle, url: String) {
    enqueue_job(app, DownloadJob::Playlist(normalize_download_url(&url)));
}
