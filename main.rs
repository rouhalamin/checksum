// ============================================================================
//  CheckSum.exe — main.rs
//  Native Windows 11 (classic Win32 dialog style) SHA-256 verifier.
//
//  Compact layout: hash box -> file path box + Browse -> [Start/End button]
//  [progress bar] on one row -> four live info fields on ONE row (total
//  size / checked so far / elapsed time / clickable likely-source link) ->
//  single result line (green = match, red = mismatch, computed hash shown
//  only on mismatch).
//
//  Fixes in this revision:
//   - WM_CTLCOLORSTATIC previously returned NULL_BRUSH for the result and
//     credit labels, which told Windows "don't erase the background before
//     repainting" — that's exactly what caused old and new result text to
//     visually overlap between runs. Now returns a proper COLOR_BTNFACE
//     brush so every repaint is a clean erase-then-draw.
//   - The detected source domain is now a real clickable link (opens the
//     browser via ShellExecuteW), with a hand cursor on hover.
//   - The window's own title-bar/taskbar icon is set explicitly at runtime
//     via LoadIconW + WM_SETICON, on top of the icon embedded into the
//     .exe by compiler.py/winres, so it can't come up blank.
//   - Layout is meaningfully shorter: Start/End sits beside the progress
//     bar (not above it), and the four info fields sit in a single row.
// ============================================================================

#![windows_subsystem = "windows"]

use native_windows_derive as nwd;
use native_windows_gui as nwg;
use nwd::NwgUi;
use nwg::NativeUi;

use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use winapi::shared::basetsd::UINT_PTR;
use winapi::shared::windef::{COLORREF, HWND};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::ShellExecuteW;
use winapi::um::uxtheme::SetWindowTheme;
use winapi::um::wingdi::{SetBkMode, SetTextColor, RGB, TRANSPARENT};
use winapi::um::winuser::{
    GetSysColorBrush, GetWindowLongPtrW, IDC_HAND, KillTimer, LoadCursorW, LoadIconW,
    MAKEINTRESOURCEW, MessageBeep, MessageBoxW, SendMessageW, SetCursor, SetTimer,
    SetWindowLongPtrW, COLOR_BTNFACE, GWL_STYLE, ICON_BIG, ICON_SMALL, MB_ICONASTERISK,
    MB_ICONHAND, SW_SHOWNORMAL, WM_CTLCOLORSTATIC, WM_LBUTTONUP, WM_SETCURSOR, WM_SETICON,
    WM_TIMER, WS_MAXIMIZEBOX, WS_THICKFRAME,
};

// ---------------------------------------------------------------------------
// String veil: every human-readable literal is XOR-folded at compile time
// via `obfstr` and only reconstructed in a stack buffer at the moment it's
// used, so a hex/strings dump of the binary won't show plaintext messages.
// ---------------------------------------------------------------------------
mod veil {
    macro_rules! s {
        ($lit:expr) => {
            obfstr::obfstr!($lit)
        };
    }
    pub(crate) use s;
}
use veil::s;

const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4 MiB read chunks: fast on local + slow drives alike
const PROGRESS_TIMER_ID: UINT_PTR = 1;
const PBM_SETBARCOLOR: u32 = 0x0409; // WM_USER + 9
const MAIN_ICON_RESOURCE_ID: u16 = 1; // must match res.set_icon_with_id(..., "1") in build.rs

const GREEN: (u8, u8, u8) = (0, 150, 0);
const RED: (u8, u8, u8) = (196, 0, 0);
const LINK_BLUE: (u8, u8, u8) = (0, 90, 200);

