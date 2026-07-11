// SPDX-License-Identifier: Apache-2.0

//! `nameparser-ffi` — a C-ABI `cdylib` wrapping [`nameparser::parse`] for the Phase 3 Java
//! FFM (Panama) binding (`bindings/java/`, a later task in the same plan). Exposes four
//! `extern "C"` functions: [`np_abi_version`], [`np_parse_json`], [`np_parse_struct`],
//! [`np_free`].
//!
//! **The FFI boundary never unwinds.** Every `extern "C"` function body is wrapped in
//! [`std::panic::catch_unwind`]: [`np_parse_json`] turns a caught panic into a null pointer;
//! [`np_parse_struct`] turns one into the sentinel `-2`; [`np_abi_version`] turns one into the
//! sentinel `0`; [`np_free`] just discards the (impossible) `Result`.
//! [`np_abi_version`]/[`np_free`] can't actually panic in practice, but wrapping them anyway
//! keeps this invariant uniform and auditable instead of resting on an unenforced convention —
//! unwinding across a C ABI boundary is undefined behaviour, so a Rust-side panic must never
//! reach the Java caller as an unwind.
//!
//! **Ownership:** every heap pointer [`np_parse_json`] returns is a Rust-allocated
//! `CString`; the caller (Java) must hand it back to [`np_free`] exactly once and must never
//! attempt to free it any other way. [`np_parse_struct`] never allocates anything the caller
//! must free — it writes into a caller-owned buffer instead (see the [`layout`] module doc for
//! the wire format and return-code protocol).
//!
//! `np_parse_json`'s error JSON shape (`{"error":{"type":...,"code":...,"message":...}}`)
//! mirrors `nameparser-cli`'s own `render_row` error-row logic exactly — same field set, same
//! field order, same "omit `code` when absent" rule (see this crate's private
//! `unparsable_json` function and `crates/nameparser-cli/src/main.rs`'s `render_row`) — this
//! module does not invent a new error shape.

pub mod layout;

use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

use nameparser::model::{NameType, NomCode, ParseError, ParsedName, Rank};

/// The C-ABI surface's version. Java verifies this at load time (a later task) and refuses to
/// proceed on a mismatch, so a stale/rebuilt-incompatible cdylib fails fast instead of
/// silently misbehaving. Bump on any change to the `extern "C"` surface itself (a new,
/// changed, or removed function/signature) — NOT on changes to the core parser's output.
#[no_mangle]
pub extern "C" fn np_abi_version() -> u32 {
    std::panic::catch_unwind(|| 1u32).unwrap_or(0)
}

/// SAFETY: `p` must be either null or a valid, NUL-terminated C string for the duration of
/// this call — guaranteed by the FFI caller (Java, via an FFM downcall) for every argument it
/// passes. Returns `None` for a null pointer or for a payload that isn't valid UTF-8 (a
/// non-UTF-8 argument could never appear in valid JSON output anyway, so folding it to `None`
/// rather than surfacing a distinct error keeps this helper — and every call site below —
/// simple; JDK 22+'s `Arena::allocateFrom(String)` always emits well-formed UTF-8, so this
/// path is not expected to be hit in practice from the Java side).
unsafe fn opt_str<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        None
    } else {
        CStr::from_ptr(p).to_str().ok()
    }
}

