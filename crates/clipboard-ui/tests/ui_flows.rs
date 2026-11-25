//! Headless NSH integration tests covering General, Shortcuts, Blacklist, and the Quick Paste Overlay.

use std::path::PathBuf;
use std::sync::Mutex;

use clipboard_core::AppSettings;
use clipboard_core::ClipboardHistory;
use clipboard_core::Clipping;
use clipboard_platform::FakePlatform;
use clipboard_platform::PlatformHost;
use clipboard_ui::App;
use clipboard_ui::LaunchConfig;
use clipboard_ui::OverlayHarness;
use clipboard_ui::configure_launch;
use clipboard_ui::theme_tokens;
use nsh_core::Points;
use nsh_core::Size;
use nsh_core::platform::Os;
use nsh_engine::EngineConfig;
use nsh_engine::Root;
use nsh_testing::Headless;

static TEST_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

fn make_fake_host() -> Box<dyn PlatformHost> {
    let dir = TEST_DIR
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_else(|| PathBuf::from("/tmp/clipboard-headless-test"));
    Box::new(FakePlatform::new(dir))
}

fn start_preferences_headless() -> (Headless, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    if let Ok(mut guard) = TEST_DIR.lock() {
        *guard = Some(dir.path().to_path_buf());
    }
    configure_launch(LaunchConfig::for_tests(make_fake_host));

    let root = Root::of::<App>().tokens(theme_tokens());
    let mut config = EngineConfig::new(root);
    config.viewport = Size::new(Points(520.0), Points(540.0));
    config.environment.os = Os::MacOs;

    let app = Headless::start(config).expect("start preferences headless");
    (app, dir)
}

fn minimal_png_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, 4, 4);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    let pixels = [48_u8, 120, 240, 255].repeat(16);
    writer.write_image_data(&pixels).expect("png data");
    drop(writer);
    out
}

fn start_overlay_headless() -> (Headless, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let data_dir = dir.path().join("data");

    let mut history = ClipboardHistory::new();
    let settings = AppSettings::default();
    history.push_clipping(
        Clipping::from_image(&minimal_png_bytes(), 4, 4, Some("CleanShot.app".to_owned())),
        settings.history_limit,
    );
    history.push_clipping(
        Clipping::from_text(
            "negative.".to_owned(),
            None,
            Some("Messages.app".to_owned()),
        ),
        settings.history_limit,
    );
    history.push_clipping(
        Clipping::from_text(
            "Is there anything else we need to do with our pull request?".to_owned(),
            None,
            Some("Slack.app".to_owned()),
        ),
        settings.history_limit,
    );
    history.push_clipping(
        Clipping::from_text("Okay.".to_owned(), None, Some("Slack.app".to_owned())),
        settings.history_limit,
    );
    history.save_to_dir(&data_dir).expect("seed history");

    if let Ok(mut guard) = TEST_DIR.lock() {
        *guard = Some(dir.path().to_path_buf());
    }
    configure_launch(LaunchConfig::for_tests(make_fake_host));

    let root = Root::of::<OverlayHarness>().tokens(theme_tokens());
    let mut config = EngineConfig::new(root);
    config.viewport = Size::new(Points(460.0), Points(520.0));
    config.environment.os = Os::MacOs;

    let app = Headless::start(config).expect("start overlay headless");
    (app, dir)
}

#[test]
fn preferences_general_shortcuts_and_blacklist_flows() {
    let (mut app, _dir) = start_preferences_headless();

    // General tab renders all expected controls and zero-network badge (no iCloud Sync).
    app.wait_for("Ignore clippings larger than:")
        .expect("general size limit visible");
    app.wait_for("Keep the last 100 clippings:")
        .expect("history slider label visible");
    app.wait_for("Always Paste Plain Text")
        .expect("plain text checkbox visible");
    app.wait_for("Quick Paste Menu:")
        .expect("quick paste menu visible");
    app.wait_for("Release to Paste")
        .expect("release to paste visible");
    app.wait_for("Enable Interaction Sounds")
        .expect("sounds checkbox visible");
    app.wait_for("Open automatically at login")
        .expect("login checkbox visible");
    app.wait_for("Zero Network")
        .expect("zero network badge visible");

    // Shortcuts tab renders Actions and Paste Recent Clippings sections.
    app.press("role:button[Shortcuts]")
        .expect("switch to shortcuts");
    app.wait_for("Edit global keyboard shortcuts.")
        .expect("shortcuts heading visible");
    app.wait_for("Show Clipboard")
        .expect("show clipboard action visible");
    app.wait_for("⇧⌘V")
        .expect("default show clipboard shortcut visible");
    app.wait_for("Toggle Sequential Paste Mode")
        .expect("sequential paste toggle visible");
    app.wait_for("Paste from Sequential Queue")
        .expect("sequential paste action visible");
    app.wait_for("Paste Clipping 1")
        .expect("paste clipping 1 visible");

    // Blacklist tab shows default sensitive apps and allows adding a new app.
    app.press("role:button[Blacklist]")
        .expect("switch to blacklist");
    app.wait_for("Applications added to the blacklist are ignored by Clipboard.")
        .expect("blacklist heading visible");
    app.wait_for("Keychain Access.app")
        .expect("keychain access listed");
    app.wait_for("Passwords.app").expect("passwords listed");

    app.press("label:Add application to blacklist")
        .expect("open add app panel");
    app.press("role:button[Terminal.app]")
        .expect("add terminal to blacklist");
    app.wait_for("Terminal.app")
        .expect("terminal added to blacklist");
}

#[test]
fn overlay_displays_clippings_searches_and_opens_filter_and_preview() {
    let (mut app, _dir) = start_overlay_headless();

    app.wait_for("Clipboard").expect("overlay header visible");
    app.wait_for("Okay.").expect("first clipping visible");
    app.wait_for("negative.").expect("third clipping visible");
    app.wait_for("text:/^Image 4×4/")
        .expect("bounded image clipping visible");

    // Open preview drawer via hover action on first row.
    app.press("label:Preview clipping >> nth=0")
        .expect("open preview drawer");
    app.wait_for("text:/^Slack\\.app/")
        .expect("source app metadata shown");

    // Type to search immediately filters the list.
    app.type_text("text:Type to Search...", "negative")
        .expect("type search query");
    app.wait_for("negative.")
        .expect("matching clipping stays visible");
}

#[test]
fn manifest_enforces_zero_network_capability() {
    let manifest = include_str!("../../../nsh.json");
    assert!(
        manifest.contains("\"clipboard\": true"),
        "clipboard capability should be declared"
    );
    assert!(
        !manifest.contains("\"network\""),
        "nsh.json must never declare network capabilities"
    );
}
