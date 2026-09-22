// ============================================================================
//  CheckSum.exe — main.rs
//  Native Windows (classic Win32 dialog style) multi-algorithm checksum
//  verifier / calculator.
//
//  Architecture overview
//  ----------------------
//  - UI is declared with native-windows-gui / native-windows-derive, kept as
//    a fixed-size classic dialog (no theming, no custom-drawn "modern" chrome)
//    per the project's design goal: Native Windows look, cleaner layout.
//  - Hashing runs on a background thread; the GUI thread polls progress via
//    a WM_TIMER (100ms) and never blocks. Cancellation is a simple
//    AtomicBool checked between chunk reads.
//  - Algorithm selection is a small enum (`Algorithm`) with a matching
//    `HasherState` enum that wraps the concrete hasher for whichever
//    algorithm was chosen. This keeps the hot read/hash loop identical for
//    every algorithm (only `HasherState::update` differs per variant).
//  - "Verify" vs "Calculate" is a single bool driven by two grouped radio
//    buttons; it only changes whether the Expected Hash field is required
//    and whether a match/mismatch verdict is computed at the end.
//  - Digital-signature checking (Authenticode) is implemented as a small,
//    isolated, best-effort feature using direct WinVerifyTrust FFI bindings
//    (wintrust.dll). It never blocks or fails the hash workflow — any error
//    from the API is mapped to a plain "Not available" / "Not signed" state.
//
//  Security note (see also the in-app "Verification Information" panel and
//  the project docs): a hash match proves file INTEGRITY against the
//  checksum the user supplied — it does not, by itself, prove the file is
//  free of malware, nor that the checksum itself came from a trustworthy
//  place. The UI is written to never claim more than that.
// ============================================================================

#![windows_subsystem = "windows"]

use native_windows_derive as nwd;
use native_windows_gui as nwg;
use nwd::NwgUi;
use nwg::NativeUi;

use blake2::Blake2b512;
use digest::Digest;
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};

use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use winapi::shared::basetsd::UINT_PTR;
use winapi::shared::minwindef::{HGLOBAL, MAX_PATH};
use winapi::shared::windef::{COLORREF, HWND};
use winapi::shared::windef::RECT;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::{DragAcceptFiles, DragFinish, DragQueryFileW, ShellExecuteW, HDROP};
use winapi::um::uxtheme::SetWindowTheme;
use winapi::um::wingdi::{CreateSolidBrush, SetBkMode, SetTextColor, RGB, TRANSPARENT};
use winapi::um::winuser::{
    CloseClipboard, EmptyClipboard, FillRect, GetClientRect,
    GetWindowLongPtrW, IDC_HAND, KillTimer, LoadCursorW, LoadIconW, MAKEINTRESOURCEW,
    MessageBeep, MessageBoxW, OpenClipboard, SendMessageW, SetClipboardData, SetCursor,
    SetTimer, SetWindowLongPtrW, CF_UNICODETEXT, GWL_STYLE, ICON_BIG,
    ICON_SMALL, MB_ICONASTERISK, MB_ICONHAND, SW_SHOWNORMAL, WM_CTLCOLORSTATIC,
    WM_DROPFILES, WM_ERASEBKGND, WM_LBUTTONUP, WM_SETCURSOR, WM_SETICON, WM_TIMER,
    WS_MAXIMIZEBOX, WS_THICKFRAME,
};
use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

mod sig;
use sig::{check_signature, SignatureStatus};

mod update;

const NEWUPDATE_URL: &str =
    "https://raw.githubusercontent.com/rouhalamin/checksum/main/newupdate.txt";

/// Drives the Check Update overlay. `Idle` means the main UI is showing;
/// every other variant is a step of the update flow, always entered and
/// left through `sync_update_timer` so the WM_TIMER polling loop is only
/// ever running when a background step (Checking/Downloading/Verifying)
/// actually needs it.
#[derive(Clone, PartialEq)]
enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Verifying,
    Ready,
    Failed(String),
}

impl Default for UpdateState {
    fn default() -> Self {
        UpdateState::Idle
    }
}

// ---------------------------------------------------------------------------
// Small local (fully offline) heuristic: guess an official source domain
// from the file name only. No network calls are made anywhere in this app.
// The result is deliberately worded as a *guess*, never a certainty — see
// `guess_source` callers, which always prefix it with "Likely Source:".
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
    "Unknown"
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

fn format_speed(bytes_per_sec: f64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec.max(0.0) as u64))
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
// Hash algorithm support
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Algorithm {
    Sha256,
    Sha384,
    Sha512,
    Sha1,
    Md5,
    Blake2b,
    Blake3,
}

// Explicit (not derived) so the default is guaranteed to be SHA-256 —
// required because `CheckSumApp` derives `Default` and holds a
// `RefCell<Algorithm>` field.
impl Default for Algorithm {
    fn default() -> Self {
        Algorithm::Sha256
    }
}

impl Algorithm {
    const ALL: [Algorithm; 7] = [
        Algorithm::Sha256,
        Algorithm::Sha384,
        Algorithm::Sha512,
        Algorithm::Sha1,
        Algorithm::Md5,
        Algorithm::Blake2b,
        Algorithm::Blake3,
    ];

    fn label(&self) -> &'static str {
        match self {
            Algorithm::Sha256 => "SHA-256",
            Algorithm::Sha384 => "SHA-384",
            Algorithm::Sha512 => "SHA-512",
            Algorithm::Sha1 => "SHA-1",
            Algorithm::Md5 => "MD5",
            Algorithm::Blake2b => "BLAKE2b-512",
            Algorithm::Blake3 => "BLAKE3",
        }
    }

    fn from_index(i: usize) -> Algorithm {
        Algorithm::ALL.get(i).copied().unwrap_or(Algorithm::Sha256)
    }

    fn index(&self) -> usize {
        Algorithm::ALL.iter().position(|a| a == self).unwrap_or(0)
    }

    /// Expected hex-digest length for this algorithm (bytes * 2).
    fn hex_len(&self) -> usize {
        match self {
            Algorithm::Sha256 => 64,
            Algorithm::Sha384 => 96,
            Algorithm::Sha512 => 128,
            Algorithm::Sha1 => 40,
            Algorithm::Md5 => 32,
            Algorithm::Blake2b => 128,
            Algorithm::Blake3 => 64,
        }
    }

    /// Guess an algorithm from a checksum sidecar file's extension.
    fn from_extension(ext: &str) -> Option<Algorithm> {
        match ext.to_lowercase().as_str() {
            "sha256" => Some(Algorithm::Sha256),
            "sha384" => Some(Algorithm::Sha384),
            "sha512" => Some(Algorithm::Sha512),
            "sha1" => Some(Algorithm::Sha1),
            "md5" => Some(Algorithm::Md5),
            "b2" | "blake2" | "blake2b" => Some(Algorithm::Blake2b),
            "blake3" => Some(Algorithm::Blake3),
            _ => None,
        }
    }
}

/// Wraps whichever concrete hasher is active. Keeping this as a plain enum
/// (instead of `Box<dyn ...>`) avoids dynamic dispatch and keeps every
/// variant's state inline, which matters here since `update()` is called
/// once per 4-8 MiB chunk for potentially very large files.
enum HasherState {
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
    Sha1(Sha1),
    Md5(Md5),
    Blake2b(Blake2b512),
    Blake3(blake3::Hasher),
}

impl HasherState {
    fn new(algo: Algorithm) -> Self {
        match algo {
            Algorithm::Sha256 => HasherState::Sha256(Sha256::new()),
            Algorithm::Sha384 => HasherState::Sha384(Sha384::new()),
            Algorithm::Sha512 => HasherState::Sha512(Sha512::new()),
            Algorithm::Sha1 => HasherState::Sha1(Sha1::new()),
            Algorithm::Md5 => HasherState::Md5(Md5::new()),
            Algorithm::Blake2b => HasherState::Blake2b(Blake2b512::new()),
            Algorithm::Blake3 => HasherState::Blake3(blake3::Hasher::new()),
        }
    }

