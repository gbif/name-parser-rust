// SPDX-License-Identifier: Apache-2.0

//! `VerdictCache` — on-disk cache of LLM verdicts, mirroring the Java CLI's `llm.VerdictCache`
//! (`/Users/markus/code/gbif/name-parser/name-parser-cli/src/main/java/org/gbif/nameparser/cli/llm/VerdictCache.java`).
//! Keyed by a SHA-256 hash of the exact thing that was judged (prompt version + model + input +
//! serialized parser output), so re-running the validator over an unchanged corpus costs nothing
//! for names already judged — only new or changed parses hit the API. See the recon doc's §7 for
//! the full verified format/behavior this reproduces.
//!
//! Backed by an append-only JSONL file: each line is `{"key":"...","verdict":{...}}`, loaded
//! fully into memory on [`VerdictCache::open`] ("verdict records are tiny", per Java's own
//! comment). Not thread-safe — Java's own javadoc says the same ("guard `put` externally when
//! judging concurrently"); moot for this port, since the (Task 5) judge loop is sequential, like
//! Java's.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{self, BufWriter, Write as _};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{brief, Verdict};

/// One line of the on-disk verdict cache: `{"key":"<sha256-hex>","verdict":{...}}` — the exact
/// shape Java's `VerdictCache.put`/`open` read and write (a bare `JsonObject` with those two
/// members, not a dedicated Gson-mapped class there either).
#[derive(Serialize, Deserialize)]
struct CacheRow {
    key: String,
    verdict: Verdict,
}

/// On-disk cache of LLM verdicts keyed by a hash of the exact thing that was judged — Java
/// `llm.VerdictCache`. See the module doc comment for the format and rationale.
///
/// [`VerdictCache::disabled`] (`--cache=none`) is a real in-memory `HashMap` for the run's
/// duration — it's just never loaded from or persisted to disk. This is Java's own documented
/// "subtlety": [`VerdictCache::put`] always updates the map first, regardless of whether a
/// file-backed writer exists, so a (name, parse-output) pair repeated within one corpus still
/// hits the cache on its second occurrence even with `--cache=none`.
pub struct VerdictCache {
    by_key: HashMap<String, Verdict>,
    appender: Option<BufWriter<std::fs::File>>,
}

impl VerdictCache {
    /// Java `VerdictCache.open(Path)`: if `path` exists, load every JSONL line into memory
    /// up front; otherwise create its parent directory (if any). Either way, then open an
    /// append-mode writer (`CREATE` + `APPEND`) for subsequent [`Self::put`] calls — existing
    /// entries are preserved, new ones are only ever appended, never rewritten or compacted.
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut by_key = HashMap::new();
        if path.exists() {
            let contents = std::fs::read_to_string(path)?;
            for line in contents.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let row: CacheRow = serde_json::from_str(line).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("malformed verdict-cache line: {e} (raw: {})", brief(line)),
                    )
                })?;
                by_key.insert(row.key, row.verdict);
            }
        } else if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(VerdictCache {
            by_key,
            appender: Some(BufWriter::new(file)),
        })
    }

    /// Java `VerdictCache.disabled()`: no file is ever read or written; see the struct doc
    /// comment for the in-memory-only "subtlety" this still preserves.
    pub fn disabled() -> Self {
        VerdictCache {
            by_key: HashMap::new(),
            appender: None,
        }
    }

    /// Java `VerdictCache.size()`.
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// Java `VerdictCache.get(String)`.
    pub fn get(&self, key: &str) -> Option<&Verdict> {
        self.by_key.get(key)
    }

    /// Java `VerdictCache.put(String, Verdict)`: unconditionally record into the in-memory map
    /// (so [`Self::disabled`] mode still caches for the run's duration — see the struct doc
    /// comment), then, only if a file-backed writer exists, append one JSONL line and flush
    /// immediately. The immediate flush (recon §7) is why the cache file, unlike the `--output`
    /// report (only flushed/closed at the very end of a run), is safe to `tail -f` for live
    /// progress.
    pub fn put(&mut self, key: String, verdict: Verdict) -> io::Result<()> {
        if let Some(writer) = self.appender.as_mut() {
            let row = CacheRow {
                key: key.clone(),
                verdict: verdict.clone(),
            };
            let line = serde_json::to_string(&row)?;
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
        self.by_key.insert(key, verdict);
        Ok(())
    }

    /// Java `VerdictCache.key(String...)`: SHA-256 hex digest over each part's UTF-8 bytes, with
    /// a single `\0` byte appended after EVERY part, including the last — this is what makes
    /// `("ab", "c")` hash differently from `("a", "bc")` despite the two pairs concatenating to
    /// the same bytes with no separator. Java's `null -> ""` fallback for a `null` part has no
    /// counterpart here: this port's parts are always `&str` (never nullable), so a caller with
    /// a logically-absent part (e.g. no `shape`) simply passes `""` directly (see [`cache_key`]).
    pub fn key(parts: &[&str]) -> String {
        let mut hasher = Sha256::new();
        for part in parts {
            hasher.update(part.as_bytes());
            hasher.update([0u8]);
        }
        to_hex(&hasher.finalize())
    }
}