/// Parses a scientific name over the C ABI. `name` is required — a null (or non-UTF-8) `name`
/// is reported back as an unparsable-name error object, not a crash. `authorship`/`rank`/
/// `code` are nullable (null = `None`, matching [`nameparser::parse`]'s own `Option`
/// parameters). `rank`/`code` are the Java enum `.name()` strings, resolved via
/// [`Rank::from_name`]/[`NomCode::from_name`]; a non-null but unrecognized name string is
/// folded to `None` (treated the same as an absent hint), matching how those helpers already
/// report an unknown name.
///
/// Returns a heap-allocated, NUL-terminated `CString` the caller must free via [`np_free`]:
/// - on success, `nameparser`'s own `ParsedName` JSON, byte-identical to
///   `serde_json::to_string(&parsed_name)` — this function adds nothing to it;
/// - on an unparsable name, the `{"error":{...}}` envelope described in the module doc
///   comment;
/// - on an internal panic, a null pointer (never an unwind across the C ABI — see the module
///   doc comment).
///
/// # Safety
///
/// `name`, `authorship`, `rank`, `code` must each be either null or a valid, NUL-terminated C
/// string for the duration of this call — the same contract as this crate's private
/// `opt_str` helper, applied to all four arguments (the FFI caller guarantees this for every
/// downcall).
#[no_mangle]
pub unsafe extern "C" fn np_parse_json(
    name: *const c_char,
    authorship: *const c_char,
    rank: *const c_char,
    code: *const c_char,
) -> *mut c_char {
    let result = catch_unwind(|| unsafe {
        let name = match opt_str(name) {
            Some(s) => s,
            None => return unparsable_json(&null_name_error()),
        };
        let authorship = opt_str(authorship);
        let rank = opt_str(rank).and_then(Rank::from_name);
        let code = opt_str(code).and_then(NomCode::from_name);
        match nameparser::parse(name, authorship, rank, code) {
            Ok(pn) => serialize_or_error(&pn, name),
            Err(e) => unparsable_json(&e),
        }
    });
    match result {
        Ok(s) => CString::new(s)
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// The [`ParseError`] used when `name` itself is a null pointer — there is no name string to
/// report `Unparsable ... name: <name>` about, so this bypasses [`ParseError::new`]'s
/// auto-formatted message in favour of one that names the actual condition (a null argument,
/// a Java-side caller bug), while still using the exact same struct — and therefore the exact
/// same [`unparsable_json`] rendering — as every other unparsable-name result. `NameType::Other`
/// is the same catch-all classification the core parser itself uses for input it cannot
/// classify at all (see `nameparser::parse`'s own `parse_rejects_empty_input` test).
fn null_name_error() -> ParseError {
    ParseError {
        type_: NameType::Other,
        code: None,
        name: String::new(),
        message: "null name".to_string(),
    }
}

/// `serde_json::to_string` on a `ParsedName` cannot actually fail in today's schema (no map
/// keys need escaping beyond what `serde_json` always handles, no non-finite floats, nothing
/// else `serde_json` ever rejects mid-stream) — this guard exists only so a future field
/// addition that *could* fail degrades to a well-formed error envelope instead of a
/// `.unwrap()` panic (which `catch_unwind` would still catch, but as an opaque null pointer
/// rather than a diagnosable message).
fn serialize_or_error(pn: &ParsedName, name: &str) -> String {
    serde_json::to_string(pn).unwrap_or_else(|_| {
        unparsable_json(&ParseError {
            type_: NameType::Other,
            code: None,
            name: name.to_string(),
            message: "serialize failed".to_string(),
        })
    })
}

/// Builds `{"error":{"type":...,"code":...,"message":...}}` — byte-for-byte the same shape
/// `nameparser-cli`'s `render_row` writes for its own `"error"` value
/// (`crates/nameparser-cli/src/main.rs`): `code` is OMITTED (not serialized as `null`) when
/// absent. Hand-assembled rather than built via `serde_json::Map`/`json!`, for the exact same
/// reason `render_row` is: this crate's `serde_json` dependency has no `preserve_order`
/// feature enabled, so a dynamically-built `Value::Object` would serialize its keys
/// alphabetically (`code`, `message`, `type`) instead of the intended `type`, `code`,
/// `message` order. Each leaf value is still rendered via `serde_json::to_string`, for
/// correct JSON string escaping.
fn unparsable_json(e: &ParseError) -> String {
    let mut out = String::with_capacity(96);
    out.push_str("{\"error\":{\"type\":");
    out.push_str(&serde_json::to_string(&e.type_).expect("NameType always serializes to JSON"));
    if let Some(code) = &e.code {
        out.push_str(",\"code\":");
        out.push_str(&serde_json::to_string(code).expect("NomCode always serializes to JSON"));
    }
    out.push_str(",\"message\":");
    out.push_str(
        &serde_json::to_string(&e.message).expect("a String always serializes to a JSON string"),
    );
    out.push_str("}}");
    out
}

/// Parses a scientific name over the C ABI, writing the result as a flat fixed-layout binary
/// struct into `out` instead of [`np_parse_json`]'s JSON text — see the [`layout`] module doc
/// for the full byte-for-byte wire format (header offsets, string table, run-slots) and the
/// enum-ordinal mapping. Return convention:
///
/// - `>= 0`: success — the number of bytes written to `out`.
/// - `-1`: unparsable name — `out` receives only the header (`status`, `name_type`, `code`;
///   see [`layout::encode_unparsable`]), so the caller can still throw with the right enums.
/// - `-2`: an internal panic was caught; `out` is untouched.
/// - overflow (the encoded size exceeds `out_cap`): `out` is untouched, returns `-(needed as
///   i64 + 3)`, so the caller recovers `needed = -ret - 3` and retries with a bigger buffer.
///
/// Input handling — `name` required (null or non-UTF-8 folds to the same "null name" error as
/// `np_parse_json`), `authorship`/`rank`/`code` nullable, `rank`/`code` resolved via
/// [`Rank::from_name`]/[`NomCode::from_name`] — is IDENTICAL to [`np_parse_json`]'s, and both
/// read every output field off the exact same [`nameparser::parse`] call's [`ParsedName`], so
/// the struct and JSON paths can never derive a field differently from one another.
///
/// # Safety
///
/// `name`, `authorship`, `rank`, `code` must each be either null or a valid, NUL-terminated C
/// string for the duration of this call — the same contract as [`np_parse_json`]. `out` must
/// be either null (only permitted when `out_cap == 0`) or valid for writes of at least
/// `out_cap` bytes; this function never writes more than `out_cap` bytes to it.
#[no_mangle]
pub unsafe extern "C" fn np_parse_struct(
    name: *const c_char,
    authorship: *const c_char,
    rank: *const c_char,
    code: *const c_char,
    out: *mut u8,
    out_cap: usize,
) -> i64 {
    let result = catch_unwind(|| unsafe {
        let name = match opt_str(name) {
            Some(s) => s,
            None => {
                return Err(layout::encode_unparsable(
                    &null_name_error(),
                    np_abi_version(),
                ))
            }
        };
        let authorship = opt_str(authorship);
        let rank = opt_str(rank).and_then(Rank::from_name);
        let code = opt_str(code).and_then(NomCode::from_name);
        match nameparser::parse(name, authorship, rank, code) {
            Ok(pn) => Ok(layout::encode(&pn, np_abi_version())),
            Err(e) => Err(layout::encode_unparsable(&e, np_abi_version())),
        }
    });
    match result {
        Ok(Ok(buf)) => write_success(&buf, out, out_cap),
        Ok(Err(header)) => {
            write_clamped_header(&header, out, out_cap);
            -1
        }
        Err(_) => -2,
    }
}

/// Copies `buf` into `out` and returns its length, or — if `buf` doesn't fit `out_cap` —
/// touches `out` not at all and returns the negative-needed overflow code (see
/// [`np_parse_struct`]'s doc comment).
fn write_success(buf: &[u8], out: *mut u8, out_cap: usize) -> i64 {
    if buf.len() > out_cap {
        return -(buf.len() as i64 + 3);
    }
    unsafe { copy_into(buf, out) };
    buf.len() as i64
}

/// Copies as much of `header` as fits `out_cap` into `out`, silently truncating if `out_cap <
/// layout::HEADER_SIZE` — used only by [`np_parse_struct`]'s unparsable path, which always
/// returns `-1` regardless of truncation (callers should supply at least
/// [`layout::HEADER_SIZE`] bytes to reliably decode it; see the `layout` module doc).
fn write_clamped_header(header: &[u8], out: *mut u8, out_cap: usize) {
    let n = header.len().min(out_cap);
    unsafe { copy_into(&header[..n], out) };
}

/// Shared `copy_nonoverlapping` wrapper: a no-op for an empty slice, so a null/dangling `out`
/// paired with a zero-length write (`out_cap == 0`, or a fully-truncated clamp) is never
/// dereferenced — `copy_nonoverlapping` requires non-null pointers even for zero-size copies.
///
/// # Safety
///
/// `out` must be valid for writes of at least `bytes.len()` bytes whenever `bytes` is
/// non-empty — guaranteed by both call sites above, which only ever pass a slice already
/// known to fit within `np_parse_struct`'s own `out_cap` safety contract.
unsafe fn copy_into(bytes: &[u8], out: *mut u8) {
    if !bytes.is_empty() {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
    }
}

/// Frees a `CString` previously returned by [`np_parse_json`]. Null-safe (a no-op on a null
/// pointer), so Java can call it unconditionally after any downcall, including one that
/// itself returned null.
///
/// # Safety
///
/// `ptr` must be either null or a pointer previously returned by [`np_parse_json`] and not
/// already passed to `np_free` — passing any other pointer, or freeing the same pointer
/// twice, is undefined behaviour (the same contract as `CString::from_raw`).
#[no_mangle]
pub unsafe extern "C" fn np_free(ptr: *mut c_char) {
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        if !ptr.is_null() {
            drop(CString::from_raw(ptr));
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn np_abi_version_is_1() {
        assert_eq!(np_abi_version(), 1);
    }

    #[test]
    fn opt_str_returns_none_for_null() {
        assert_eq!(unsafe { opt_str(std::ptr::null()) }, None);
    }

    #[test]
    fn opt_str_returns_some_for_a_valid_c_string() {
        let c = CString::new("Abies alba").unwrap();
        assert_eq!(unsafe { opt_str(c.as_ptr()) }, Some("Abies alba"));
    }

    #[test]
    fn unparsable_json_omits_code_when_absent() {
        let e = ParseError::new(NameType::Other, None, "???");
        assert_eq!(
            unparsable_json(&e),
            r#"{"error":{"type":"OTHER","message":"Unparsable OTHER name: ???"}}"#
        );
    }

    #[test]
    fn unparsable_json_includes_code_when_present() {
        let e = ParseError::new(
            NameType::Other,
            Some(NomCode::Virus),
            "Tobacco mosaic virus",
        );
        assert_eq!(
            unparsable_json(&e),
            r#"{"error":{"type":"OTHER","code":"VIRUS","message":"Unparsable OTHER name: Tobacco mosaic virus"}}"#
        );
    }

    #[test]
    fn null_name_error_is_other_typed_with_no_code() {
        let e = null_name_error();
        assert_eq!(e.type_, NameType::Other);
        assert_eq!(e.code, None);
        assert_eq!(e.message, "null name");
    }

    #[test]
    fn np_free_of_null_is_a_no_op() {
        unsafe { np_free(std::ptr::null_mut()) };
    }

    #[test]
    fn np_parse_json_round_trip_via_raw_pointers_ok_case() {
        let name = CString::new("Abies alba Mill.").unwrap();
        let ptr = unsafe {
            np_parse_json(
                name.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_string();
        assert!(json.contains(r#""genus":"Abies""#));
        unsafe { np_free(ptr) };
    }

    #[test]
    fn np_parse_json_null_name_returns_non_null_error_json() {
        let ptr = unsafe {
            np_parse_json(
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_string();
        assert_eq!(json, r#"{"error":{"type":"OTHER","message":"null name"}}"#);
        unsafe { np_free(ptr) };
    }

    // ---- np_parse_struct: basic mechanics (full field-level coverage lives in
    // tests/ffi_struct.rs, which decodes the wire format and compares against
    // nameparser::parse directly) ----

    #[test]
    fn np_parse_struct_zero_cap_returns_the_negative_needed_overflow_code() {
        let name = CString::new("Abies alba Mill.").unwrap();
        let ret = unsafe {
            np_parse_struct(
                name.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                0,
            )
        };
        assert!(ret <= -3, "expected an overflow code (<= -3), got {ret}");
        let needed = -ret - 3;
        assert!(needed > 0);
    }

    #[test]
    fn np_parse_struct_round_trip_via_raw_pointers_ok_case_writes_the_abi_version_header() {
        let name = CString::new("Abies alba Mill.").unwrap();
        let mut buf = vec![0u8; 4096];
        let ret = unsafe {
            np_parse_struct(
                name.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                buf.as_mut_ptr(),
                buf.len(),
            )
        };
        assert!(ret >= 0, "expected success, got {ret}");
        let abi = u32::from_le_bytes(
            buf[layout::OFF_ABI_VERSION..layout::OFF_ABI_VERSION + 4]
                .try_into()
                .unwrap(),
        );
        assert_eq!(abi, np_abi_version());
        let status = i32::from_le_bytes(
            buf[layout::OFF_STATUS..layout::OFF_STATUS + 4]
                .try_into()
                .unwrap(),
        );
        assert_eq!(status, layout::STATUS_SUCCESS);
    }

    #[test]
    fn np_parse_struct_null_name_returns_minus_one_with_header_only() {
        let mut buf = vec![0xAAu8; layout::HEADER_SIZE];
        let ret = unsafe {
            np_parse_struct(
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                buf.as_mut_ptr(),
                buf.len(),
            )
        };
        assert_eq!(ret, -1);
        let status = i32::from_le_bytes(
            buf[layout::OFF_STATUS..layout::OFF_STATUS + 4]
                .try_into()
                .unwrap(),
        );
        assert_eq!(status, layout::STATUS_UNPARSABLE);
        let name_type = i32::from_le_bytes(
            buf[layout::OFF_NAME_TYPE..layout::OFF_NAME_TYPE + 4]
                .try_into()
                .unwrap(),
        );
        assert_eq!(name_type, layout::name_type_ordinal(NameType::Other));
    }

    #[test]
    fn np_parse_struct_unparsable_virus_returns_minus_one_with_type_and_code() {
        let name = CString::new("Tobacco mosaic virus").unwrap();
        let mut buf = vec![0u8; layout::HEADER_SIZE];
        let ret = unsafe {
            np_parse_struct(
                name.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                buf.as_mut_ptr(),
                buf.len(),
            )
        };
        assert_eq!(ret, -1);
        let code = i32::from_le_bytes(
            buf[layout::OFF_CODE..layout::OFF_CODE + 4]
                .try_into()
                .unwrap(),
        );
        assert_eq!(code, layout::nomcode_ordinal(NomCode::Virus));
    }
}