    fn update(&mut self, data: &[u8]) {
        match self {
            HasherState::Sha256(h) => Digest::update(h, data),
            HasherState::Sha384(h) => Digest::update(h, data),
            HasherState::Sha512(h) => Digest::update(h, data),
            HasherState::Sha1(h) => Digest::update(h, data),
            HasherState::Md5(h) => Digest::update(h, data),
            HasherState::Blake2b(h) => Digest::update(h, data),
            HasherState::Blake3(h) => {
                h.update(data);
            }
        }
    }

    fn finalize_hex(self) -> String {
        match self {
            HasherState::Sha256(h) => hex::encode(h.finalize()),
            HasherState::Sha384(h) => hex::encode(h.finalize()),
            HasherState::Sha512(h) => hex::encode(h.finalize()),
            HasherState::Sha1(h) => hex::encode(h.finalize()),
            HasherState::Md5(h) => hex::encode(h.finalize()),
            HasherState::Blake2b(h) => hex::encode(h.finalize()),
            HasherState::Blake3(h) => h.finalize().to_hex().to_string(),
        }
    }
}

/// Validate a user-entered expected hash against the selected algorithm:
/// trims whitespace, lower-cases it, and checks it's the right length and
/// made only of hex digits. Returns the normalized hash or a human-readable
/// error message.
fn normalize_expected_hash(raw: &str, algo: Algorithm) -> Result<String, String> {
    let trimmed = raw.trim().to_lowercase();
    if trimmed.is_empty() {
        return Err("Please enter the expected checksum.".to_string());
    }
    if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "Invalid {} checksum — it must contain only hex digits (0-9, a-f).",
            algo.label()
        ));
    }
    if trimmed.len() != algo.hex_len() {
        return Err(format!(
            "Invalid {} checksum length — expected {} hex characters, got {}.",
            algo.label(),
            algo.hex_len(),
            trimmed.len()
        ));
    }
    Ok(trimmed)
}

// ---------------------------------------------------------------------------
// Checksum sidecar file parsing (.sha256 / .sha512 / .sha1 / .md5 / etc.)
// Supports the two checksum-file conventions in common use:
//   1. GNU coreutils style:   <hex>  <filename>          (or "<hex> *<filename>")
//   2. BSD/OpenSSL style:     SHA256(<filename>) = <hex>
// Returns (hash, optional referenced filename) for the first valid line.
// ---------------------------------------------------------------------------
fn parse_checksum_file(path: &Path) -> Result<(String, Option<String>), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Could not read checksum file: {}", e))?;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // BSD/OpenSSL style: ALGO(filename) = hash
        if let Some(eq_pos) = line.find('=') {
            if let (Some(open), Some(close)) = (line.find('('), line.find(')')) {
                if open < close && close < eq_pos {
                    let filename = line[open + 1..close].trim().to_string();
                    let hash = line[eq_pos + 1..].trim().to_lowercase();
                    if !hash.is_empty() && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Ok((hash, Some(filename)));
                    }
                }
            }
        }

        // GNU coreutils style: hash<space(s)>[*]filename
        let mut parts = line.splitn(2, char::is_whitespace);
        if let Some(first) = parts.next() {
            let candidate = first.trim().to_lowercase();
            if !candidate.is_empty() && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
                let filename = parts
                    .next()
                    .map(|s| s.trim().trim_start_matches('*').to_string())
                    .filter(|s| !s.is_empty());
                return Ok((candidate, filename));
            }
        }
    }

    Err("No valid checksum entry found in this file.".to_string())
}

const CHECKSUM_FILE_EXTS: &[&str] = &[
    "sha256", "sha384", "sha512", "sha1", "md5", "b2", "blake2", "blake2b", "blake3", "checksum",
    "sum", "sfv",
];

fn is_checksum_sidecar(path: &Path) -> bool {
    path.extension()
        .map(|e| CHECKSUM_FILE_EXTS.contains(&e.to_string_lossy().to_lowercase().as_str()))
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Clipboard (Unicode text) — small, self-contained, no external crate.
// ---------------------------------------------------------------------------
fn copy_text_to_clipboard(hwnd: HWND, text: &str) -> bool {
    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return false;
        }
        let ok = (|| {
            if EmptyClipboard() == 0 {
                return false;
            }
            let wide = to_wide(text);
            let bytes = wide.len() * std::mem::size_of::<u16>();
            let hmem: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if hmem.is_null() {
                return false;
            }
            let ptr = GlobalLock(hmem) as *mut u16;
            if ptr.is_null() {
                return false;
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
            GlobalUnlock(hmem);
            !SetClipboardData(CF_UNICODETEXT, hmem as _).is_null()
        })();
        CloseClipboard();
        ok
    }
}

const CHUNK_SIZE: usize = 8 * 1024 * 1024; // 8 MiB: see build notes — sweet spot
                                            // for large-file throughput on both SSD
                                            // and spinning/network drives without
                                            // bloating peak RAM use noticeably.
const PROGRESS_TIMER_ID: UINT_PTR = 1;
const UPDATE_TIMER_ID: UINT_PTR = 2;
const PBM_SETBARCOLOR: u32 = 0x0409; // WM_USER + 9
const MAIN_ICON_RESOURCE_ID: u16 = 1; // must match res.set_icon_with_id(..., "1") in build.rs

const GREEN: (u8, u8, u8) = (0, 150, 0);
const RED: (u8, u8, u8) = (196, 0, 0);
const LINK_BLUE: (u8, u8, u8) = (0, 90, 200);
const GREY: (u8, u8, u8) = (110, 110, 110);
// Softened, slightly-off-white main canvas (default Win32 static/window
// background is pure white on modern Windows, which the reference
// screenshot showed as too stark) and a marginally darker grey for the
// dedicated footer band, so the footer visually separates from the main
// content without looking like a heavy "card" or web-style panel.
const CANVAS_BG: (u8, u8, u8) = (246, 246, 248);
const FOOTER_BG: (u8, u8, u8) = (232, 232, 236);
const FOOTER_TOP_Y: i32 = 388;

