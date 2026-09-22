// ============================================================================
//  update.rs — CheckSum's self-update support.
//
//  Design goals (per project philosophy — see main.rs doc comment and the
//  project README): simple, auditable, no hidden behavior, no extra crate
//  dependencies. Everything here is either:
//    - a WinINet call (wininet.dll — the same HTTP(S) stack most native
//      Windows apps use; TLS is handled by Windows itself, so there is no
//      bundled TLS library and no custom certificate handling to audit), or
//    - a plain generated .bat script for the actual file replacement step,
//      which anyone can open in Notepad and read top to bottom.
//
//  Flow: newupdate.txt (fetch+parse) -> compare_versions -> download_file
//        (progress-reporting) -> caller SHA-256-verifies the result ->
//        stage_and_relaunch (writes+launches the .bat helper) -> process exits.
//
//  This module never decides on its own whether to update anything; it only
//  provides the primitives. All user-facing confirmation/cancel decisions
//  live in main.rs.
// ============================================================================

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use winapi::um::wininet::{
    HttpQueryInfoW, InternetCloseHandle, InternetOpenUrlW, InternetOpenW, InternetReadFile,
    HINTERNET, HTTP_QUERY_CONTENT_LENGTH, HTTP_QUERY_FLAG_NUMBER, INTERNET_FLAG_NO_CACHE_WRITE,
    INTERNET_FLAG_RELOAD, INTERNET_OPEN_TYPE_PRECONFIG,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Thin RAII wrapper so every WinINet handle is closed on every exit path
/// (including the `?` early-returns below), without repeating cleanup code.
struct NetHandle(HINTERNET);

impl Drop for NetHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                InternetCloseHandle(self.0);
            }
        }
    }
}

fn open_session() -> Result<NetHandle, String> {
    let agent = wide("CheckSum-Updater/1.0");
    let h = unsafe {
        InternetOpenW(
            agent.as_ptr(),
            INTERNET_OPEN_TYPE_PRECONFIG,
            ptr::null(),
            ptr::null(),
            0,
        )
    };
    if h.is_null() {
        return Err("Could not initialize the Windows Internet (WinINet) session.".to_string());
    }
    Ok(NetHandle(h))
}

fn open_url(session: &NetHandle, url: &str) -> Result<NetHandle, String> {
    let url_w = wide(url);
    let flags = INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE;
    let h = unsafe { InternetOpenUrlW(session.0, url_w.as_ptr(), ptr::null(), 0, flags, 0) };
    if h.is_null() {
        return Err(format!("Could not reach: {}", url));
    }
    Ok(NetHandle(h))
}

fn content_length(request: &NetHandle) -> Option<u64> {
    let mut number: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let mut index: u32 = 0;
    let ok = unsafe {
        HttpQueryInfoW(
            request.0,
            HTTP_QUERY_CONTENT_LENGTH | HTTP_QUERY_FLAG_NUMBER,
            &mut number as *mut u32 as *mut _,
            &mut size,
            &mut index,
        )
    };
    if ok != 0 {
        Some(number as u64)
    } else {
        None
    }
}

/// Small-file fetch, fully in memory — used only for `newupdate.txt`, never
/// for the update payload itself (see `download_file` for that).
pub fn fetch_text(url: &str) -> Result<String, String> {
    let session = open_session()?;
    let request = open_url(&session, url)?;

    let mut buf = [0u8; 8192];
    let mut out: Vec<u8> = Vec::new();
    loop {
        let mut read: u32 = 0;
        let ok = unsafe {
            InternetReadFile(
                request.0,
                buf.as_mut_ptr() as *mut _,
                buf.len() as u32,
                &mut read,
            )
        };
        if ok == 0 {
            return Err("Network error while reading the update information.".to_string());
        }
        if read == 0 {
            break;
        }
        out.extend_from_slice(&buf[..read as usize]);
        // newupdate.txt is a small hand-written text file; anything wildly
        // larger than expected is treated as a problem, not a valid answer.
        if out.len() > 1_000_000 {
            return Err("Update information response was unexpectedly large.".to_string());
        }
    }

    String::from_utf8(out).map_err(|_| "Update information was not valid UTF-8 text.".to_string())
}

/// Streams `url` to `dest`, reporting live progress through the two atomics
/// (`downloaded` updated continuously, `total` set once as soon as the
/// server reports a Content-Length). Checked against `cancel` between reads
/// so a user-requested cancel takes effect within one read chunk.
pub fn download_file(
    url: &str,
    dest: &Path,
    downloaded: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let session = open_session()?;
    let request = open_url(&session, url)?;

    if let Some(len) = content_length(&request) {
        total.store(len, Ordering::Relaxed);
    }

    let mut file =
        File::create(dest).map_err(|e| format!("Could not create the update file: {}", e))?;

    let mut buf = vec![0u8; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(dest);
            return Err("Cancelled".to_string());
        }
        let mut read: u32 = 0;
        let ok = unsafe {
            InternetReadFile(
                request.0,
                buf.as_mut_ptr() as *mut _,
                buf.len() as u32,
                &mut read,
            )
        };
        if ok == 0 {
            return Err("Network error while downloading the update.".to_string());
        }
        if read == 0 {
            break;
        }
        file.write_all(&buf[..read as usize])
            .map_err(|e| format!("Could not write the update file: {}", e))?;
        downloaded.fetch_add(read as u64, Ordering::Relaxed);
    }

    Ok(())
}

