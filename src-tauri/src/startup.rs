use crate::state::toast;
use std::os::windows::process::CommandExt;
use std::process::Command;
use tauri::AppHandle;

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const TASK: &str = "Keyframe";
const NO_WINDOW: u32 = 0x0800_0000;

fn exe_path() -> Result<String, String> {
    let p = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(p.to_string_lossy().trim_start_matches(r"\\?\").to_string())
}

fn run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .creation_flags(NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn enable_run_key() -> Result<(), String> {
    let value = format!("\"{}\"", exe_path()?);
    if !run("reg", &["add", RUN_KEY, "/v", "Keyframe", "/t", "REG_SZ", "/d", value.as_str(), "/f"]) {
        return Err("could not write the startup entry".into());
    }
    run("reg", &["add", APPROVED_KEY, "/v", "Keyframe", "/t", "REG_BINARY", "/d", "020000000000000000000000", "/f"]);
    Ok(())
}

fn disable_run_key() {
    for name in ["Keyframe", "keyframe"] {
        run("reg", &["delete", RUN_KEY, "/v", name, "/f"]);
    }
}

fn task_exists() -> bool {
    run("schtasks", &["/Query", "/TN", TASK])
}

fn elevated_schtasks(args: &[&str]) -> Result<(), String> {
    let list = args
        .iter()
        .map(|a| format!("'\"{}\"'", a.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    let script = format!(
        "$p = Start-Process -FilePath 'schtasks.exe' -ArgumentList {list} -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode"
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script.as_str()])
        .creation_flags(NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err("Windows did not apply the change. The administrator prompt may have been declined.".into())
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn create_task() -> Result<(), String> {
    let exe = exe_path()?;
    let dir = std::path::Path::new(&exe).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let user = format!(
        "{}\\{}",
        std::env::var("USERDOMAIN").unwrap_or_default(),
        std::env::var("USERNAME").unwrap_or_default()
    );
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>Starts Keyframe when you log in.</Description></RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
      <Delay>PT5S</Delay>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
      <WorkingDirectory>{dir}</WorkingDirectory>
    </Exec>
  </Actions>
</Task>"#,
        user = xml_escape(&user),
        exe = xml_escape(&exe),
        dir = xml_escape(&dir),
    );
    let path = std::env::temp_dir().join("keyframe-task.xml");
    let mut bytes = vec![0xFF, 0xFE];
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    let path_str = path.to_string_lossy().to_string();
    let res = elevated_schtasks(&["/Create", "/TN", TASK, "/XML", path_str.as_str(), "/F"]);
    let _ = std::fs::remove_file(&path);
    res
}

fn delete_task() -> Result<(), String> {
    if task_exists() {
        elevated_schtasks(&["/Delete", "/TN", TASK, "/F"])
    } else {
        Ok(())
    }
}

/// Called when the user changes "Start with Windows" or "Run as administrator".
/// With admin rights it uses a scheduled task (the only way Windows starts an elevated app at login
/// without a prompt); otherwise a normal startup entry.
pub fn apply(app: &AppHandle, launch: bool, admin: bool) {
    let res = if launch && admin {
        disable_run_key();
        create_task()
    } else if launch {
        delete_task().and_then(|_| enable_run_key())
    } else {
        disable_run_key();
        delete_task()
    };
    if let Err(e) = res {
        toast(app, "error", format!("Start with Windows: {e}"));
    }
}

/// Silent refresh at every launch (keeps the registered path current). Never shows a prompt.
pub fn at_launch(launch: bool, admin: bool) {
    if cfg!(debug_assertions) {
        return;
    }
    if launch && !admin {
        let _ = enable_run_key();
    } else if !launch {
        disable_run_key();
    }
}