// ---------------------------------------------------------------------------
// UI definition
// ---------------------------------------------------------------------------
#[derive(Default, NwgUi)]
pub struct CheckSumApp {
    #[nwg_control(
        size: (560, 470),
        position: (300, 220),
        title: "CheckSum",
        flags: "WINDOW|VISIBLE"
    )]
    #[nwg_events( OnWindowClose: [CheckSumApp::exit], OnInit: [CheckSumApp::on_init] )]
    window: nwg::Window,

    // --- Section 1: algorithm -------------------------------------------
    #[nwg_control(text: "Hash Algorithm:", position: (16, 14), size: (120, 20))]
    lbl_algo: nwg::Label,

    // NOTE (build fix): a Win32 CBS_DROPDOWNLIST control's requested height
    // sets how much room Windows reserves for the *open* drop-down list —
    // the closed box always renders at a height driven by the font metrics
    // regardless of this number. A height of only 24 (closed-box height)
    // left no room for the list at all, which is why clicking the combo
    // previously did nothing visible. 300 gives the list room to open while
    // the closed control still renders at its normal compact height.
    #[nwg_control(collection: vec!["SHA-256","SHA-384","SHA-512","SHA-1","MD5","BLAKE2b-512","BLAKE3"], selected_index: Some(0), position: (140, 11), size: (150, 300))]
    #[nwg_events( OnComboxBoxSelection: [CheckSumApp::on_algo_changed] )]
    algo_combo: nwg::ComboBox<&'static str>,

    #[nwg_control(text: "Verify", position: (310, 13), size: (60, 20))]
    #[nwg_events( OnButtonClick: [CheckSumApp::on_verify_clicked] )]
    mode_verify: nwg::RadioButton,

    #[nwg_control(text: "Calculate", position: (375, 13), size: (80, 20))]
    #[nwg_events( OnButtonClick: [CheckSumApp::on_calculate_clicked] )]
    mode_calculate: nwg::RadioButton,

    // --- Section 2: expected hash (Verify mode only) ---------------------
    #[nwg_control(text: "Expected Hash:", position: (16, 46), size: (120, 18))]
    lbl_hash: nwg::Label,

    #[nwg_control(position: (16, 65), size: (528, 24), flags: "VISIBLE", placeholder_text: Some("Paste the official checksum here"))]
    hash_input: nwg::TextInput,

    // --- Section 3: file ---------------------------------------------------
    #[nwg_control(text: "File (or drag & drop it anywhere on this window):", position: (16, 98), size: (400, 18))]
    lbl_file: nwg::Label,

    #[nwg_control(position: (16, 117), size: (398, 24), flags: "VISIBLE", placeholder_text: Some("e.g. C:\\Downloads\\example.iso"))]
    file_input: nwg::TextInput,

    #[nwg_control(text: "Browse...", position: (422, 116), size: (122, 26))]
    #[nwg_events( OnButtonClick: [CheckSumApp::browse] )]
    browse_btn: nwg::Button,

    // --- Section 4: action + progress --------------------------------------
    #[nwg_control(text: "Start", position: (16, 152), size: (100, 28))]
    #[nwg_events( OnButtonClick: [CheckSumApp::on_action_click] )]
    action_btn: nwg::Button,

    #[nwg_control(range: 0..100, position: (124, 154), size: (420, 24))]
    progress: nwg::ProgressBar,

    // --- Section 5: live information grid ----------------------------------
    #[nwg_control(text: "Size: —", position: (16, 190), size: (170, 18))]
    info_size_label: nwg::Label,

    #[nwg_control(text: "Checked: —", position: (196, 190), size: (170, 18))]
    info_checked_label: nwg::Label,

    #[nwg_control(text: "Speed: —", position: (376, 190), size: (170, 18))]
    info_speed_label: nwg::Label,

    #[nwg_control(text: "Time: —", position: (16, 212), size: (170, 18))]
    info_time_label: nwg::Label,

    #[nwg_control(text: "Likely Source: —", position: (196, 212), size: (350, 18))]
    info_source_label: nwg::Label,

    // --- Section 6: result --------------------------------------------------
    #[nwg_control(text: "", position: (16, 242), size: (528, 40))]
    result_label: nwg::Label,

    #[nwg_control(text: "", position: (16, 284), size: (528, 44))]
    verification_info_label: nwg::Label,

    #[nwg_control(text: "Computed Hash:", position: (16, 332), size: (120, 18))]
    lbl_computed: nwg::Label,

    #[nwg_control(position: (16, 351), size: (450, 24), readonly: true, flags: "VISIBLE")]
    computed_hash_display: nwg::TextInput,

    #[nwg_control(text: "Copy", position: (472, 350), size: (72, 26))]
    #[nwg_events( OnButtonClick: [CheckSumApp::copy_hash] )]
    copy_btn: nwg::Button,

    // --- Update overlay: occupies the same content area as the main UI,
    // shown/hidden as a whole via set_update_ui_visible / set_main_ui_visible.
    // update_changes_label and update_status_label share one rect since
    // they're never shown at the same time (What's New vs. live download
    // stats belong to different steps of the flow).
    #[nwg_control(text: "", position: (16, 26), size: (528, 24))]
    update_title_label: nwg::Label,

    #[nwg_control(range: 0..100, position: (180, 60), size: (200, 20))]
    update_spinner: nwg::ProgressBar,

    #[nwg_control(text: "", position: (16, 96), size: (528, 40))]
    update_detail_label: nwg::Label,

    #[nwg_control(text: "", position: (16, 144), size: (528, 130))]
    update_changes_label: nwg::Label,

    #[nwg_control(text: "", position: (16, 144), size: (528, 130))]
    update_status_label: nwg::Label,

    #[nwg_control(range: 0..100, position: (16, 284), size: (528, 22))]
    update_progress: nwg::ProgressBar,

    #[nwg_control(text: "Update", position: (300, 340), size: (110, 30))]
    #[nwg_events( OnButtonClick: [CheckSumApp::on_update_primary_click] )]
    update_primary_btn: nwg::Button,

    #[nwg_control(text: "Cancel", position: (420, 340), size: (110, 30))]
    #[nwg_events( OnButtonClick: [CheckSumApp::on_update_secondary_click] )]
    update_secondary_btn: nwg::Button,

    // --- Footer (distinct grey band, painted in the window's WM_ERASEBKGND
    // handler — see install_window_hook) --------------------------------
    #[nwg_control(text: "by Rohulamin Erfani", position: (16, 396), size: (180, 16))]
    credit_label: nwg::Label,

    #[nwg_control(text: "", position: (330, 396), size: (48, 16))]
    version_label: nwg::Label,

    #[nwg_control(text: "Check Update", position: (382, 396), size: (100, 16))]
    check_update_label: nwg::Label,

    #[nwg_control(text: "Github", position: (16, 418), size: (52, 16))]
    link_github: nwg::Label,

    #[nwg_control(text: "CheckSum-site", position: (80, 418), size: (92, 16))]
    link_site: nwg::Label,

    #[nwg_control(text: "Support via Crypto", position: (184, 418), size: (128, 16))]
    link_donate: nwg::Label,

    // --- shared state between the worker thread and the GUI thread -------
    progress_counter: RefCell<Option<Arc<AtomicU64>>>,
    total_bytes: RefCell<u64>,
    worker_done: RefCell<Option<Arc<AtomicBool>>>,
    cancel_flag: RefCell<Option<Arc<AtomicBool>>>,
    was_cancelled: RefCell<Option<Arc<AtomicBool>>>,
    io_error: Arc<Mutex<Option<String>>>,
    computed_hash: Arc<Mutex<Option<String>>>,
    result_state: RefCell<i8>, // 0 = neutral, 1 = success (green), 2 = error (red)
    hashing_active: RefCell<bool>,
    start_time: RefCell<Option<Instant>>,
    last_poll: RefCell<Option<(Instant, u64)>>, // for instantaneous speed
    current_source_url: RefCell<String>,        // empty when there's nothing to open
    current_algorithm: RefCell<Algorithm>,
    is_verify_mode: RefCell<bool>,
    // Written from a background thread, so this must be a thread-safe
    // container (unlike the RefCell fields above, which are only ever
    // touched from the GUI thread) — never dereference `self` itself from
    // a spawned thread.
    signature_status: Arc<Mutex<Option<SignatureStatus>>>,

    credit_font: RefCell<Option<nwg::Font>>,
    info_font: RefCell<Option<nwg::Font>>,
    result_font: RefCell<Option<nwg::Font>>,
    mono_font: RefCell<Option<nwg::Font>>,

    // --- Update flow state -------------------------------------------------
    update_state: RefCell<UpdateState>,
    update_info: RefCell<Option<update::UpdateInfo>>,
    update_check_result: Arc<Mutex<Option<Result<update::UpdateInfo, String>>>>,
    update_download_result: Arc<Mutex<Option<Result<(), String>>>>,
    update_verify_result: Arc<Mutex<Option<Result<(String, SignatureStatus), String>>>>,
    update_downloaded: RefCell<Option<Arc<AtomicU64>>>,
    update_total: RefCell<Option<Arc<AtomicU64>>>,
    update_done: RefCell<Option<Arc<AtomicBool>>>,
    update_start_time: RefCell<Option<Instant>>,
    update_temp_path: RefCell<Option<PathBuf>>,
    update_signature_status: RefCell<Option<SignatureStatus>>,
}

