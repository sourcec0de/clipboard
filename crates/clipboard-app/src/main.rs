//! Native desktop launcher for `clipboard`.
//!
//! Strictly local-only: makes zero network calls.

#![forbid(unsafe_code)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::process::Termination;

use clipboard_platform::native_platform;
use clipboard_ui::App;
use clipboard_ui::LaunchConfig;
use clipboard_ui::configure_launch;
use clipboard_ui::theme_tokens;
use nsh_core::platform::Appearance;
#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> impl Termination {
    env_logger::init();
    configure_launch(LaunchConfig::new(native_platform));

    let root = platform::Root::of::<App>().tokens(theme_tokens());

    platform::run_app(
        platform::AppSource::Rust(root),
        platform::AppOptions {
            title: "Clipboard".to_owned(),
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            app_id: "dev.sourcec0de.clipboard".to_owned(),
            size: (520, 500),
            appearance: Appearance::System,
            #[cfg(target_os = "macos")]
            activation_policy: platform::ActivationPolicy::Accessory,
            #[cfg(target_os = "macos")]
            keep_alive_on_close: true,
            ..platform::AppOptions::default()
        },
    )
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("clipboard desktop launcher requires macOS, Linux, or Windows.");
