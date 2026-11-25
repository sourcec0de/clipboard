//! Production OS implementations of [`crate::PlatformHost`] for macOS, Linux, and Windows.
//!
//! Strictly local-only: no network sockets, no HTTP clients, no telemetry.

use std::env;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use crate::ClipboardContent;
use crate::ClipboardSnapshot;
use crate::ClipboardWrite;
use crate::CursorPoint;
use crate::InstalledApp;
use crate::PlatformError;
use crate::PlatformHost;
use crate::SoundEffect;

/// Production OS host implementation for macOS, Linux, and Windows.
#[derive(Debug)]
pub struct NativePlatform {
    last_change_token: AtomicU64,
}

impl Default for NativePlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl NativePlatform {
    /// Creates a new [`NativePlatform`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_change_token: AtomicU64::new(0),
        }
    }
}

impl PlatformHost for NativePlatform {
    fn config_dir(&self) -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            if let Some(home) = env::var_os("HOME") {
                return PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("Clipboard");
            }
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
                return PathBuf::from(xdg).join("clipboard");
            }
            if let Some(home) = env::var_os("HOME") {
                return PathBuf::from(home).join(".config").join("clipboard");
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = env::var_os("APPDATA") {
                return PathBuf::from(appdata).join("Clipboard");
            }
        }
        env::temp_dir().join("clipboard-config")
    }

    fn data_dir(&self) -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            self.config_dir()
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(xdg) = env::var_os("XDG_DATA_HOME") {
                return PathBuf::from(xdg).join("clipboard");
            }
            if let Some(home) = env::var_os("HOME") {
                return PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join("clipboard");
            }
            env::temp_dir().join("clipboard-data")
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(local) = env::var_os("LOCALAPPDATA") {
                return PathBuf::from(local).join("Clipboard");
            }
            self.config_dir()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            env::temp_dir().join("clipboard-data")
        }
    }

    fn read_clipboard(&self) -> Result<Option<ClipboardSnapshot>, PlatformError> {
        #[cfg(target_os = "macos")]
        {
            read_clipboard_macos(&self.last_change_token)
        }
        #[cfg(target_os = "linux")]
        {
            read_clipboard_linux(&self.last_change_token)
        }
        #[cfg(target_os = "windows")]
        {
            read_clipboard_windows(&self.last_change_token)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = &self.last_change_token;
            Ok(None)
        }
    }

    fn write_clipboard(&self, payload: &ClipboardWrite) -> Result<(), PlatformError> {
        #[cfg(target_os = "macos")]
        {
            write_clipboard_macos(payload)
        }
        #[cfg(target_os = "linux")]
        {
            write_clipboard_linux(payload)
        }
        #[cfg(target_os = "windows")]
        {
            write_clipboard_windows(payload)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = payload;
            Ok(())
        }
    }

    fn simulate_paste(&self) -> Result<(), PlatformError> {
        #[cfg(target_os = "macos")]
        {
            let out = Command::new("osascript")
                .args([
                    "-e",
                    "tell application \"System Events\" to keystroke \"v\" using command down",
                ])
                .output()
                .map_err(|err| PlatformError::PasteSimulation(err.to_string()))?;
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
                return Err(PlatformError::PasteSimulation(if stderr.is_empty() {
                    "Grant Accessibility permission to paste into other apps".to_owned()
                } else {
                    stderr
                }));
            }
            Ok(())
        }
        #[cfg(target_os = "linux")]
        {
            if env::var_os("WAYLAND_DISPLAY").is_some() {
                let status = Command::new("wtype")
                    .args(["-M", "ctrl", "v", "-m", "ctrl"])
                    .status();
                if matches!(status, Ok(s) if s.success()) {
                    return Ok(());
                }
            }
            let status = Command::new("xdotool")
                .args(["key", "--clearmodifiers", "ctrl+v"])
                .status()
                .map_err(|err| PlatformError::PasteSimulation(err.to_string()))?;
            if !status.success() {
                return Err(PlatformError::PasteSimulation(
                    "xdotool/wtype failed to send Ctrl+V".to_owned(),
                ));
            }
            Ok(())
        }
        #[cfg(target_os = "windows")]
        {
            let status = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('^v')",
                ])
                .status()
                .map_err(|err| PlatformError::PasteSimulation(err.to_string()))?;
            if !status.success() {
                return Err(PlatformError::PasteSimulation(
                    "SendKeys failed to send Ctrl+V".to_owned(),
                ));
            }
            Ok(())
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Ok(())
        }
    }

    fn frontmost_app(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            let script = concat!(
                "ObjC.import('AppKit');",
                "var app = $.NSWorkspace.sharedWorkspace.frontmostApplication;",
                "if (app) { ObjC.unwrap(app.localizedName) + '.app'; } else { ''; }"
            );
            let out = Command::new("osascript")
                .args(["-l", "JavaScript", "-e", script])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let name = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            if name.is_empty() { None } else { Some(name) }
        }
        #[cfg(target_os = "linux")]
        {
            let out = Command::new("xdotool")
                .args(["getactivewindow", "getwindowclassname"])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let name = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            if name.is_empty() { None } else { Some(name) }
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    fn capture_frontmost_target(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            let own_pid = std::process::id();
            let script = concat!(
                "ObjC.import('AppKit');",
                "var app = $.NSWorkspace.sharedWorkspace.frontmostApplication;",
                "if (app && !app.isNil()) { String(app.processIdentifier); } else { ''; }"
            );
            let out = Command::new("osascript")
                .args(["-l", "JavaScript", "-e", script])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let pid_str = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            let pid: u32 = pid_str.parse().ok()?;
            if pid == own_pid {
                return None;
            }
            Some(pid_str)
        }
        #[cfg(target_os = "linux")]
        {
            let out = Command::new("xdotool")
                .arg("getactivewindow")
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let win_id = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            if win_id.is_empty() {
                None
            } else {
                Some(win_id)
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    fn restore_focus(&self, target: Option<&str>) {
        #[cfg(target_os = "macos")]
        {
            let script = target.and_then(|s| s.parse::<u32>().ok()).map_or_else(
                || {
                    let own_pid = std::process::id();
                    format!(
                        "tell application \"System Events\" to set visible of (first application process whose unix id is {own_pid}) to false"
                    )
                },
                |target_pid| {
                    format!(
                        "tell application \"System Events\" to set frontmost of (first application process whose unix id is {target_pid}) to true"
                    )
                },
            );
            let _ = Command::new("osascript").args(["-e", &script]).output();
            std::thread::sleep(std::time::Duration::from_millis(90));
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(win_id) = target {
                let _ = Command::new("xdotool")
                    .args(["windowactivate", "--sync", win_id])
                    .output();
            }
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = target;
        }
    }

    fn list_installed_apps(&self) -> Vec<InstalledApp> {
        #[cfg(target_os = "macos")]
        {
            list_macos_apps()
        }
        #[cfg(target_os = "linux")]
        {
            list_linux_apps()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Vec::new()
        }
    }

    fn mouse_cursor_position(&self) -> Option<CursorPoint> {
        #[cfg(target_os = "macos")]
        {
            let script = concat!(
                "ObjC.import('AppKit');",
                "var p = $.NSEvent.mouseLocation;",
                "var h = $.NSScreen.mainScreen ? $.NSScreen.mainScreen.frame.size.height : 900;",
                "Math.round(p.x) + ',' + Math.round(h - p.y);"
            );
            let out = Command::new("osascript")
                .args(["-l", "JavaScript", "-e", script])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let raw = String::from_utf8_lossy(&out.stdout);
            let mut parts = raw.trim().split(',');
            let x = parts.next()?.parse::<f32>().ok()?;
            let y = parts.next()?.parse::<f32>().ok()?;
            Some(CursorPoint::new(x, y))
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    fn set_launch_at_login(&self, enabled: bool) -> Result<(), PlatformError> {
        #[cfg(target_os = "macos")]
        {
            let Some(home) = env::var_os("HOME") else {
                return Err(PlatformError::LaunchAtLogin(
                    "HOME environment variable is not set".to_owned(),
                ));
            };
            let agents_dir = PathBuf::from(home).join("Library").join("LaunchAgents");
            let plist_path = agents_dir.join("dev.sourcec0de.clipboard.plist");
            if !enabled {
                if plist_path.exists() {
                    fs::remove_file(&plist_path)?;
                }
                return Ok(());
            }
            fs::create_dir_all(&agents_dir)?;
            let exe =
                env::current_exe().map_err(|err| PlatformError::LaunchAtLogin(err.to_string()))?;
            let plist = format!(
                concat!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                    "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" ",
                    "\"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
                    "<plist version=\"1.0\">\n<dict>\n",
                    "  <key>Label</key><string>dev.sourcec0de.clipboard</string>\n",
                    "  <key>ProgramArguments</key><array><string>{}</string></array>\n",
                    "  <key>RunAtLoad</key><true/>\n",
                    "</dict>\n</plist>\n"
                ),
                exe.display()
            );
            fs::write(&plist_path, plist)?;
            Ok(())
        }
        #[cfg(target_os = "linux")]
        {
            let autostart_dir = env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
                .ok_or_else(|| {
                    PlatformError::LaunchAtLogin("HOME/XDG_CONFIG_HOME not set".to_owned())
                })?
                .join("autostart");
            let desktop_path = autostart_dir.join("dev.sourcec0de.clipboard.desktop");
            if !enabled {
                if desktop_path.exists() {
                    fs::remove_file(&desktop_path)?;
                }
                return Ok(());
            }
            fs::create_dir_all(&autostart_dir)?;
            let exe =
                env::current_exe().map_err(|err| PlatformError::LaunchAtLogin(err.to_string()))?;
            let desktop = format!(
                "[Desktop Entry]\nType=Application\nName=Clipboard\nExec={}\nX-GNOME-Autostart-enabled=true\n",
                exe.display()
            );
            fs::write(&desktop_path, desktop)?;
            Ok(())
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = enabled;
            Ok(())
        }
    }

    fn play_sound(&self, effect: SoundEffect) {
        #[cfg(target_os = "macos")]
        {
            let file = match effect {
                SoundEffect::Copy => "/System/Library/Sounds/Tink.aiff",
                SoundEffect::Paste => "/System/Library/Sounds/Pop.aiff",
                SoundEffect::Error => "/System/Library/Sounds/Basso.aiff",
            };
            let _ = Command::new("afplay")
                .arg(file)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let sound_id = match effect {
                SoundEffect::Copy => "message-new-instant",
                SoundEffect::Paste => "button-pressed",
                SoundEffect::Error => "dialog-error",
            };
            let _ = Command::new("canberra-gtk-play")
                .args(["-i", sound_id])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = effect;
        }
    }

    fn platform_name(&self) -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "macOS"
        }
        #[cfg(target_os = "linux")]
        {
            "Linux"
        }
        #[cfg(target_os = "windows")]
        {
            "Windows"
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            "Unknown"
        }
    }
}

/// Decodes PNG dimensions (`width`, `height`) from raw PNG bytes without decoding full pixels.
#[must_use]
pub(crate) fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().ok()?;
    let info = reader.info();
    Some((info.width, info.height))
}