impl CheckSumApp {
    fn on_init(&self) {
        *self.current_algorithm.borrow_mut() = Algorithm::Sha256;
        *self.is_verify_mode.borrow_mut() = true;
        self.mode_verify.set_check_state(nwg::RadioButtonState::Checked);

        // Lock the window to a fixed size (no resize border, no maximize box)
        // so it behaves like a classic Windows dialog box.
        unsafe {
            if let Some(hwnd) = self.window.handle.hwnd() {
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let fixed_style = style & !(WS_THICKFRAME as isize) & !(WS_MAXIMIZEBOX as isize);
                SetWindowLongPtrW(hwnd, GWL_STYLE, fixed_style);

                // Accept OS-level file drops (Explorer drag & drop) anywhere
                // on the window.
                DragAcceptFiles(hwnd, 1);

                // Force the title-bar / taskbar / Alt-Tab icon to the one
                // embedded into this exe by compiler.py, independently of
                // whatever Explorer's icon cache happens to be showing.
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

        // Subtle, small grey signature / footer links — same understated
        // look as the placeholder text.
        let mut credit_font = nwg::Font::default();
        if nwg::Font::builder()
            .family("Segoe UI")
            .size(13)
            .build(&mut credit_font)
            .is_ok()
        {
            self.credit_label.set_font(Some(&credit_font));
            self.version_label.set_font(Some(&credit_font));
            self.check_update_label.set_font(Some(&credit_font));
            self.link_github.set_font(Some(&credit_font));
            self.link_site.set_font(Some(&credit_font));
            self.link_donate.set_font(Some(&credit_font));
        }
        *self.credit_font.borrow_mut() = Some(credit_font);

        // Slightly bolder font for the live info fields.
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
            self.info_speed_label.set_font(Some(&info_font));
            self.info_time_label.set_font(Some(&info_font));
            self.info_source_label.set_font(Some(&info_font));
            self.verification_info_label.set_font(Some(&info_font));
            self.update_detail_label.set_font(Some(&info_font));
            self.update_changes_label.set_font(Some(&info_font));
            self.update_status_label.set_font(Some(&info_font));
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
            self.update_title_label.set_font(Some(&result_font));
        }
        *self.result_font.borrow_mut() = Some(result_font);

        // Monospace for the computed-hash box so long hex strings stay
        // legible and evenly spaced.
        let mut mono_font = nwg::Font::default();
        if nwg::Font::builder()
            .family("Consolas")
            .size(14)
            .build(&mut mono_font)
            .is_ok()
        {
            self.computed_hash_display.set_font(Some(&mono_font));
        }
        *self.mono_font.borrow_mut() = Some(mono_font);

        self.version_label
            .set_text(&format!("V {}", display_version(env!("CARGO_PKG_VERSION"))));

        self.result_label.set_text("");
        self.verification_info_label.set_text("");
        self.progress.set_pos(0);
        self.install_window_hook();
        self.install_link_hook(&self.info_source_label, 0x1_0001);
        self.install_static_link(&self.link_github, "https://github.com/rouhalamin/checksum", 0x1_0002);
        self.install_static_link(&self.link_site, "https://checksumapp.netlify.app/", 0x1_0003);
        self.install_static_link(&self.link_donate, "https://checksumapp.netlify.app/#donate", 0x1_0004);
        self.install_click_hook(&self.check_update_label, 0x1_0005, CheckSumApp::on_check_update_click);
        self.update_mode_visibility();

        // The update overlay starts fully hidden — every one of its
        // controls is created VISIBLE by default like any other nwg
        // control, so this must be explicit rather than assumed.
        self.set_update_ui_visible(false);
    }

    fn on_algo_changed(&self) {
        let idx = self.algo_combo.selection().unwrap_or(0);
        *self.current_algorithm.borrow_mut() = Algorithm::from_index(idx);
    }

    /// NWG radio buttons are only auto-grouped when they're declared with
    /// no other control between them AND share the WS_GROUP style, which
    /// isn't reliably exposed through the declarative flags list — so
    /// exclusivity between these two is enforced explicitly here instead of
    /// relying on native grouping behavior.
    fn on_verify_clicked(&self) {
        self.mode_verify.set_check_state(nwg::RadioButtonState::Checked);
        self.mode_calculate.set_check_state(nwg::RadioButtonState::Unchecked);
        *self.is_verify_mode.borrow_mut() = true;
        self.update_mode_visibility();
    }

    fn on_calculate_clicked(&self) {
        self.mode_calculate.set_check_state(nwg::RadioButtonState::Checked);
        self.mode_verify.set_check_state(nwg::RadioButtonState::Unchecked);
        *self.is_verify_mode.borrow_mut() = false;
        self.update_mode_visibility();
    }

    fn update_mode_visibility(&self) {
        let verify = *self.is_verify_mode.borrow();
        self.lbl_hash.set_visible(verify);
        self.hash_input.set_visible(verify);
        self.action_btn
            .set_text(if verify { "Verify" } else { "Calculate" });
    }

    // ------------------------------------------------------------------
    // Update flow
    // ------------------------------------------------------------------

    fn set_main_ui_visible(&self, visible: bool) {
        self.lbl_algo.set_visible(visible);
        self.algo_combo.set_visible(visible);
        self.mode_verify.set_visible(visible);
        self.mode_calculate.set_visible(visible);
        self.lbl_file.set_visible(visible);
        self.file_input.set_visible(visible);
        self.browse_btn.set_visible(visible);
        self.action_btn.set_visible(visible);
        self.progress.set_visible(visible);
        self.info_size_label.set_visible(visible);
        self.info_checked_label.set_visible(visible);
        self.info_speed_label.set_visible(visible);
        self.info_time_label.set_visible(visible);
        self.info_source_label.set_visible(visible);
        self.result_label.set_visible(visible);
        self.verification_info_label.set_visible(visible);
        self.lbl_computed.set_visible(visible);
        self.computed_hash_display.set_visible(visible);
        self.copy_btn.set_visible(visible);
        if visible {
            // Expected Hash's visibility depends on Verify/Calculate, not
            // just "is the main UI showing" — restore it properly instead
            // of assuming it should always reappear.
            self.update_mode_visibility();
        } else {
            self.lbl_hash.set_visible(false);
            self.hash_input.set_visible(false);
        }
    }

    fn set_update_ui_visible(&self, visible: bool) {
        self.update_title_label.set_visible(visible);
        self.update_detail_label.set_visible(visible);
        if !visible {
            // Sub-parts each phase turns on individually; always safe to
            // force them off together when leaving the overlay entirely.
            self.update_spinner.set_visible(false);
            self.update_changes_label.set_visible(false);
            self.update_status_label.set_visible(false);
            self.update_progress.set_visible(false);
            self.update_primary_btn.set_visible(false);
            self.update_secondary_btn.set_visible(false);
        }
    }

    const PBS_MARQUEE: u32 = 0x08;
    const PBM_SETMARQUEE: u32 = 0x040A; // WM_USER + 10

    fn start_marquee(&self) {
        if let Some(hwnd) = self.update_spinner.handle.hwnd() {
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                SetWindowLongPtrW(hwnd, GWL_STYLE, style | Self::PBS_MARQUEE as isize);
                SendMessageW(hwnd, Self::PBM_SETMARQUEE, 1, 30);
            }
        }
        self.update_spinner.set_visible(true);
    }

    fn stop_marquee(&self) {
        if let Some(hwnd) = self.update_spinner.handle.hwnd() {
            unsafe {
                SendMessageW(hwnd, Self::PBM_SETMARQUEE, 0, 0);
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                SetWindowLongPtrW(hwnd, GWL_STYLE, style & !(Self::PBS_MARQUEE as isize));
            }
        }
        self.update_spinner.set_visible(false);
    }