/// Parsed contents of the single, always-overwritten `newupdate.txt` file.
#[derive(Clone, Debug, Default)]
pub struct UpdateInfo {
    pub version: String,
    pub size_display: String,
    pub download_url: String,
    pub sha256: String,
    pub changes: Vec<String>,
}

/// Parses the simple `Key: value` + `Changes:` format described in the
/// project docs. `Changes:` may be followed by any number of lines, each
/// shown verbatim as one bullet — there is no hard-coded changelog anywhere
/// in this binary; every line shown to the user comes from this file.
pub fn parse_update_info(text: &str) -> Option<UpdateInfo> {
    let mut info = UpdateInfo::default();
    let mut in_changes = false;

    for raw_line in text.lines() {
        let line = raw_line.trim();

        if in_changes {
            if !line.is_empty() {
                info.changes.push(line.to_string());
            }
            continue;
        }

        if line.eq_ignore_ascii_case("Changes:") {
            in_changes = true;
            continue;
        }

        if let Some(rest) = line.strip_prefix("Version:") {
            info.version = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("Size:") {
            info.size_display = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("Download:") {
            info.download_url = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("SHA256:") {
            info.sha256 = rest.trim().to_lowercase();
        }
    }

    if info.version.is_empty() || info.download_url.is_empty() {
        return None;
    }
    Some(info)
}

/// Proper (not lexicographic) version comparison: `"1.9"` < `"1.10"`.
/// Missing trailing components are treated as `0` (`"1.2"` == `"1.2.0"`).
/// Non-numeric components fall back to `0` rather than panicking.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u64> {
        s.trim()
            .split('.')
            .map(|part| part.trim().parse::<u64>().unwrap_or(0))
            .collect()
    };
    let pa = parse(a);
    let pb = parse(b);
    let len = pa.len().max(pb.len());
    for i in 0..len {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Writes a small, fully human-readable `.bat` helper to the temp folder and
/// launches it, then returns immediately — the caller is expected to close
/// the app right after this succeeds. The script:
///   1. waits for this process (by PID) to actually exit,
///   2. backs up the current EXE,
///   3. replaces it with the downloaded one,
///   4. on failure, restores the backup and relaunches the OLD exe,
///   5. on success, relaunches the NEW exe and deletes the backup + itself.
/// No compiled helper binary is introduced — the plain-text script IS the
/// "Helper/Updater process", by design, so it stays trivially auditable.
pub fn stage_and_relaunch(new_exe: &Path) -> Result<(), String> {
    let current_exe =
        std::env::current_exe().map_err(|e| format!("Could not locate the running executable: {}", e))?;
    let backup_path = {
        let mut p = current_exe.clone();
        p.set_extension("exe.bak");
        p
    };
    let pid = std::process::id();
    let bat_path = std::env::temp_dir().join("checksum_updater.bat");

    let script = format!(
        "@echo off\r\n\
         setlocal\r\n\
         set \"TARGET={target}\"\r\n\
         set \"NEWFILE={newfile}\"\r\n\
         set \"BACKUP={backup}\"\r\n\
         set \"PID={pid}\"\r\n\
         \r\n\
         :waitloop\r\n\
         tasklist /FI \"PID eq %PID%\" 2>NUL | find \"%PID%\" >NUL\r\n\
         if not errorlevel 1 (\r\n\
         \x20\x20\x20\x20timeout /t 1 /nobreak >NUL\r\n\
         \x20\x20\x20\x20goto waitloop\r\n\
         )\r\n\
         \r\n\
         copy /Y \"%TARGET%\" \"%BACKUP%\" >NUL\r\n\
         move /Y \"%NEWFILE%\" \"%TARGET%\" >NUL\r\n\
         if errorlevel 1 (\r\n\
         \x20\x20\x20\x20copy /Y \"%BACKUP%\" \"%TARGET%\" >NUL\r\n\
         \x20\x20\x20\x20del \"%BACKUP%\" >NUL 2>&1\r\n\
         \x20\x20\x20\x20start \"\" \"%TARGET%\"\r\n\
         \x20\x20\x20\x20del \"%~f0\"\r\n\
         \x20\x20\x20\x20exit /b 1\r\n\
         )\r\n\
         \r\n\
         del \"%BACKUP%\" >NUL 2>&1\r\n\
         start \"\" \"%TARGET%\"\r\n\
         del \"%~f0\"\r\n",
        target = current_exe.display(),
        newfile = new_exe.display(),
        backup = backup_path.display(),
        pid = pid,
    );

    std::fs::write(&bat_path, script)
        .map_err(|e| format!("Could not write the updater script: {}", e))?;

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("cmd")
        .args(["/C", &bat_path.to_string_lossy()])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Could not launch the updater script: {}", e))?;

    Ok(())
}