// ---------------------------------------------------------------------------
// Small local (fully offline) heuristic: guess an official source domain
// from the file name only. No network calls are made anywhere in this app.
// ---------------------------------------------------------------------------
fn guess_source(path: &Path) -> &'static str {
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    const KNOWN: &[(&str, &str)] = &[
        ("kali", "kali.org"),
        ("kubuntu", "kubuntu.org"),
        ("xubuntu", "xubuntu.org"),
        ("lubuntu", "lubuntu.me"),
        ("ubuntu", "ubuntu.com"),
        ("debian", "debian.org"),
        ("fedora", "getfedora.org"),
        ("centos", "centos.org"),
        ("rocky", "rockylinux.org"),
        ("almalinux", "almalinux.org"),
        ("mint", "linuxmint.com"),
        ("manjaro", "manjaro.org"),
        ("archlinux", "archlinux.org"),
        ("opensuse", "opensuse.org"),
        ("tails", "tails.net"),
        ("whonix", "whonix.org"),
        ("parrot", "parrotsec.org"),
        ("freebsd", "freebsd.org"),
        ("openbsd", "openbsd.org"),
        ("proxmox", "proxmox.com"),
        ("vmware", "vmware.com"),
        ("virtualbox", "virtualbox.org"),
        ("firefox", "mozilla.org"),
        ("vscode", "code.visualstudio.com"),
        ("python", "python.org"),
        ("nodejs", "nodejs.org"),
        ("node-v", "nodejs.org"),
        ("git-", "git-scm.com"),
        ("7z", "7-zip.org"),
        ("nmap", "nmap.org"),
        ("wireshark", "wireshark.org"),
        ("docker", "docker.com"),
        ("blender", "blender.org"),
        ("gimp", "gimp.org"),
        ("libreoffice", "libreoffice.org"),
        ("obs-studio", "obsproject.com"),
        ("putty", "putty.org"),
        ("filezilla", "filezilla-project.org"),
        ("npp.", "notepad-plus-plus.org"),
    ];

    for (keyword, domain) in KNOWN {
        if name.contains(keyword) {
            return domain;
        }
    }
    "N/A"
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