    /// Keeps `UPDATE_TIMER_ID` running exactly while the current state is
    /// one that a background thread is working on (Checking / Downloading /
    /// Verifying) and stopped the rest of the time, so nothing polls a dead
    /// job forever and nothing needs to remember to arm/disarm by hand at
    /// every call site.
    fn sync_update_timer(&self) {
        let needs_timer = matches!(
            *self.update_state.borrow(),
            UpdateState::Checking | UpdateState::Downloading | UpdateState::Verifying
        );
        unsafe {
            if let Some(hwnd) = self.window.handle.hwnd() {
                if needs_timer {
                    SetTimer(hwnd, UPDATE_TIMER_ID, 150, None);
                } else {
                    KillTimer(hwnd, UPDATE_TIMER_ID);
                }
            }
        }
    }

    fn on_check_update_click(&self) {
        // Don't let a Check Update click interrupt a hash job that's mid-run.
        if *self.hashing_active.borrow() {
            return;
        }

        self.set_main_ui_visible(false);
        self.set_update_ui_visible(true);
        *self.update_state.borrow_mut() = UpdateState::Checking;
        *self.update_info.borrow_mut() = None;
        *self.update_check_result.lock().unwrap() = None;

        self.update_title_label.set_text("Checking for Update...");
        self.update_detail_label.set_text("");
        self.start_marquee();

        let result_slot = self.update_check_result.clone();
        thread::spawn(move || {
            let outcome = (|| -> Result<update::UpdateInfo, String> {
                let text = update::fetch_text(NEWUPDATE_URL)?;
                update::parse_update_info(&text)
                    .ok_or_else(|| "newupdate.txt could not be parsed.".to_string())
            })();
            *result_slot.lock().unwrap() = Some(outcome);
        });

        self.sync_update_timer();
    }

    /// Runs on the GUI thread via WM_TIMER while `update_state` is one of
    /// the three background-work variants.
    fn poll_update(&self) {
        let state = self.update_state.borrow().clone();
        match state {
            UpdateState::Checking => self.poll_update_checking(),
            UpdateState::Downloading => self.poll_update_downloading(),
            UpdateState::Verifying => self.poll_update_verifying(),
            _ => {}
        }
    }

    fn poll_update_checking(&self) {
        let outcome = match self.update_check_result.lock().unwrap().take() {
            Some(o) => o,
            None => return,
        };
        self.stop_marquee();

        match outcome {
            Ok(info) => {
                let local_is_older =
                    update::compare_versions(env!("CARGO_PKG_VERSION"), &info.version)
                        == std::cmp::Ordering::Less;
                if local_is_older {
                    self.update_title_label.set_text("New update available");
                    self.update_detail_label.set_text(&format!(
                        "Version {}\r\nSize Update: {}",
                        info.version, info.size_display
                    ));
                    let bullets = info
                        .changes
                        .iter()
                        .map(|c| format!("\u{2022} {}", c))
                        .collect::<Vec<_>>()
                        .join("\r\n");
                    self.update_changes_label
                        .set_text(&format!("What's New\r\n{}", bullets));
                    self.update_changes_label.set_visible(true);
                    self.update_primary_btn.set_text("Update");
                    self.update_primary_btn.set_visible(true);
                    self.update_secondary_btn.set_text("Cancel");
                    self.update_secondary_btn.set_visible(true);
                    *self.update_info.borrow_mut() = Some(info);
                    *self.update_state.borrow_mut() = UpdateState::Available;
                } else {
                    self.update_title_label.set_text("You are up to date");
                    self.update_detail_label.set_text(&format!(
                        "Version {}",
                        display_version(env!("CARGO_PKG_VERSION"))
                    ));
                    self.update_primary_btn.set_text("Back");
                    self.update_primary_btn.set_visible(true);
                    *self.update_state.borrow_mut() = UpdateState::UpToDate;
                }
            }
            Err(msg) => self.fail_update(&format!("Could not check for updates: {}", msg)),
        }
        self.sync_update_timer();
    }

