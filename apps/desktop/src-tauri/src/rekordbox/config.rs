use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RekordboxConfig {
    pub version: Option<String>,
    pub install_dir: Option<PathBuf>,
    pub app_dir: Option<PathBuf>,
    pub db_dir: Option<PathBuf>,
    pub db_path: Option<PathBuf>,
    pub analysis_root: Option<PathBuf>,
    pub settings_root: Option<PathBuf>,
}

pub fn load_config() -> Result<RekordboxConfig, String> {
    let pioneer_app_dir = pioneer_app_dir()?;
    let options = read_options_json(&pioneer_app_dir)?;
    let rb_app_dir = pioneer_app_dir.join("rekordbox6");
    let settings_dir = read_master_db_directory(&rb_app_dir)?;
    let install = detect_install_dir()?;

    let db_path = options
        .db_path
        .or_else(|| settings_dir.as_ref().map(|dir| dir.join("master.db")))
        .filter(|path| path.is_file());

    Ok(RekordboxConfig {
        version: install.as_ref().map(|info| info.version.clone()),
        install_dir: install.map(|info| info.path),
        app_dir: Some(rb_app_dir),
        db_dir: db_path.as_ref().and_then(|path| path.parent().map(Path::to_path_buf)),
        db_path: db_path.clone(),
        analysis_root: options.analysis_root,
        settings_root: options.settings_root,
    })
}

#[derive(Debug, Default)]
struct OptionsJson {
    db_path: Option<PathBuf>,
    analysis_root: Option<PathBuf>,
    settings_root: Option<PathBuf>,
}

struct InstallInfo {
    path: PathBuf,
    version: String,
}

fn pioneer_app_dir() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir().ok_or_else(|| "could not resolve app data directory".to_string())?;
    let path = data_dir.join("Pioneer");
    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!("Pioneer app data directory not found: {}", path.display()))
    }
}

fn read_options_json(pioneer_app_dir: &Path) -> Result<OptionsJson, String> {
    let path = pioneer_app_dir
        .join("rekordboxAgent")
        .join("storage")
        .join("options.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|error| format!("invalid options.json: {error}"))?;

    let mut options = OptionsJson::default();
    let Some(entries) = value.get("options").and_then(|value| value.as_array()) else {
        return Ok(options);
    };

    for entry in entries {
        let Some(pair) = entry.as_array() else {
            continue;
        };
        if pair.len() != 2 {
            continue;
        }
        let Some(key) = pair[0].as_str() else {
            continue;
        };
        let value = &pair[1];
        match key {
            "db-path" => options.db_path = value.as_str().map(PathBuf::from),
            "analysisDataPath" => options.analysis_root = value.as_str().map(PathBuf::from),
            "settingsDataPath" => options.settings_root = value.as_str().map(PathBuf::from),
            _ => {}
        }
    }

    Ok(options)
}

fn read_master_db_directory(rb_app_dir: &Path) -> Result<Option<PathBuf>, String> {
    let path = rb_app_dir.join("rekordbox3.settings");
    if !path.is_file() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("<VALUE name=\"masterDbDirectory\" val=\"") {
            if let Some(value) = rest.strip_suffix('"') {
                return Ok(Some(PathBuf::from(value)));
            }
        }
    }

    Ok(None)
}

fn detect_install_dir() -> Result<Option<InstallInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        let program_files = std::env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .ok_or_else(|| "ProgramFiles environment variable is not set".to_string())?;
        return Ok(newest_version_dir(&program_files.join("rekordbox")));
    }

    #[cfg(target_os = "macos")]
    {
        for version in ["7", "6"] {
            let path = PathBuf::from(format!("/Applications/rekordbox {version}.app"));
            if path.is_dir() {
                return Ok(Some(InstallInfo {
                    path,
                    version: version.to_string(),
                }));
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = ();
    }

    Ok(None)
}

fn newest_version_dir(root: &Path) -> Option<InstallInfo> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(version) = name.strip_prefix("rekordbox ") {
            candidates.push(InstallInfo {
                path,
                version: version.to_string(),
            });
        }
    }

    candidates.sort_by(|left, right| right.version.cmp(&left.version));
    candidates.into_iter().next()
}

pub fn is_rekordbox_running() -> bool {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System};

    // Process name is enough to detect rekordbox.exe; skip full process metadata refresh.
    let mut system = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    );
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    system.processes().values().any(|process| {
        process
            .name()
            .to_string_lossy()
            .trim_end_matches(".exe")
            .eq_ignore_ascii_case("rekordbox")
    })
}
