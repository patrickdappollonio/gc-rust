use is_terminal::IsTerminal;
use std::fmt;

// Platforms that are based on Unix support Emojis if the system language is a UTF-8 one.
#[cfg(all(unix, not(target_os = "macos")))]
lazy_static::lazy_static! {
    static ref IS_LANG_UTF8: bool = {
        match std::env::var("LANG") {
            Ok(lang) => lang.to_uppercase().ends_with("UTF-8"),
            _ => false,
        }
    };
}

// The new Windows Terminal does support emojis. Currently, the terminal will
// set the environment variable `WT_SESSION`. This can be used to check if the
// user uses that specific app.
#[cfg(windows)]
fn platform_supports_emoji() -> bool {
    std::env::var("WT_SESSION").is_ok()
}

// macOS by default has emoji support.
#[cfg(target_os = "macos")]
fn platform_supports_emoji() -> bool {
    true
}

// On unix systems the enabled language decides whether emojis are supported or
// not.
#[cfg(all(unix, not(target_os = "macos")))]
fn platform_supports_emoji() -> bool {
    *IS_LANG_UTF8
}

#[cfg(all(not(unix), not(windows)))]
fn platform_supports_emoji() -> bool {
    false
}

/// Check if stdout supports Emoji output.
///
/// Platforms support is determined by checking these conditions:
///     - macOS has Emoji support by default
///     - Unix systems have support if the active language supports them.
///     - Windows machines running the new Terminal app support Emojis.
///
/// Additionally, this function recognizes terminal multiplexers as terminals.
///
/// # Returns
///
/// `true` if stdout is a terminal and the platform supports Emoji output, `false` otherwise.
fn supports_emoji() -> bool {
    let is_terminal = std::io::stdout().is_terminal() || is_terminal_multiplexer();
    platform_supports_emoji() && is_terminal
}

/// Check if we're running inside a terminal multiplexer.
///
/// This function detects various terminal multiplexers by checking for their
/// characteristic environment variables:
/// - tmux: TMUX
/// - GNU Screen: STY
/// - Zellij: ZELLIJ
/// - byobu: BYOBU_TTY
fn is_terminal_multiplexer() -> bool {
    std::env::var("TMUX").is_ok()
        || std::env::var("STY").is_ok()
        || std::env::var("ZELLIJ").is_ok()
        || std::env::var("BYOBU_TTY").is_ok()
}

/// An emoji with safety fallback.
///
/// The struct wraps an emoji and only renders it on platforms that actually
/// support it. On non-supported platforms the fallback value is being rendered.
///
/// Support is determined by two factors:
///
/// 1) The processes stdout has to be a tty.
/// 2) Platform dependent:
///     - macOS has emoji support by default
///     - Unix systems have support if the active language supports them.
///     - Windows machines running the new Terminal app support emojis.
pub struct Emoji<'a>(pub &'a str, pub &'a str);

impl fmt::Display for Emoji<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if supports_emoji() {
            write!(f, "{}", self.0)
        } else {
            write!(f, "{}", self.1)
        }
    }
}

impl<'a> From<(&'a str, &'a str)> for Emoji<'a> {
    fn from(v: (&'a str, &'a str)) -> Self {
        Emoji(v.0, v.1)
    }
}