    fn begin_update_download(&self) {
        let info = match self.update_info.borrow().clone() {
            Some(i) => i,
            None => return,
        };

        *self.update_state.borrow_mut() = UpdateState::Downloading;
        self.update_title_label.set_text("Downloading Update...");
        self.update_detail_label
            .set_text("Please do not close CheckSum while the update is downloading.");
        self.update_changes_label.set_visible(false);
        self.update_primary_btn.set_visible(false);
        self.update_secondary_btn.set_visible(false);
        self.update_progress.set_pos(0);
        self.update_progress.set_visible(true);
        self.update_status_label.set_visible(true);

        let downloaded = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(0));
        // Not currently wired to a Cancel control on the download screen
        // (the spec only calls for Cancel on the pre-download "New update
        // available" screen) — kept so `update::download_file`'s signature
        // stays reusable if a Cancel button is added here later.
        let cancel = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));

        *self.update_downloaded.borrow_mut() = Some(downloaded.clone());
        *self.update_total.borrow_mut() = Some(total.clone());
        *self.update_done.borrow_mut() = Some(done.clone());
        *self.update_start_time.borrow_mut() = Some(Instant::now());
        *self.update_download_result.lock().unwrap() = None;

        let temp_path = std::env::temp_dir().join(format!("CheckSum-update-{}.exe", info.version));
        *self.update_temp_path.borrow_mut() = Some(temp_path.clone());

        let result_slot = self.update_download_result.clone();
        let url = info.download_url.clone();
        thread::spawn(move || {
            let outcome = update::download_file(&url, &temp_path, downloaded, total, cancel);
            *result_slot.lock().unwrap() = Some(outcome);
            done.store(true, Ordering::Relaxed);
        });

        self.sync_update_timer();
    }

    fn poll_update_downloading(&self) {
        let downloaded = self
            .update_downloaded
            .borrow()
            .as_ref()
            .map(|a| a.load(Ordering::Relaxed))
            .unwrap_or(0);
        let total = self
            .update_total
            .borrow()
            .as_ref()
            .map(|a| a.load(Ordering::Relaxed))
            .unwrap_or(0);

        if total > 0 {
            let pct = ((downloaded as f64 / total as f64) * 100.0).min(100.0) as u32;
            self.update_progress.set_pos(pct);
        }

        let elapsed = self
            .update_start_time
            .borrow()
            .as_ref()
            .map(|t| t.elapsed())
            .unwrap_or_default();
        let secs = elapsed.as_secs_f64();
        let speed = if secs > 0.0 { downloaded as f64 / secs } else { 0.0 };
        let eta = if speed > 0.0 && total > downloaded {
            format!("{:.0}s", (total - downloaded) as f64 / speed)
        } else {
            "\u{2014}".to_string()
        };

        self.update_status_label.set_text(&format!(
            "Downloaded: {}\r\nTotal: {}\r\nTime: {}\r\nSpeed: {}\r\nEstimated remaining: {}",
            format_bytes(downloaded),
            if total > 0 { format_bytes(total) } else { "\u{2014}".to_string() },
            format_elapsed(elapsed),
            format_speed(speed),
            eta
        ));

        let done = self
            .update_done
            .borrow()
            .as_ref()
            .map(|d| d.load(Ordering::Relaxed))
            .unwrap_or(false);
        if !done {
            return;
        }

        match self.update_download_result.lock().unwrap().take() {
            Some(Ok(())) => self.begin_update_verify(),
            Some(Err(msg)) => self.fail_update(&format!("Update download failed: {}", msg)),
            None => {}
        }
    }

    fn begin_update_verify(&self) {
        *self.update_state.borrow_mut() = UpdateState::Verifying;
        self.update_title_label.set_text("Verifying update...");
        self.update_status_label
            .set_text("Checking SHA-256 and digital signature.");
        self.update_progress.set_visible(false);

        let path = match self.update_temp_path.borrow().clone() {
            Some(p) => p,
            None => return,
        };
        let done = Arc::new(AtomicBool::new(false));
        *self.update_done.borrow_mut() = Some(done.clone());
        *self.update_verify_result.lock().unwrap() = None;

        let result_slot = self.update_verify_result.clone();
        thread::spawn(move || {
            let outcome = (|| -> Result<(String, SignatureStatus), String> {
                let mut file = File::open(&path)
                    .map_err(|e| format!("Could not open the downloaded file: {}", e))?;
                let mut hasher = HasherState::new(Algorithm::Sha256);
                let mut buf = vec![0u8; CHUNK_SIZE];
                loop {
                    let n = file
                        .read(&mut buf)
                        .map_err(|e| format!("Read error while verifying: {}", e))?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buf[..n]);
                }
                let hex_hash = hasher.finalize_hex();
                let sig = check_signature(&path);
                Ok((hex_hash, sig))
            })();
            *result_slot.lock().unwrap() = Some(outcome);
            done.store(true, Ordering::Relaxed);
        });

        self.sync_update_timer();
    }

    fn poll_update_verifying(&self) {
        let done = self
            .update_done
            .borrow()
            .as_ref()
            .map(|d| d.load(Ordering::Relaxed))
            .unwrap_or(false);
        if !done {
            return;
        }

        let outcome = self.update_verify_result.lock().unwrap().take();
        let expected = self
            .update_info
            .borrow()
            .as_ref()
            .map(|i| i.sha256.clone())
            .unwrap_or_default();

        match outcome {
            Some(Ok((hex_hash, sig))) => {
                if !expected.is_empty() && hex_hash == expected {
                    *self.update_signature_status.borrow_mut() = Some(sig);
                    *self.update_state.borrow_mut() = UpdateState::Ready;
                    self.update_title_label.set_text("Update Ready");
                    let version = self
                        .update_info
                        .borrow()
                        .as_ref()
                        .map(|i| i.version.clone())
                        .unwrap_or_default();
                    self.update_status_label.set_text(&format!(
                        "Version {} downloaded and verified (SHA-256 match).",
                        version
                    ));
                    self.update_primary_btn.set_text("Install");
                    self.update_primary_btn.set_visible(true);
                    self.sync_update_timer();
                } else {
                    if let Some(p) = self.update_temp_path.borrow_mut().take() {
                        let _ = std::fs::remove_file(p);
                    }
                    self.fail_update(
                        "Update verification failed — the downloaded file's SHA-256 does not match newupdate.txt.",
                    );
                }
            }
            Some(Err(msg)) => self.fail_update(&msg),
            None => {}
        }
    }

    fn fail_update(&self, msg: &str) {
        *self.update_state.borrow_mut() = UpdateState::Failed(msg.to_string());
        self.update_title_label.set_text("Update failed");
        self.update_detail_label.set_text("");
        self.update_status_label.set_text(msg);
        self.update_status_label.set_visible(true);
        self.update_changes_label.set_visible(false);
        self.update_progress.set_visible(false);
        self.update_primary_btn.set_text("Back");
        self.update_primary_btn.set_visible(true);
        self.update_secondary_btn.set_visible(false);
        self.stop_marquee();
        self.sync_update_timer();
    }

    fn install_update(&self) {
        let path = match self.update_temp_path.borrow().clone() {
            Some(p) => p,
            None => return,
        };
        match update::stage_and_relaunch(&path) {
            Ok(()) => {
                self.sync_update_timer();
                self.exit();
            }
            Err(msg) => self.fail_update(&format!("Update installation failed: {}", msg)),
        }
    }

    fn close_update_screen(&self) {
        self.stop_marquee();
        *self.update_state.borrow_mut() = UpdateState::Idle;
        self.sync_update_timer();
        self.set_update_ui_visible(false);
        self.set_main_ui_visible(true);
    }

    fn on_update_primary_click(&self) {
        let state = self.update_state.borrow().clone();
        match state {
            UpdateState::UpToDate => self.close_update_screen(),
            UpdateState::Failed(_) => self.close_update_screen(),
            UpdateState::Available => self.begin_update_download(),
            UpdateState::Ready => self.install_update(),
            _ => {}
        }
    }

    fn on_update_secondary_click(&self) {
        // Only shown on the pre-download "New update available" screen —
        // closing it there must leave everything exactly as it was
        // (nothing downloaded, nothing changed).
        self.close_update_screen();
    }

    /// Raw hook on the main window: paints result/credit/source/link labels
    /// with the right color and a real background brush (a NULL_BRUSH here
    /// tells Windows "don't erase before repainting", which causes old and
    /// new text to visually overlap between runs), drives progress-bar
    /// polling via WM_TIMER, and handles OS file drops (WM_DROPFILES).
    fn install_window_hook(&self) {
        let self_ptr: *const CheckSumApp = self;

        // Created once and kept for the process lifetime (matches the
        // lifetime of the single main window) — two GDI brush handles is a
        // negligible, one-time cost, reclaimed automatically on exit.
        let canvas_brush = unsafe { CreateSolidBrush(RGB(CANVAS_BG.0, CANVAS_BG.1, CANVAS_BG.2)) };
        let footer_brush = unsafe { CreateSolidBrush(RGB(FOOTER_BG.0, FOOTER_BG.1, FOOTER_BG.2)) };

        // NWG reserves raw-event-handler ids in the 0..=0xFFFF range for its
        // own internal use, so this custom hook must use an id above it.
        //
        // NOTE (bug fix): the previous version of this hook pre-captured a
        // handful of control HWNDs *before* registering the handler and
        // bailed out of installing the hook entirely if any single one of
        // them wasn't available yet. All HWND lookups are now done live,
        // through `self_ptr`, on every message instead — cheap, and it
        // means one missing/late control can never silently disable
        // painting, drag & drop, and progress polling for the whole window.
        let _ = nwg::bind_raw_event_handler(&self.window.handle, 0x1_0000, move |h, msg, wparam, lparam| {
            let this = unsafe { &*self_ptr };

            if msg == WM_ERASEBKGND {
                let hdc = wparam as winapi::shared::windef::HDC;
                let mut rect: RECT = unsafe { std::mem::zeroed() };
                unsafe {
                    GetClientRect(h, &mut rect);
                    FillRect(hdc, &rect, canvas_brush);
                    let mut footer_rect = rect;
                    footer_rect.top = FOOTER_TOP_Y;
                    FillRect(hdc, &footer_rect, footer_brush);
                }
                return Some(1);
            }

            if msg == WM_CTLCOLORSTATIC {
                let ctrl_hwnd = lparam as HWND;
                let hdc = wparam as winapi::shared::windef::HDC;

                let is_ctrl = |ctrl: &nwg::Label| ctrl.handle.hwnd() == Some(ctrl_hwnd);

                // Footer band: text tinted to match, background brush set
                // to the footer color so labels blend into the band
                // instead of showing a mismatched canvas-colored square.
                if is_ctrl(&this.credit_label) || is_ctrl(&this.version_label) {
                    unsafe {
                        SetTextColor(hdc, RGB(GREY.0, GREY.1, GREY.2));
                        SetBkMode(hdc, TRANSPARENT as i32);
                    }
                    return Some(footer_brush as isize);
                }
                if is_ctrl(&this.check_update_label)
                    || is_ctrl(&this.link_github)
                    || is_ctrl(&this.link_site)
                    || is_ctrl(&this.link_donate)
                {
                    unsafe {
                        SetTextColor(hdc, RGB(LINK_BLUE.0, LINK_BLUE.1, LINK_BLUE.2));
                        SetBkMode(hdc, TRANSPARENT as i32);
                    }
                    return Some(footer_brush as isize);
                }

                // Main content area.
                if is_ctrl(&this.result_label) {
                    let state = *this.result_state.borrow();
                    let color = match state {
                        1 => RGB(0, 130, 0),
                        2 => RGB(196, 0, 0),
                        _ => RGB(0, 0, 0),
                    };
                    unsafe {
                        SetTextColor(hdc, color);
                        SetBkMode(hdc, TRANSPARENT as i32);
                    }
                    return Some(canvas_brush as isize);
                }
                if is_ctrl(&this.verification_info_label) {
                    unsafe {
                        SetTextColor(hdc, RGB(GREY.0, GREY.1, GREY.2));
                        SetBkMode(hdc, TRANSPARENT as i32);
                    }
                    return Some(canvas_brush as isize);
                }
                if is_ctrl(&this.info_source_label) {
                    unsafe {
                        SetTextColor(hdc, RGB(LINK_BLUE.0, LINK_BLUE.1, LINK_BLUE.2));
                        SetBkMode(hdc, TRANSPARENT as i32);
                    }
                    return Some(canvas_brush as isize);
                }

                // Every other plain label: keep default black text, but
                // paint the new canvas color behind it instead of a
                // leftover system-white square.
                unsafe {
                    SetBkMode(hdc, TRANSPARENT as i32);
                }
                return Some(canvas_brush as isize);
            }

            if msg == WM_TIMER && wparam == PROGRESS_TIMER_ID {
                this.poll_progress();
                return Some(0);
            }

            if msg == WM_TIMER && wparam == UPDATE_TIMER_ID {
                this.poll_update();
                return Some(0);
            }

            if msg == WM_DROPFILES {
                let hdrop = wparam as HDROP;
                this.handle_drop(hdrop);
                return Some(0);
            }

            None
        });
    }

    /// Reads the first dropped file's path from an HDROP and routes it
    /// through the same logic Browse uses.
    fn handle_drop(&self, hdrop: HDROP) {
        unsafe {
            let mut buf = [0u16; MAX_PATH];
            let len = DragQueryFileW(hdrop, 0, buf.as_mut_ptr(), MAX_PATH as u32);
            if len > 0 {
                let path = String::from_utf16_lossy(&buf[..len as usize]);
                self.load_selected_path(PathBuf::from(path));
            }
            DragFinish(hdrop);
        }
    }

    /// Shared entry point for both Browse and Drag & Drop: if the picked
    /// file looks like a checksum sidecar file (.sha256 etc.), parse it and
    /// populate the Expected Hash field (and switch to Verify mode); if it
    /// also names a referenced file that exists alongside it, populate File
    /// Path too. Otherwise it's treated as the file to hash directly.
    fn load_selected_path(&self, path: PathBuf) {
        if is_checksum_sidecar(&path) {
            match parse_checksum_file(&path) {
                Ok((hash, referenced_name)) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if let Some(algo) = Algorithm::from_extension(ext) {
                            *self.current_algorithm.borrow_mut() = algo;
                            self.algo_combo.set_selection(Some(algo.index()));
                        }
                    }
                    self.hash_input.set_text(&hash);
                    self.mode_verify.set_check_state(nwg::RadioButtonState::Checked);
                    *self.is_verify_mode.borrow_mut() = true;
                    self.update_mode_visibility();

                    if let Some(name) = referenced_name {
                        let candidate = path.parent().map(|p| p.join(&name));
                        if let Some(candidate) = candidate {
                            if candidate.is_file() {
                                self.file_input.set_text(&candidate.to_string_lossy());
                                return;
                            }
                        }
                    }
                    self.result_label.set_text("");
                }
                Err(msg) => {
                    *self.result_state.borrow_mut() = 2;
                    self.result_label
                        .set_text(&format!("Invalid checksum file — {}", msg));
                }
            }
            return;
        }

        self.file_input.set_text(&path.to_string_lossy());
    }

    /// Makes a label behave like a real hyperlink to a fixed URL: hand
    /// cursor on hover, opens the default browser on click.
    fn install_static_link(&self, label: &nwg::Label, url: &'static str, handler_id: UINT_PTR) {
        if label.handle.hwnd().is_none() {
            return;
        }
        let _ = nwg::bind_raw_event_handler(&label.handle, handler_id, move |_h, msg, _w, _l| {
            if msg == WM_SETCURSOR {
                unsafe {
                    SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_HAND));
                }
                return Some(1);
            }
            if msg == WM_LBUTTONUP {
                let op = to_wide("open");
                let url_w = to_wide(url);
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
                return Some(0);
            }
            None
        });
    }

    /// Generic clickable-label hook: hand cursor on hover, calls back into
    /// `self` on click. Used for actions that aren't "open this URL"
    /// (e.g. Check Update).
    fn install_click_hook(&self, label: &nwg::Label, handler_id: UINT_PTR, action: fn(&CheckSumApp)) {
        if label.handle.hwnd().is_none() {
            return;
        }
        let self_ptr: *const CheckSumApp = self;
        let _ = nwg::bind_raw_event_handler(&label.handle, handler_id, move |_h, msg, _w, _l| {
            if msg == WM_SETCURSOR {
                unsafe {
                    SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_HAND));
                }
                return Some(1);
            }
            if msg == WM_LBUTTONUP {
                let this = unsafe { &*self_ptr };
                action(this);
                return Some(0);
            }
            None
        });
    }

    /// Same idea as `install_static_link` but for the Source label, whose
    /// URL changes at runtime (it's set right before each hash run).
    fn install_link_hook(&self, label: &nwg::Label, handler_id: UINT_PTR) {
        let self_ptr: *const CheckSumApp = self;
        let _ = nwg::bind_raw_event_handler(&label.handle, handler_id, move |_h, msg, _wparam, _lparam| {
            if msg == WM_SETCURSOR {
                unsafe {
                    SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_HAND));
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
            .title("Select a file to check")
            .action(nwg::FileDialogAction::Open)
            .build(&mut dialog);

        if built.is_err() {
            return;
        }

        if dialog.run(Some(&self.window)) {
            if let Ok(path) = dialog.get_selected_item() {
                self.load_selected_path(PathBuf::from(path));
            }
        }
    }

    fn copy_hash(&self) {
        let text = self.computed_hash_display.text();
        if text.is_empty() {
            return;
        }
        if let Some(hwnd) = self.window.handle.hwnd() {
            copy_text_to_clipboard(hwnd, &text);
        }
    }

    fn on_action_click(&self) {
        if *self.hashing_active.borrow() {
            if let Some(flag) = self.cancel_flag.borrow().as_ref() {
                flag.store(true, Ordering::Relaxed);
            }
            self.set_progress_color(RED);
        } else {
            self.begin_hashing();
        }
    }

    fn begin_hashing(&self) {
        let verify_mode = *self.is_verify_mode.borrow();
        let algo = *self.current_algorithm.borrow();
        let path = PathBuf::from(self.file_input.text().trim().to_string());

        if !path.is_file() {
            *self.result_state.borrow_mut() = 2;
            self.result_label
                .set_text("Please choose a valid file first.");
            return;
        }

        // Validate the expected-hash format/length up front (in Verify mode)
        // so a bad checksum is reported immediately instead of after a full
        // file read. The actual match/mismatch comparison against the
        // computed hash happens later, in `poll_progress`, re-reading
        // `hash_input` directly — so only the validation `Err` path matters
        // here; there is no separate normalized value to carry forward.
        if verify_mode {
            if let Err(msg) = normalize_expected_hash(&self.hash_input.text(), algo) {
                *self.result_state.borrow_mut() = 2;
                self.result_label.set_text(&msg);
                return;
            }
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
        *self.io_error.lock().unwrap() = None;
        *self.result_state.borrow_mut() = 0;
        *self.start_time.borrow_mut() = Some(Instant::now());
        *self.last_poll.borrow_mut() = Some((Instant::now(), 0));
        *self.hashing_active.borrow_mut() = true;
        *self.signature_status.lock().unwrap() = None;

        // Wipe every field from any previous run before this one starts, so
        // nothing from a prior check can linger or blend with the new result.
        self.progress.set_pos(0);
        self.set_progress_color(GREEN);
        self.action_btn.set_text("Cancel");
        self.browse_btn.set_enabled(false);
        self.hash_input.set_enabled(false);
        self.file_input.set_enabled(false);
        self.algo_combo.set_enabled(false);
        self.mode_verify.set_enabled(false);
        self.mode_calculate.set_enabled(false);
        self.result_label.set_text("");
        self.verification_info_label.set_text("");
        self.computed_hash_display.set_text("");

        let source_guess = guess_source(&path);
        let known_source = source_guess != "Unknown";
        *self.current_source_url.borrow_mut() = if known_source {
            format!("https://{}", source_guess)
        } else {
            String::new()
        };

        self.info_size_label
            .set_text(&format!("Size: {}", format_bytes(file_len)));
        self.info_checked_label
            .set_text(&format!("Checked: {}", format_bytes(0)));
        self.info_time_label.set_text("Time: 0.0s");
        self.info_speed_label.set_text("Speed: —");
        self.info_source_label.set_text(&if known_source {
            format!("Likely Source: {}  (Confidence: Filename)", source_guess)
        } else {
            "Likely Source: Unknown".to_string()
        });

        let computed_hash_slot = self.computed_hash.clone();
        let io_error_slot = self.io_error.clone();

        // `path` itself must stay alive after this closure — the signature
        // check spawned right below also needs it — so the hashing thread
        // only takes a clone.
        let hash_path = path.clone();

        thread::spawn(move || {
            let mut hasher = HasherState::new(algo);
            let mut had_error = false;
            match File::open(&hash_path) {
                Ok(mut file) => {
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
                            Err(e) => {
                                *io_error_slot.lock().unwrap() =
                                    Some(format!("Read error while hashing: {}", e));
                                had_error = true;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    let msg = match e.kind() {
                        std::io::ErrorKind::NotFound => "File not found.".to_string(),
                        std::io::ErrorKind::PermissionDenied => {
                            "Permission denied opening the file.".to_string()
                        }
                        _ => format!("Could not open file: {}", e),
                    };
                    *io_error_slot.lock().unwrap() = Some(msg);
                    had_error = true;
                }
            }

            if !cancelled_flag.load(Ordering::Relaxed) && !had_error {
                // Detect a file that shrank/changed size mid-hash (best-effort).
                if let Ok(meta) = std::fs::metadata(&hash_path) {
                    if meta.len() != file_len {
                        *io_error_slot.lock().unwrap() = Some(
                            "The file changed size while it was being read — result may be unreliable."
                                .to_string(),
                        );
                    }
                }
                let hex_hash = hasher.finalize_hex();
                *computed_hash_slot.lock().unwrap() = Some(hex_hash);
            }
            done_flag.store(true, Ordering::Relaxed);
        });

        // Kick off an (optional, best-effort) Authenticode signature check
        // in parallel — fully independent of the hash result. Only the
        // Arc<Mutex<..>> slot is moved into the thread, never `self`.
        let sig_path = path.clone();
        let sig_slot = self.signature_status.clone();
        thread::spawn(move || {
            let status = check_signature(&sig_path);
            *sig_slot.lock().unwrap() = Some(status);
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
            .set_text(&format!("Checked: {}", format_bytes(done_bytes)));

        if let Some(start) = self.start_time.borrow().as_ref() {
            self.info_time_label
                .set_text(&format!("Time: {}", format_elapsed(start.elapsed())));
        }

        // Instantaneous speed: bytes moved since the last poll / time since
        // the last poll (both ~100ms apart).
        let mut last_poll = self.last_poll.borrow_mut();
        if let Some((last_time, last_bytes)) = *last_poll {
            let dt = last_time.elapsed().as_secs_f64();
            if dt > 0.0 {
                let db = done_bytes.saturating_sub(last_bytes) as f64;
                self.info_speed_label
                    .set_text(&format!("Speed: {}", format_speed(db / dt)));
            }
        }
        *last_poll = Some((Instant::now(), done_bytes));

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
        self.update_mode_visibility();
        self.browse_btn.set_enabled(true);
        self.hash_input.set_enabled(true);
        self.file_input.set_enabled(true);
        self.algo_combo.set_enabled(true);
        self.mode_verify.set_enabled(true);
        self.mode_calculate.set_enabled(true);

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
            self.result_label.set_text("Cancelled.");
            return;
        }

        if let Some(err) = self.io_error.lock().unwrap().clone() {
            self.progress.set_pos(0);
            self.set_progress_color(RED);
            *self.result_state.borrow_mut() = 2;
            self.result_label.set_text(&err);
            return;
        }

        self.progress.set_pos(100);
        self.set_progress_color(GREEN);

        let algo = *self.current_algorithm.borrow();
        let computed = self.computed_hash.lock().unwrap().clone().unwrap_or_default();
        self.computed_hash_display.set_text(&computed);

        // Average speed over the whole run.
        if let Some(start) = self.start_time.borrow().as_ref() {
            let secs = start.elapsed().as_secs_f64().max(0.001);
            let total_done = self
                .progress_counter
                .borrow()
                .as_ref()
                .map(|c| c.load(Ordering::Relaxed))
                .unwrap_or(0);
            self.info_speed_label.set_text(&format!(
                "Speed: {} (avg)",
                format_speed(total_done as f64 / secs)
            ));
        }

        let verify_mode = *self.is_verify_mode.borrow();
        let source_text = self.info_source_label.text();

        if !verify_mode {
            *self.result_state.borrow_mut() = 1;
            self.result_label
                .set_text(&format!("{} computed successfully.", algo.label()));
            self.verification_info_label.set_text(&format!(
                "Verification Information: no expected checksum was provided — this is the file's {} digest only.",
                algo.label()
            ));
            unsafe { MessageBeep(MB_ICONASTERISK) };
            return;
        }

        let expected = self.hash_input.text().trim().to_lowercase();

        if !computed.is_empty() && computed == expected {
            *self.result_state.borrow_mut() = 1;
            self.result_label
                .set_text("✓ Hash matched — File integrity verified");
            self.verification_info_label.set_text(&format!(
                "Verification Information: Hash matched ({}) · Expected hash provided by user · {} · Digital Signature: {}",
                algo.label(),
                source_text,
                self.signature_status_text(),
            ));
            unsafe { MessageBeep(MB_ICONASTERISK) };
        } else {
            *self.result_state.borrow_mut() = 2;
            self.result_label
                .set_text("✕ Hash mismatch — The file does not match the expected checksum");
            self.verification_info_label.set_text(&format!(
                "Verification Information: Hash did NOT match ({}) · {} · Digital Signature: {}",
                algo.label(),
                source_text,
                self.signature_status_text(),
            ));
            unsafe { MessageBeep(MB_ICONHAND) };
        }
    }

    fn signature_status_text(&self) -> String {
        match self.signature_status.lock().unwrap().as_ref() {
            Some(SignatureStatus::Valid(signer)) => {
                if signer.is_empty() {
                    "Valid".to_string()
                } else {
                    format!("Valid ({})", signer)
                }
            }
            Some(SignatureStatus::Invalid) => "Invalid".to_string(),
            Some(SignatureStatus::NotSigned) => "Not signed".to_string(),
            Some(SignatureStatus::Unavailable) | None => "Not available".to_string(),
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
