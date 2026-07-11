// SPDX-License-Identifier: Apache-2.0

//! `nameparser-ffi` — a C-ABI `cdylib` wrapping [`nameparser::parse`] for the Phase 3 Java
//! FFM (Panama) binding (`bindings/java/`, a later task in the same plan). Exposes three
//! `extern "C"` functions: [`np_abi_version`], [`np_parse_json`], [`np_free`].
//!
//! **The FFI boundary never unwinds.** Every `extern "C"` function body is wrapped in
//! [`std::panic::catch_unwind`]: [`np_parse_json`] turns a caught panic into a null pointer;
//! [`np_abi_version`] turns one into the sentinel `0`; [`np_free`] just discards the
//! (impossible) `Result`. [`np_abi_version`]/[`np_free`] can't actually panic in practice, but
//! wrapping them anyway keeps this invariant uniform and auditable instead of resting on an
//! unenforced convention — unwinding across a C ABI boundary is undefined behaviour, so a
//! Rust-side panic must never reach the Java caller as an unwind.
//!
//! **Ownership:** every heap pointer [`np_parse_json`] returns is a Rust-allocated
//! `CString`; the caller (Java) must hand it back to [`np_free`] exactly once and must never
//! attempt to free it any other way.
//!
//! `np_parse_json`'s error JSON shape (`{"error":{"type":...,"code":...,"message":...}}`)
//! mirrors `nameparser-cli`'s own `render_row` error-row logic exactly — same field set, same
//! field order, same "omit `code` when absent" rule (see this crate's private
//! `unparsable_json` function and `crates/nameparser-cli/src/main.rs`'s `render_row`) — this
//! module does not invent a new error shape.

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
}
