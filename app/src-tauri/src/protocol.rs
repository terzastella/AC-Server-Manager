use std::path::Path;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn run(args: &[&str]) -> Result<String, String> {
    #[cfg(windows)]
    let mut command: Command = {
        let mut command = Command::new("reg");
        command.args(args);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    };
    #[cfg(not(windows))]
    let mut command: Command = {
        let mut command = Command::new("reg");
        command.args(args);
        command
    };
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn register_protocol(exe: &Path) -> Result<(), String> {
    let exe = exe.to_string_lossy().to_string();
    let command = format!("\"{}\" \"%1\"", exe);
    run(&[
        "add",
        "HKCU\\Software\\Classes\\acservercontrol",
        "/ve",
        "/d",
        "URL:AC Server Control",
        "/f",
    ])?;
    run(&[
        "add",
        "HKCU\\Software\\Classes\\acservercontrol",
        "/v",
        "URL Protocol",
        "/d",
        "",
        "/f",
    ])?;
    run(&[
        "add",
        "HKCU\\Software\\Classes\\acservercontrol\\shell\\open\\command",
        "/ve",
        "/d",
        &command,
        "/f",
    ])?;
    Ok(())
}

pub fn protocol_registered(exe: &Path) -> bool {
    let expected = exe.to_string_lossy().to_lowercase();
    match run(&[
        "query",
        "HKCU\\Software\\Classes\\acservercontrol\\shell\\open\\command",
        "/ve",
    ]) {
        Ok(text) => text.to_lowercase().contains(&expected),
        Err(_) => false,
    }
}
