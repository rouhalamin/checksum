// ============================================================================
//  sig.rs — optional, best-effort Windows Authenticode signature check.
//
//  This is intentionally isolated from the hashing logic: it answers a
//  different question ("did Microsoft's trust chain accept a signature on
//  this file?") and is never allowed to affect the hash match/mismatch
//  verdict. Any failure to determine signature status (missing API, no
//  wintrust.dll, unexpected error) is reported as `Unavailable`, never
//  treated as a hard error for the rest of the app.
//
//  Limitation (documented here on purpose): this checks whether a valid
//  Authenticode signature chain exists, but does not currently resolve and
//  display the signer's certificate subject name — that requires walking
//  the file's PKCS#7 blob via CryptQueryObject/CertGetNameString, which was
//  left out to keep this module small and auditable. `SignatureStatus::Valid`
//  therefore always carries an empty signer string today.
// ============================================================================

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use winapi::shared::guiddef::GUID;
use winapi::shared::minwindef::{DWORD, LPVOID};
use winapi::shared::ntdef::{HANDLE, LONG, LPCWSTR};
use winapi::shared::windef::HWND;

#[derive(Clone, Debug)]
pub enum SignatureStatus {
    /// A valid Authenticode signature chain was found. The signer name is
    /// left empty (see module doc comment) but the variant is kept as
    /// `String` so it can be filled in later without changing call sites.
    Valid(String),
    /// A signature is present but did not validate (broken chain, revoked
    /// certificate, tampered file, etc.).
    Invalid,
    /// The file has no embedded Authenticode signature at all.
    NotSigned,
    /// Could not be determined in this environment (API missing, wintrust
    /// unavailable, or an unexpected error occurred).
    Unavailable,
}

#[repr(C)]
struct WintrustFileInfo {
    cb_struct: DWORD,
    pcwsz_file_path: LPCWSTR,
    h_file: HANDLE,
    pg_known_subject: *mut GUID,
}

#[repr(C)]
struct WintrustData {
    cb_struct: DWORD,
    p_policy_callback_data: LPVOID,
    p_sip_client_data: LPVOID,
    dw_ui_choice: DWORD,
    fdw_revocation_checks: DWORD,
    dw_union_choice: DWORD,
    p_file: *mut WintrustFileInfo,
    dw_state_action: DWORD,
    h_wvt_state_data: HANDLE,
    pwsz_url_reference: LPCWSTR,
    dw_prov_flags: DWORD,
    dw_ui_context: DWORD,
}

const WTD_UI_NONE: DWORD = 2;
const WTD_REVOKE_NONE: DWORD = 0;
const WTD_CHOICE_FILE: DWORD = 1;
const WTD_STATEACTION_VERIFY: DWORD = 1;
const WTD_STATEACTION_CLOSE: DWORD = 2;
const WTD_SAFER_FLAG: DWORD = 0x100;

// {00AAC56B-CD44-11D0-8CC2-00C04FC295EE} — WINTRUST_ACTION_GENERIC_VERIFY_V2
const ACTION_GENERIC_VERIFY_V2: GUID = GUID {
    Data1: 0x00aac56b,
    Data2: 0xcd44,
    Data3: 0x11d0,
    Data4: [0x8c, 0xc2, 0x00, 0xc0, 0x4f, 0xc2, 0x95, 0xee],
};

const TRUST_E_NOSIGNATURE: LONG = 0x800B0100u32 as i32;
const TRUST_E_SUBJECT_FORM_UNKNOWN: LONG = 0x800B0003u32 as i32;
const TRUST_E_PROVIDER_UNKNOWN: LONG = 0x800B0001u32 as i32;

#[link(name = "wintrust")]
extern "system" {
    fn WinVerifyTrust(hwnd: HWND, pg_action_id: *const GUID, pwvt_data: *mut WintrustData) -> LONG;
}

fn to_wide_null(s: &Path) -> Vec<u16> {
    s.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Best-effort Authenticode check. Never panics; any unexpected condition
/// collapses to `SignatureStatus::Unavailable`.
pub fn check_signature(path: &Path) -> SignatureStatus {
    let wide_path = to_wide_null(path);

    let mut file_info = WintrustFileInfo {
        cb_struct: std::mem::size_of::<WintrustFileInfo>() as DWORD,
        pcwsz_file_path: wide_path.as_ptr(),
        h_file: std::ptr::null_mut(),
        pg_known_subject: std::ptr::null_mut(),
    };

    let mut data = WintrustData {
        cb_struct: std::mem::size_of::<WintrustData>() as DWORD,
        p_policy_callback_data: std::ptr::null_mut(),
        p_sip_client_data: std::ptr::null_mut(),
        dw_ui_choice: WTD_UI_NONE,
        fdw_revocation_checks: WTD_REVOKE_NONE,
        dw_union_choice: WTD_CHOICE_FILE,
        p_file: &mut file_info as *mut WintrustFileInfo,
        dw_state_action: WTD_STATEACTION_VERIFY,
        h_wvt_state_data: std::ptr::null_mut(),
        pwsz_url_reference: std::ptr::null(),
        dw_prov_flags: WTD_SAFER_FLAG,
        dw_ui_context: 0,
    };

    let result = unsafe {
        WinVerifyTrust(
            std::ptr::null_mut(),
            &ACTION_GENERIC_VERIFY_V2 as *const GUID,
            &mut data as *mut WintrustData,
        )
    };

    // Always release the state WinVerifyTrust allocated, regardless of the
    // verdict, or WTD_STATEACTION_VERIFY leaks resources per-call.
    data.dw_state_action = WTD_STATEACTION_CLOSE;
    unsafe {
        WinVerifyTrust(
            std::ptr::null_mut(),
            &ACTION_GENERIC_VERIFY_V2 as *const GUID,
            &mut data as *mut WintrustData,
        );
    }

    match result {
        0 => SignatureStatus::Valid(String::new()),
        TRUST_E_NOSIGNATURE => SignatureStatus::NotSigned,
        TRUST_E_SUBJECT_FORM_UNKNOWN | TRUST_E_PROVIDER_UNKNOWN => SignatureStatus::Unavailable,
        _ => SignatureStatus::Invalid,
    }
}
