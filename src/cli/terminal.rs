//! Terminal capability detection and utilities

use owo_colors::{colors::css, OwoColorize};

/// Detects whether colored output should be enabled
pub fn supports_color() -> bool {
    supports_color::on(supports_color::Stream::Stdout).is_some()
}

/// Detects terminal width, returning None if not available
pub fn terminal_width() -> Option<u16> {
    terminal_size::terminal_size().map(|(w, _)| w.0)
}

/// Check if terminal is narrow (< 60 columns)
pub fn is_narrow() -> bool {
    terminal_width().map_or(false, |w| w < 60)
}

/// Extension trait for colorizing output
pub trait Colorize {
    /// Color as success (green)
    fn success(&self) -> String;
    /// Color as warning (amber)
    fn warning(&self) -> String;
    /// Color as info (blue)
    fn info(&self) -> String;
    /// Dim the text
    fn dim(&self) -> String;
}

impl Colorize for str {
    fn success(&self) -> String {
        if supports_color() {
            self.fg::<css::Green>().to_string()
        } else {
            self.to_string()
        }
    }

    fn warning(&self) -> String {
        if supports_color() {
            self.fg::<css::Orange>().to_string()
        } else {
            self.to_string()
        }
    }

    fn info(&self) -> String {
        if supports_color() {
            self.fg::<css::LightBlue>().to_string()
        } else {
            self.to_string()
        }
    }

    fn dim(&self) -> String {
        if supports_color() {
            self.dimmed().to_string()
        } else {
            self.to_string()
        }
    }
}

impl Colorize for String {
    fn success(&self) -> String {
        self.as_str().success()
    }

    fn warning(&self) -> String {
        self.as_str().warning()
    }

    fn info(&self) -> String {
        self.as_str().info()
    }

    fn dim(&self) -> String {
        self.as_str().dim()
    }
}
