// Fires a native desktop notification when possible.

use crate::detect;

// Fires a native desktop notification when possible; otherwise it just
// prints, so a run over SSH or on a bare system never fails on this.
pub fn notify(title: &str, body: &str) {
    if cfg!(target_os = "linux") && detect::has_tool("notify-send") {
        let _ = std::process::Command::new("notify-send")
            .arg(title)
            .arg(body)
            .status();
    } else if cfg!(target_os = "macos") {
        let script = format!(r#"display notification "{body}" with title "{title}""#);
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .status();
    } else if cfg!(target_os = "windows") && detect::has_tool("msg") {
        let _ = std::process::Command::new("msg")
            .arg("*")
            .arg(format!("{title}: {body}"))
            .status();
    } else {
        println!("{title}: {body}");
    }
}