/// Lowercase hex encoding of a byte slice (SHA-256 digests only, here) — no extra crate needed
/// for something this small.
fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Java `ValidateCli.cacheKey(model, r)`: the four parts hashed into one verdict-cache key, in
/// order — prompt `ValidationPrompt::VERSION`, `model` id, raw `input` string, and `shape` (the
/// full serialized `parsed`/`error` JSON the model was actually shown — the same JSON used in
/// the prompt payload — or `""` if the caller has neither, e.g. Java's `r.parsed != null ?
/// GSON.toJson(r.parsed) : (r.error == null ? "" : GSON.toJson(r.error))`). Line number / record
/// id are deliberately NOT part of the key: identical (name, parse-output) pairs anywhere in the
/// corpus, or across separate runs, share one cache entry — re-runs and budget bumps don't
/// re-judge already-judged content. Cross-provider/model safety and prompt-version invalidation
/// both fall out of `model`/`version` being hashed in: cloud vs. local verdicts never collide,
/// and a prompt-shape change (a `VERSION` bump) invalidates old entries automatically.
pub fn cache_key(version: &str, model: &str, input: &str, shape: &str) -> String {
    VerdictCache::key(&[version, model, input, shape])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a temp dir under `std::env::temp_dir()` unique to this test — same pattern the
    /// parent `validate` module's own tests already use for filesystem fixtures.
    fn temp_dir_for(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nameparser-cli-validate-cache-test-{label}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn sample_verdict(index: usize) -> Verdict {
        Verdict {
            index,
            verdict: "suspect".to_string(),
            confidence: "med".to_string(),
            fields: vec![super::super::FieldIssue {
                name: "rank".to_string(),
                parsed: "INFRASPECIFIC_NAME".to_string(),
                expected: "SUBSPECIES".to_string(),
                reason: "zoological trinomial".to_string(),
            }],
            note: Some("check rank".to_string()),
        }
    }

    // ---- cache_key / VerdictCache::key — determinism + the `\0`-separator anti-collision ----

    #[test]
    fn cache_key_is_deterministic_for_identical_inputs() {
        let a = cache_key(
            "v1",
            "claude-opus-4-8",
            "Abies alba Mill.",
            "{\"genus\":\"Abies\"}",
        );
        let b = cache_key(
            "v1",
            "claude-opus-4-8",
            "Abies alba Mill.",
            "{\"genus\":\"Abies\"}",
        );
        assert_eq!(a, b);
        // SHA-256 hex: 64 lowercase hex characters.
        assert_eq!(a.len(), 64);
        assert!(a
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn cache_key_changes_if_any_single_part_changes() {
        let base = cache_key("v1", "m", "Abies alba", "{}");
        assert_ne!(
            base,
            cache_key("v2", "m", "Abies alba", "{}"),
            "version must matter"
        );
        assert_ne!(
            base,
            cache_key("v1", "m2", "Abies alba", "{}"),
            "model must matter"
        );
        assert_ne!(
            base,
            cache_key("v1", "m", "Quercus robur", "{}"),
            "input must matter"
        );
        assert_ne!(
            base,
            cache_key("v1", "m", "Abies alba", "{\"x\":1}"),
            "shape must matter"
        );
    }

    #[test]
    fn verdict_cache_key_null_byte_separator_prevents_boundary_collisions() {
        // The whole reason for the `\0`-after-every-part scheme: without it, ("ab","c") and
        // ("a","bc") would concatenate to the identical byte string "abc" and hash the same.
        let ab_c = VerdictCache::key(&["ab", "c"]);
        let a_bc = VerdictCache::key(&["a", "bc"]);
        assert_ne!(
            ab_c, a_bc,
            "(\"ab\",\"c\") and (\"a\",\"bc\") must hash differently"
        );
    }

    #[test]
    fn cache_key_four_part_boundary_shift_also_does_not_collide() {
        // Same anti-collision property, exercised through the real 4-part `cache_key` shape
        // (version/model/input/shape) rather than the low-level 2-part case above.
        let moved_left = cache_key("v1", "m", "ab", "c");
        let moved_right = cache_key("v1", "m", "a", "bc");
        assert_ne!(moved_left, moved_right);
    }

    #[test]
    fn verdict_cache_key_treats_an_empty_shape_part_like_any_other_string() {
        // `shape = ""` (Java: neither `r.parsed` nor `r.error` present) must still hash
        // deterministically and differ from a non-empty shape.
        let empty_shape = cache_key("v1", "m", "Abies alba", "");
        let non_empty_shape = cache_key("v1", "m", "Abies alba", "x");
        assert_eq!(empty_shape, cache_key("v1", "m", "Abies alba", ""));
        assert_ne!(empty_shape, non_empty_shape);
    }

    // ---- open / put / get / reopen — round-trip ----

    #[test]
    fn verdict_cache_open_put_reopen_round_trip_hits_the_cache() {
        let dir = temp_dir_for("round-trip");
        let path = dir.join("cache.jsonl");

        let key = cache_key("v1", "m", "Abies alba Mill.", "{}");
        {
            let mut cache = VerdictCache::open(&path).expect("open (creating) must succeed");
            assert!(cache.get(&key).is_none(), "must start empty");
            cache
                .put(key.clone(), sample_verdict(0))
                .expect("put must succeed");
            assert_eq!(cache.get(&key), Some(&sample_verdict(0)));
        } // cache dropped here — the writer's file handle closes.

        let reopened = VerdictCache::open(&path).expect("reopen must succeed");
        assert_eq!(
            reopened.get(&key),
            Some(&sample_verdict(0)),
            "a verdict written by one VerdictCache instance must be visible after reopening"
        );
        assert_eq!(reopened.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verdict_cache_open_creates_a_missing_parent_directory() {
        let dir = temp_dir_for("mkdir-parent");
        let path = dir.join("nested").join("subdir").join("cache.jsonl");
        assert!(!path.parent().unwrap().exists());

        let cache = VerdictCache::open(&path);
        assert!(cache.is_ok(), "open must create the missing parent dir(s)");
        assert!(path.parent().unwrap().is_dir());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verdict_cache_open_loads_existing_jsonl_entries_without_any_put() {
        let dir = temp_dir_for("preload");
        let path = dir.join("cache.jsonl");
        let key = "deadbeef".to_string();
        let row = CacheRow {
            key: key.clone(),
            verdict: sample_verdict(3),
        };
        std::fs::write(&path, format!("{}\n", serde_json::to_string(&row).unwrap()))
            .expect("write fixture file");

        let cache = VerdictCache::open(&path).expect("open must load the existing line");
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&key), Some(&sample_verdict(3)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verdict_cache_put_appends_and_flushes_immediately_so_the_file_is_readable_while_open() {
        let dir = temp_dir_for("flush-immediately");
        let path = dir.join("cache.jsonl");
        let mut cache = VerdictCache::open(&path).expect("open must succeed");
        cache
            .put("k1".to_string(), sample_verdict(0))
            .expect("put must succeed");

        // Read the file directly, without going through `VerdictCache`, while `cache` is still
        // open (not dropped) — proves `put` flushed rather than relying on eventual/on-drop
        // buffering.
        let on_disk = std::fs::read_to_string(&path).expect("read the cache file directly");
        assert_eq!(on_disk.lines().count(), 1);
        assert!(on_disk.contains("\"key\":\"k1\""));

        cache
            .put("k2".to_string(), sample_verdict(1))
            .expect("second put must succeed");
        let on_disk = std::fs::read_to_string(&path).expect("read again");
        assert_eq!(
            on_disk.lines().count(),
            2,
            "entries are appended, not overwritten"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- disabled() — in-memory-only, per-run cache ----

    #[test]
    fn verdict_cache_disabled_put_then_get_still_hits_the_in_memory_map() {
        // Java's documented "subtlety": `--cache=none` is not a true no-op — `put` always
        // updates the map first, so a (name, parse-output) pair repeated within the SAME run
        // still hits the cache on its second occurrence, even with persistence off.
        let mut cache = VerdictCache::disabled();
        assert!(cache.is_empty());
        let key = cache_key("v1", "m", "Abies alba Mill.", "{}");
        cache
            .put(key.clone(), sample_verdict(0))
            .expect("put on a disabled cache must still succeed");
        assert_eq!(cache.get(&key), Some(&sample_verdict(0)));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn verdict_cache_disabled_never_touches_the_filesystem() {
        // No path is ever given to `disabled()`, so there is nothing to assert against a real
        // file — this test instead pins that many `put`s never panic/error for lack of a
        // backing file (the `appender.is_none()` short-circuit in `put`).
        let mut cache = VerdictCache::disabled();
        for i in 0..5 {
            cache
                .put(format!("k{i}"), sample_verdict(i))
                .expect("disabled-cache put must never fail");
        }
        assert_eq!(cache.len(), 5);
    }
}