fn format_elapsed(d: Duration) -> String {
    let secs = d.as_secs();
    if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

/// "1.1.0" -> "1.1" (drops a trailing ".0" patch component for a cleaner
/// on-screen version string). Falls back to the raw string on anything odd.
fn display_version(raw: &str) -> String {
    let parts: Vec<&str> = raw.split('.').collect();
    if parts.len() == 3 && parts[2] == "0" {
        format!("{}.{}", parts[0], parts[1])
    } else {
        raw.to_string()
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---------------------------------------------------------------------------
// UI definition
// ---------------------------------------------------------------------------
#[derive(Default, NwgUi)]
pub struct CheckSumApp {
    #[nwg_control(
        size: (480, 300),
        position: (300, 300),
        title: "CheckSum Verifier",
        flags: "WINDOW|VISIBLE"
    )]
    #[nwg_events( OnWindowClose: [CheckSumApp::exit], OnInit: [CheckSumApp::on_init] )]
    window: nwg::Window,

    #[nwg_control(text: "Expected CheckSum (SHA-256):", position: (15, 12), size: (300, 18))]
    lbl_hash: nwg::Label,

    #[nwg_control(position: (15, 32), size: (450, 24), flags: "VISIBLE", placeholder_text: Some("Paste the official checksum here"))]
    hash_input: nwg::TextInput,

    #[nwg_control(text: "File Path:", position: (15, 64), size: (300, 18))]
    lbl_file: nwg::Label,

    #[nwg_control(position: (15, 84), size: (320, 24), flags: "VISIBLE", placeholder_text: Some("e.g. C:/exp/example.iso"))]
    file_input: nwg::TextInput,

    #[nwg_control(text: "Browse...", position: (340, 83), size: (115, 26))]
    #[nwg_events( OnButtonClick: [CheckSumApp::browse] )]
    browse_btn: nwg::Button,

    #[nwg_control(text: "Start", position: (15, 118), size: (95, 28))]
    #[nwg_events( OnButtonClick: [CheckSumApp::on_action_click] )]
    action_btn: nwg::Button,

    #[nwg_control(range: 0..100, position: (120, 118), size: (345, 28))]
    progress: nwg::ProgressBar,

    #[nwg_control(text: "Total: —", position: (15, 154), size: (110, 18))]
    info_size_label: nwg::Label,

    #[nwg_control(text: "Checked: —", position: (130, 154), size: (110, 18))]
    info_checked_label: nwg::Label,

    #[nwg_control(text: "Time: —", position: (245, 154), size: (85, 18))]
    info_time_label: nwg::Label,

    #[nwg_control(text: "Source: —", position: (335, 154), size: (130, 18))]
    info_source_label: nwg::Label,

    #[nwg_control(text: "", position: (15, 180), size: (450, 46))]
    result_label: nwg::Label,

    #[nwg_control(text: "by Rohulamin Erfani", position: (15, 236), size: (220, 16))]
    credit_label: nwg::Label,

    #[nwg_control(text: "", position: (390, 236), size: (75, 16))]
    version_label: nwg::Label,

    // --- shared state between the worker thread and the GUI thread -------
    progress_counter: RefCell<Option<Arc<AtomicU64>>>,
    total_bytes: RefCell<u64>,
    worker_done: RefCell<Option<Arc<AtomicBool>>>,
    cancel_flag: RefCell<Option<Arc<AtomicBool>>>,
    was_cancelled: RefCell<Option<Arc<AtomicBool>>>,
    computed_hash: Arc<Mutex<Option<String>>>,
    result_state: RefCell<i8>, // 0 = neutral, 1 = success (green), 2 = error (red)
    hashing_active: RefCell<bool>,
    start_time: RefCell<Option<Instant>>,
    current_source_url: RefCell<String>, // empty when there's nothing to open

    credit_font: RefCell<Option<nwg::Font>>,
    info_font: RefCell<Option<nwg::Font>>,
    result_font: RefCell<Option<nwg::Font>>,
}

impl CheckSumApp {
    fn on_init(&self) {
        // Lock the window to a fixed size (no resize border, no maximize box)
        // so it behaves like a classic Windows dialog box.
        unsafe {
            if let Some(hwnd) = self.window.handle.hwnd() {
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let fixed_style = style & !(WS_THICKFRAME as isize) & !(WS_MAXIMIZEBOX as isize);
                SetWindowLongPtrW(hwnd, GWL_STYLE, fixed_style);
            }

            // Force the title-bar / taskbar / Alt-Tab icon to the one
            // embedded into this exe by compiler.py, independently of
            // whatever Explorer's icon cache happens to be showing.
            if let Some(hwnd) = self.window.handle.hwnd() {
                let hinstance = GetModuleHandleW(std::ptr::null());
                let hicon = LoadIconW(hinstance, MAKEINTRESOURCEW(MAIN_ICON_RESOURCE_ID));
                if !hicon.is_null() {
                    SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, hicon as isize);
                    SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, hicon as isize);
                }
            }
        }

        // Disable the progress bar's visual theme so PBM_SETBARCOLOR can
        // actually recolor it (green while running, red while cancelling).
        if let Some(hwnd) = self.progress.handle.hwnd() {
            let empty: Vec<u16> = vec![0];
            unsafe {
                SetWindowTheme(hwnd, empty.as_ptr(), empty.as_ptr());
            }
        }
        self.set_progress_color(GREEN);

        // Subtle, small grey signature — same understated look as the
        // hash box's placeholder text.
        let mut credit_font = nwg::Font::default();
        if nwg::Font::builder()
            .family("Segoe UI")
            .size(13)
            .build(&mut credit_font)
            .is_ok()
        {
            self.credit_label.set_font(Some(&credit_font));
            self.version_label.set_font(Some(&credit_font));
        }
        *self.credit_font.borrow_mut() = Some(credit_font);

        // Bigger, semi-bold font for the four live info fields.
        let mut info_font = nwg::Font::default();
        if nwg::Font::builder()
            .family("Segoe UI")
            .size(14)
            .weight(600)
            .build(&mut info_font)
            .is_ok()
        {
            self.info_size_label.set_font(Some(&info_font));
            self.info_checked_label.set_font(Some(&info_font));
            self.info_time_label.set_font(Some(&info_font));
            self.info_source_label.set_font(Some(&info_font));
        }
        *self.info_font.borrow_mut() = Some(info_font);

        // Bigger, bold font for the final verdict line.
        let mut result_font = nwg::Font::default();
        if nwg::Font::builder()
            .family("Segoe UI")
            .size(16)
            .weight(700)
            .build(&mut result_font)
            .is_ok()
        {
            self.result_label.set_font(Some(&result_font));
        }
        *self.result_font.borrow_mut() = Some(result_font);

        self.version_label
            .set_text(&format!("V {}", display_version(env!("CARGO_PKG_VERSION"))));

        self.result_label.set_text("");
        self.progress.set_pos(0);
        self.install_window_hook();
        self.install_link_hook();
    }

    /// Raw hook on the main window: paints result/credit/source labels with
    /// the right color AND a real background brush (see module doc comment
    /// for why NULL_BRUSH was causing overlapping text), and drives
    /// progress-bar polling via WM_TIMER.
    fn install_window_hook(&self) {
        let result_hwnd: HWND = match self.result_label.handle.hwnd() {
            Some(h) => h,
            None => return,
        };
        let credit_hwnd: HWND = match self.credit_label.handle.hwnd() {
            Some(h) => h,
            None => return,
        };
        let source_hwnd: HWND = match self.info_source_label.handle.hwnd() {
            Some(h) => h,
            None => return,
        };

        // SAFETY: `self` is heap-owned by the Rc<CheckSumApp> that NWG's
        // build_ui() produces and lives for the entire process lifetime,
        // so these raw pointers stay valid for as long as this callback
        // exists (i.e. until the window closes and the process exits).
        let state_ptr: *const RefCell<i8> = &self.result_state;
        let self_ptr: *const CheckSumApp = self;

        // NWG reserves raw-event-handler ids in the 0..=0xFFFF range for
        // its own internal use, so this custom hook must use an id above it.
        let _ = nwg::bind_raw_event_handler(&self.window.handle, 0x1_0000, move |_h, msg, wparam, lparam| {
            if msg == WM_CTLCOLORSTATIC {
                let ctrl_hwnd = lparam as HWND;
                let hdc = wparam as winapi::shared::windef::HDC;
                // A real system brush (not NULL_BRUSH!) so the OS fully
                // erases the previous text before drawing the new text.
                let bg_brush = unsafe { GetSysColorBrush(COLOR_BTNFACE as i32) };

                if ctrl_hwnd == result_hwnd {
                    let state = unsafe { *(*state_ptr).borrow() };
                    let color = match state {
                        1 => RGB(0, 130, 0), // success: green
                        2 => RGB(196, 0, 0), // error: red
                        _ => RGB(0, 0, 0),   // neutral: black
                    };
                    unsafe {
                        SetTextColor(hdc, color);
                        SetBkMode(hdc, TRANSPARENT as i32);
                    }
                    return Some(bg_brush as isize);
                }

                if ctrl_hwnd == credit_hwnd {
                    unsafe {
                        SetTextColor(hdc, RGB(153, 153, 153)); // same subtle grey as placeholder text
                        SetBkMode(hdc, TRANSPARENT as i32);
                    }
                    return Some(bg_brush as isize);
                }

                if ctrl_hwnd == source_hwnd {
                    let (r, g, b) = LINK_BLUE;
                    unsafe {
                        SetTextColor(hdc, RGB(r, g, b));
                        SetBkMode(hdc, TRANSPARENT as i32);
                    }
                    return Some(bg_brush as isize);
                }
            }

            if msg == WM_TIMER && wparam == PROGRESS_TIMER_ID {
                let this = unsafe { &*self_ptr };
                this.poll_progress();
                return Some(0);
            }

            None
        });
    }

    /// Makes the "Source: <domain>" label behave like a real hyperlink:
    /// hand cursor on hover, opens the default browser on click.
    fn install_link_hook(&self) {
        let source_hwnd: HWND = match self.info_source_label.handle.hwnd() {
            Some(h) => h,
            None => return,
        };
        let self_ptr: *const CheckSumApp = self;

        let _ = nwg::bind_raw_event_handler(&self.info_source_label.handle, 0x1_0001, move |_h, msg, _wparam, _lparam| {
            if msg == WM_SETCURSOR {
                unsafe {
                    let cursor = LoadCursorW(std::ptr::null_mut(), IDC_HAND);
                    SetCursor(cursor);
                }
                return Some(1);
            }

            if msg == WM_LBUTTONUP {
                let this = unsafe { &*self_ptr };
                let url = this.current_source_url.borrow().clone();
                if !url.is_empty() {
                    let op = to_wide("open");
                    let url_w = to_wide(&url);
                    unsafe {
                        ShellExecuteW(
                            std::ptr::null_mut(),
                            op.as_ptr(),
                            url_w.as_ptr(),
                            std::ptr::null(),
                            std::ptr::null(),
                            SW_SHOWNORMAL,
                        );
                    }
                }
                return Some(0);
            }

            let _ = source_hwnd;
            None
        });
    }

    fn set_progress_color(&self, rgb: (u8, u8, u8)) {
        if let Some(hwnd) = self.progress.handle.hwnd() {
            let color: COLORREF = RGB(rgb.0, rgb.1, rgb.2);
            unsafe {
                SendMessageW(hwnd, PBM_SETBARCOLOR, 0, color as isize);
            }
        }
    }

    fn browse(&self) {
        let mut dialog = Default::default();
        let built = nwg::FileDialog::builder()
            .title(s!("Select the downloaded file"))
            .action(nwg::FileDialogAction::Open)
            .build(&mut dialog);

        if built.is_err() {
            return;
        }

        if dialog.run(Some(&self.window)) {
            if let Ok(path) = dialog.get_selected_item() {
                self.file_input.set_text(&path.to_string_lossy());
            }
        }
    }

    fn on_action_click(&self) {
        if *self.hashing_active.borrow() {
            // Currently running -> this click means "End" (cancel).
            if let Some(flag) = self.cancel_flag.borrow().as_ref() {
                flag.store(true, Ordering::Relaxed);
            }
            self.set_progress_color(RED);
        } else {
            self.begin_hashing();
        }
    }

    fn begin_hashing(&self) {
        let expected = self.hash_input.text().trim().to_lowercase();
        let path = PathBuf::from(self.file_input.text().trim().to_string());

        if expected.is_empty() || !path.is_file() {
            *self.result_state.borrow_mut() = 2;
            self.result_label
                .set_text(s!("Please provide both a checksum and a valid file path."));
            return;
        }

        let file_len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        *self.total_bytes.borrow_mut() = file_len.max(1);

        let progress_counter = Arc::new(AtomicU64::new(0));
        let done_flag = Arc::new(AtomicBool::new(false));
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancelled_flag = Arc::new(AtomicBool::new(false));

        *self.progress_counter.borrow_mut() = Some(progress_counter.clone());
        *self.worker_done.borrow_mut() = Some(done_flag.clone());
        *self.cancel_flag.borrow_mut() = Some(cancel_flag.clone());
        *self.was_cancelled.borrow_mut() = Some(cancelled_flag.clone());
        *self.computed_hash.lock().unwrap() = None;
        *self.result_state.borrow_mut() = 0;
        *self.start_time.borrow_mut() = Some(Instant::now());
        *self.hashing_active.borrow_mut() = true;

        // Wipe every field from any previous run before this one starts,
        // so nothing from a prior check can linger or blend with the new
        // result.
        self.progress.set_pos(0);
        self.set_progress_color(GREEN);
        self.action_btn.set_text(s!("End"));
        self.browse_btn.set_enabled(false);
        self.hash_input.set_enabled(false);
        self.file_input.set_enabled(false);
        self.result_label.set_text("");

        let source_guess = guess_source(&path);
        *self.current_source_url.borrow_mut() = if source_guess == "N/A" {
            String::new()
        } else {
            format!("https://{}", source_guess)
        };

        self.info_size_label
            .set_text(&format!("{} {}", s!("Total:"), format_bytes(file_len)));
        self.info_checked_label
            .set_text(&format!("{} {}", s!("Checked:"), format_bytes(0)));
        self.info_time_label
            .set_text(&format!("{} 0.0s", s!("Time:")));
        self.info_source_label
            .set_text(&format!("{} {}", s!("Source:"), source_guess));

        let computed_hash_slot = self.computed_hash.clone();

        thread::spawn(move || {
            let mut hasher = Sha256::new();
            if let Ok(mut file) = File::open(&path) {
                let mut buffer = vec![0u8; CHUNK_SIZE];
                loop {
                    if cancel_flag.load(Ordering::Relaxed) {
                        cancelled_flag.store(true, Ordering::Relaxed);
                        break;
                    }
                    match file.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            hasher.update(&buffer[..n]);
                            progress_counter.fetch_add(n as u64, Ordering::Relaxed);
                        }
                        Err(_) => break,
                    }
                }
            }
            if !cancelled_flag.load(Ordering::Relaxed) {
                let hex_hash = hex::encode(hasher.finalize());
                *computed_hash_slot.lock().unwrap() = Some(hex_hash);
            }
            done_flag.store(true, Ordering::Relaxed);
        });

        // Standard Win32 timer with NO callback pointer (last arg = None):
        // Windows posts plain WM_TIMER messages into the GUI thread's
        // message queue — the well-defined, thread-safe way to poll.
        unsafe {
            if let Some(hwnd) = self.window.handle.hwnd() {
                SetTimer(hwnd, PROGRESS_TIMER_ID, 100, None);
            }
        }
    }

    /// Runs on the GUI thread, invoked via WM_TIMER while a hash job is active.
    fn poll_progress(&self) {
        let total = *self.total_bytes.borrow();
        let done_bytes = self
            .progress_counter
            .borrow()
            .as_ref()
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0);
        let pct = ((done_bytes as f64 / total as f64) * 100.0).min(100.0) as u32;
        self.progress.set_pos(pct);
        self.info_checked_label
            .set_text(&format!("{} {}", s!("Checked:"), format_bytes(done_bytes)));

        if let Some(start) = self.start_time.borrow().as_ref() {
            self.info_time_label
                .set_text(&format!("{} {}", s!("Time:"), format_elapsed(start.elapsed())));
        }

        let finished = self
            .worker_done
            .borrow()
            .as_ref()
            .map(|f| f.load(Ordering::Relaxed))
            .unwrap_or(false);

        if !finished {
            return;
        }

        unsafe {
            if let Some(hwnd) = self.window.handle.hwnd() {
                KillTimer(hwnd, PROGRESS_TIMER_ID);
            }
        }

        *self.hashing_active.borrow_mut() = false;
        self.action_btn.set_text(s!("Start"));
        self.browse_btn.set_enabled(true);
        self.hash_input.set_enabled(true);
        self.file_input.set_enabled(true);

        let was_cancelled = self
            .was_cancelled
            .borrow()
            .as_ref()
            .map(|f| f.load(Ordering::Relaxed))
            .unwrap_or(false);

        if was_cancelled {
            self.progress.set_pos(0);
            self.set_progress_color(GREEN);
            *self.result_state.borrow_mut() = 0;
            self.result_label.set_text(s!("Cancelled."));
            return;
        }

        self.progress.set_pos(100);
        self.set_progress_color(GREEN);

        let expected = self.hash_input.text().trim().to_lowercase();
        let computed = self.computed_hash.lock().unwrap().clone().unwrap_or_default();

        if !computed.is_empty() && computed == expected {
            *self.result_state.borrow_mut() = 1;
            self.result_label
                .set_text(s!("Hash matched — this file is safe and complete."));
            unsafe { MessageBeep(MB_ICONASTERISK) };
        } else {
            *self.result_state.borrow_mut() = 2;
            let message = format!(
                "{}\n{} {}",
                s!("Hash does NOT match — this file may be corrupted or unofficial."),
                s!("Computed hash:"),
                computed
            );
            self.result_label.set_text(&message);
            unsafe { MessageBeep(MB_ICONHAND) };
        }
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

/// Instead of silently vanishing on a genuine Rust-level panic, pop up a
/// real message box with the panic details, so any real crash is
/// immediately diagnosable instead of looking identical to an external kill.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let text = format!("{}", info);
        let wide_text = to_wide(&text);
        let wide_title = to_wide("CheckSum - Fatal Error");
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                wide_text.as_ptr(),
                wide_title.as_ptr(),
                MB_ICONHAND,
            );
        }
    }));
}

fn main() {
    install_panic_hook();
    nwg::init().expect("Failed to initialize native-windows-gui");
    let _ = nwg::Font::set_global_family("Segoe UI");
    let _app = CheckSumApp::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}
