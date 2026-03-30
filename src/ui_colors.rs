//! @efficiency-role: util-pure
//!
//! UI - ANSI Color Functions

use crate::*;

pub(crate) fn ansi_grey(s: &str) -> String {
    format!("\x1b[90m{s}\x1b[0m")
}

pub(crate) fn ansi_dim_gray(s: &str) -> String {
    format!("\x1b[2;90m{s}\x1b[0m")
}

pub(crate) fn ansi_orange(s: &str) -> String {
    format!("\x1b[38;2;222;33;142m{s}\x1b[0m")
}

pub(crate) fn ansi_pale_yellow(s: &str) -> String {
    format!("\x1b[38;5;229m{s}\x1b[0m")
}

pub(crate) fn ansi_paler_yellow(s: &str) -> String {
    format!("\x1b[38;5;179m{s}\x1b[0m")
}

pub(crate) fn ansi_soft_gold(s: &str) -> String {
    format!("\x1b[38;5;180m{s}\x1b[0m")
}

pub(crate) fn ansi_soft_green(s: &str) -> String {
    format!("\x1b[38;5;114m{s}\x1b[0m")
}
