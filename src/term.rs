//! Terminal introspection — width detection and tty check.
//!
//! No external crates beyond what `anstream` already pulls in.

/// Returns the terminal width in columns, or `80` as fallback.
///
/// Uses `TIOCGWINSZ` ioctl on Unix and `GetConsoleScreenBufferInfo` on Windows.
/// Returns `80` when output is piped or size cannot be determined.
pub fn width() -> usize {
    terminal_width().unwrap_or(80)
}

/// Returns `true` if stderr is an interactive terminal.
pub fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stderr().is_terminal()
}

#[cfg(unix)]
fn terminal_width() -> Option<usize> {
    // TIOCGWINSZ via inline libc call — no dependency needed
    #[repr(C)]
    struct Winsize {
        rows: u16,
        cols: u16,
        _xpixel: u16,
        _ypixel: u16,
    }

    let ws = Winsize {
        rows: 0,
        cols: 0,
        _xpixel: 0,
        _ypixel: 0,
    };
    // TIOCGWINSZ = 0x5413 on Linux, 0x40087468 on macOS
    #[cfg(target_os = "macos")]
    const TIOCGWINSZ: u64 = 0x40087468;
    #[cfg(not(target_os = "macos"))]
    const TIOCGWINSZ: u64 = 0x5413;

    let ret = unsafe { libc_ioctl(STDERR_FILENO, TIOCGWINSZ, &ws as *const Winsize) };
    if ret == 0 && ws.cols > 0 {
        Some(ws.cols as usize)
    } else {
        None
    }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "ioctl"]
    fn libc_ioctl(fd: i32, request: u64, ...) -> i32;
}

#[cfg(unix)]
const STDERR_FILENO: i32 = 2;

#[cfg(windows)]
fn terminal_width() -> Option<usize> {
    use windows_sys::Win32::System::Console::{
        GetConsoleScreenBufferInfo, GetStdHandle, CONSOLE_SCREEN_BUFFER_INFO, STD_ERROR_HANDLE,
    };
    unsafe {
        let handle = GetStdHandle(STD_ERROR_HANDLE);
        if handle == 0 {
            return None;
        }
        let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
        if GetConsoleScreenBufferInfo(handle, &mut info) != 0 {
            let w = info.srWindow.Right - info.srWindow.Left + 1;
            if w > 0 {
                Some(w as usize)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn terminal_width() -> Option<usize> {
    None
}