#[cfg(target_os = "macos")]
fn read_clipboard_macos(
    last_token: &AtomicU64,
) -> Result<Option<ClipboardSnapshot>, PlatformError> {
    use std::hash::Hash;
    use std::hash::Hasher;

    // Fast path (~1-2ms): read UTF-8 text via `pbpaste` without spawning JXA/osascript.
    let text_out = Command::new("pbpaste")
        .output()
        .map_err(|err| PlatformError::ClipboardRead(err.to_string()))?;
    if !text_out.status.success() {
        return Ok(None);
    }

    if !text_out.stdout.is_empty() {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        b"text:".hash(&mut hasher);
        text_out.stdout.hash(&mut hasher);
        let token = hasher.finish();
        let prev = last_token.swap(token, Ordering::SeqCst);
        if prev == token {
            return Ok(None);
        }

        let text = String::from_utf8_lossy(&text_out.stdout).into_owned();
        // Check transient/concealed markers & source app only when content actually changed.
        let (is_transient, source_app) = read_macos_pasteboard_meta();
        return Ok(Some(ClipboardSnapshot::new(
            ClipboardContent::Text { text, html: None },
            source_app,
            is_transient,
            token,
        )));
    }

    // Empty text pasteboard: check if an image is present.
    let png_bytes = read_macos_pasteboard_png()?;
    if png_bytes.is_empty() {
        return Ok(None);
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    b"img:".hash(&mut hasher);
    png_bytes.hash(&mut hasher);
    let token = hasher.finish();
    let prev = last_token.swap(token, Ordering::SeqCst);
    if prev == token {
        return Ok(None);
    }
    let (width, height) = png_dimensions(&png_bytes).unwrap_or((1, 1));
    let (is_transient, source_app) = read_macos_pasteboard_meta();
    Ok(Some(ClipboardSnapshot::new(
        ClipboardContent::Image {
            png_bytes,
            width,
            height,
        },
        source_app,
        is_transient,
        token,
    )))
}

#[cfg(target_os = "macos")]
fn read_macos_pasteboard_meta() -> (bool, Option<String>) {
    let meta_script = concat!(
        "ObjC.import('AppKit');",
        "var pb = $.NSPasteboard.generalPasteboard;",
        "var types = pb.types ? ObjC.unwrap(pb.types).map(function(t){ return ObjC.unwrap(t); }) : [];",
        "var sensitive = types.some(function(t){ return ",
        "  t === 'org.nspasteboard.TransientType' || ",
        "  t === 'org.nspasteboard.ConcealedType' || ",
        "  t === 'org.nspasteboard.AutoGeneratedType' || ",
        "  t.indexOf('onepassword') !== -1 || ",
        "  t.indexOf('TransientPasteboardType') !== -1; });",
        "var app = $.NSWorkspace.sharedWorkspace.frontmostApplication;",
        "var appName = app ? (ObjC.unwrap(app.localizedName) + '.app') : '';",
        "(sensitive ? '1' : '0') + '\\t' + appName;"
    );
    let Ok(out) = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", meta_script])
        .output()
    else {
        return (false, None);
    };
    if !out.status.success() {
        return (false, None);
    }
    let line = String::from_utf8_lossy(&out.stdout);
    let mut parts = line.trim_end_matches(['\r', '\n']).split('\t');
    let is_transient = parts.next() == Some("1");
    let source_app = parts
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    (is_transient, source_app)
}

#[cfg(target_os = "macos")]
fn read_macos_pasteboard_png() -> Result<Vec<u8>, PlatformError> {
    let tmp = env::temp_dir().join(format!("clipboard-img-{}.png", std::process::id()));
    let script = format!(
        concat!(
            "ObjC.import('AppKit');",
            "var pb = $.NSPasteboard.generalPasteboard;",
            "var data = pb.dataForType('public.png');",
            "if (!data || data.isNil()) {{",
            "  var tiff = pb.dataForType('public.tiff');",
            "  if (tiff && !tiff.isNil()) {{",
            "    var rep = $.NSBitmapImageRep.imageRepWithData(tiff);",
            "    if (rep && !rep.isNil()) {{",
            "      data = rep.representationUsingTypeProperties($.NSBitmapImageFileTypePNG, $({{}}));",
            "    }}",
            "  }}",
            "}}",
            "if (data && !data.isNil()) {{ data.writeToFileAtomically('{}', true); 'ok'; }} else {{ ''; }}"
        ),
        tmp.display()
    );
    let out = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", &script])
        .output()
        .map_err(|err| PlatformError::ClipboardRead(err.to_string()))?;
    if !out.status.success() || !tmp.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(&tmp).unwrap_or_default();
    let _ = fs::remove_file(&tmp);
    Ok(bytes)
}

#[cfg(target_os = "macos")]
fn write_clipboard_macos(payload: &ClipboardWrite) -> Result<(), PlatformError> {
    use std::io::Write;

    match payload {
        ClipboardWrite::PlainText(text) | ClipboardWrite::RichText { text, .. } => {
            let mut child = Command::new("pbcopy")
                .stdin(Stdio::piped())
                .spawn()
                .map_err(|err| PlatformError::ClipboardWrite(err.to_string()))?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(text.as_bytes())
                    .map_err(|err| PlatformError::ClipboardWrite(err.to_string()))?;
            }
            let status = child
                .wait()
                .map_err(|err| PlatformError::ClipboardWrite(err.to_string()))?;
            if !status.success() {
                return Err(PlatformError::ClipboardWrite(
                    "pbcopy exited with non-zero status".to_owned(),
                ));
            }
            Ok(())
        }
        ClipboardWrite::ImagePng(bytes) => {
            let tmp = env::temp_dir().join(format!("clipboard-write-{}.png", std::process::id()));
            fs::write(&tmp, bytes)?;
            let script = format!(
                concat!(
                    "ObjC.import('AppKit');",
                    "var data = $.NSData.dataWithContentsOfFile('{}');",
                    "if (data && !data.isNil()) {{",
                    "  var pb = $.NSPasteboard.generalPasteboard;",
                    "  pb.clearContents;",
                    "  pb.setDataForType(data, 'public.png');",
                    "}}"
                ),
                tmp.display()
            );
            let out = Command::new("osascript")
                .args(["-l", "JavaScript", "-e", &script])
                .output()
                .map_err(|err| PlatformError::ClipboardWrite(err.to_string()))?;
            let _ = fs::remove_file(&tmp);
            if !out.status.success() {
                return Err(PlatformError::ClipboardWrite(
                    String::from_utf8_lossy(&out.stderr).trim().to_owned(),
                ));
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
fn list_macos_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    let dirs = [
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    for dir in dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("app")
                && let Some(name) = path.file_name().and_then(|n| n.to_str()).map(str::to_owned)
            {
                apps.push(InstalledApp::new(name.clone(), name, Some(path)));
            }
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    apps
}

#[cfg(target_os = "linux")]
fn read_clipboard_linux(
    last_token: &AtomicU64,
) -> Result<Option<ClipboardSnapshot>, PlatformError> {
    use std::hash::Hash;
    use std::hash::Hasher;

    let is_wayland = env::var_os("WAYLAND_DISPLAY").is_some();
    let types_out = if is_wayland {
        Command::new("wl-paste").arg("--list-types").output()
    } else {
        Command::new("xclip")
            .args(["-o", "-selection", "clipboard", "-t", "TARGETS"])
            .output()
    };

    let Ok(types_out) = types_out else {
        return Ok(None);
    };
    if !types_out.status.success() {
        return Ok(None);
    }

    let types_str = String::from_utf8_lossy(&types_out.stdout);
    let is_transient = types_str.contains("x-kde-passwordManagerHint");
    let has_png = types_str.contains("image/png");
    let has_text = types_str.contains("text/plain") || types_str.contains("UTF8_STRING");

    let content = if has_text {
        let out = if is_wayland {
            Command::new("wl-paste").arg("--no-newline").output()
        } else {
            Command::new("xclip")
                .args(["-o", "-selection", "clipboard"])
                .output()
        }
        .map_err(|err| PlatformError::ClipboardRead(err.to_string()))?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        if text.is_empty() {
            return Ok(None);
        }
        ClipboardContent::Text { text, html: None }
    } else if has_png {
        let out = if is_wayland {
            Command::new("wl-paste")
                .args(["--type", "image/png"])
                .output()
        } else {
            Command::new("xclip")
                .args(["-o", "-selection", "clipboard", "-t", "image/png"])
                .output()
        }
        .map_err(|err| PlatformError::ClipboardRead(err.to_string()))?;
        if out.stdout.is_empty() {
            return Ok(None);
        }
        let (width, height) = png_dimensions(&out.stdout).unwrap_or((1, 1));
        ClipboardContent::Image {
            png_bytes: out.stdout,
            width,
            height,
        }
    } else {
        return Ok(None);
    };

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    match &content {
        ClipboardContent::Text { text, .. } => text.hash(&mut hasher),
        ClipboardContent::Image { png_bytes, .. } => png_bytes.hash(&mut hasher),
    }
    let token = hasher.finish();
    let prev = last_token.swap(token, Ordering::SeqCst);
    if prev == token {
        return Ok(None);
    }

    Ok(Some(ClipboardSnapshot::new(
        content,
        None,
        is_transient,
        token,
    )))
}

#[cfg(target_os = "linux")]
fn write_clipboard_linux(payload: &ClipboardWrite) -> Result<(), PlatformError> {
    use std::io::Write;

    let is_wayland = env::var_os("WAYLAND_DISPLAY").is_some();
    let (mut child, bytes): (std::process::Child, &[u8]) = match payload {
        ClipboardWrite::PlainText(text) | ClipboardWrite::RichText { text, .. } => {
            let child = if is_wayland {
                Command::new("wl-copy").stdin(Stdio::piped()).spawn()
            } else {
                Command::new("xclip")
                    .args(["-i", "-selection", "clipboard"])
                    .stdin(Stdio::piped())
                    .spawn()
            }
            .map_err(|err| PlatformError::ClipboardWrite(err.to_string()))?;
            (child, text.as_bytes())
        }
        ClipboardWrite::ImagePng(png_bytes) => {
            let child = if is_wayland {
                Command::new("wl-copy")
                    .args(["--type", "image/png"])
                    .stdin(Stdio::piped())
                    .spawn()
            } else {
                Command::new("xclip")
                    .args(["-i", "-selection", "clipboard", "-t", "image/png"])
                    .stdin(Stdio::piped())
                    .spawn()
            }
            .map_err(|err| PlatformError::ClipboardWrite(err.to_string()))?;
            (child, png_bytes.as_slice())
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(bytes)
            .map_err(|err| PlatformError::ClipboardWrite(err.to_string()))?;
    }
    let _ = child.wait();
    Ok(())
}

#[cfg(target_os = "linux")]
fn list_linux_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    let mut dirs = vec![PathBuf::from("/usr/share/applications")];
    if let Some(home) = env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }
    for dir in dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("desktop")
                && let Some(stem) = path.file_stem().and_then(|n| n.to_str()).map(str::to_owned)
            {
                apps.push(InstalledApp::new(stem.clone(), stem, Some(path)));
            }
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    apps
}

#[cfg(target_os = "windows")]
fn read_clipboard_windows(
    last_token: &AtomicU64,
) -> Result<Option<ClipboardSnapshot>, PlatformError> {
    use std::hash::Hash;
    use std::hash::Hasher;

    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", "Get-Clipboard -Raw"])
        .output()
        .map_err(|err| PlatformError::ClipboardRead(err.to_string()))?;
    if !out.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    if text.is_empty() {
        return Ok(None);
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    let token = hasher.finish();
    let prev = last_token.swap(token, Ordering::SeqCst);
    if prev == token {
        return Ok(None);
    }
    Ok(Some(ClipboardSnapshot::new(
        ClipboardContent::Text { text, html: None },
        None,
        false,
        token,
    )))
}

#[cfg(target_os = "windows")]
fn write_clipboard_windows(payload: &ClipboardWrite) -> Result<(), PlatformError> {
    use std::io::Write;

    match payload {
        ClipboardWrite::PlainText(text) | ClipboardWrite::RichText { text, .. } => {
            let mut child = Command::new("clip")
                .stdin(Stdio::piped())
                .spawn()
                .map_err(|err| PlatformError::ClipboardWrite(err.to_string()))?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(text.as_bytes())
                    .map_err(|err| PlatformError::ClipboardWrite(err.to_string()))?;
            }
            let _ = child.wait();
            Ok(())
        }
        ClipboardWrite::ImagePng(_) => Ok(()),
    }
}
