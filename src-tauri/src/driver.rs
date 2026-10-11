use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverStatus {
    pub dll_found: bool,
    pub installer_found: bool,
    pub active: bool,
    pub message: String,
}

fn candidates(app: &AppHandle, file: &str) -> Vec<PathBuf> {
    let mut v = vec![];
    if let Ok(r) = app.path().resource_dir() {
        v.push(r.join("interception").join(file));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            v.push(d.join("interception").join(file));
            v.push(d.join(file));
        }
    }
    #[cfg(debug_assertions)]
    v.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("interception").join(file));
    v
}

fn find(app: &AppHandle, file: &str) -> Option<PathBuf> {
    candidates(app, file).into_iter().find(|p| p.exists())
}

fn plain(p: &PathBuf) -> PathBuf {
    PathBuf::from(p.to_string_lossy().trim_start_matches(r"\\?\").to_string())
}

pub fn dll_path(app: &AppHandle) -> Option<PathBuf> {
    find(app, "interception.dll").map(|p| plain(&p))
}

pub fn status(app: &AppHandle) -> DriverStatus {
    let dll = dll_path(app);
    let installer = find(app, "install-interception.exe");
    let (active, message) = match &dll {
        None => (false, "interception.dll is missing from the app folder.".to_string()),
        Some(p) => match crate::inject::probe(p) {
            Ok(()) => (true, "Driver active.".to_string()),
            Err(_) => (false, "Driver not active. Install it, then restart Windows.".to_string()),
        },
    };
    DriverStatus { dll_found: dll.is_some(), installer_found: installer.is_some(), active, message }
}

pub fn run_installer(app: &AppHandle, flag: &str) -> Result<(), String> {
    let exe = find(app, "install-interception.exe").ok_or("install-interception.exe is missing from the app folder.")?;
    let exe = plain(&exe).to_string_lossy().replace('\'', "''");
    let script = format!(
        "$p = Start-Process -FilePath '{exe}' -ArgumentList '{flag}' -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode"
    );
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script.as_str()]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "The installer did not finish (exit code {}). Approve the administrator prompt and try again.",
            out.status.code().unwrap_or(-1)
        ))
    }
}
