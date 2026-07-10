// SPDX-License-Identifier: Apache-2.0
//! Java `org.gbif.nameparser.util.UnicodeUtils` — ported subset: `normalizeQuotes`, plus
//! the homoglyph-replacement table (`HOMOGLYHPS`/`containsHomoglyphs`/`replaceHomoglyphs`)
//! used by `pipeline::stripandstash`'s step 13. `decompose`/`foldToAscii`/
//! `replaceSpecialCases`/`removeNonAscii`/`replaceNonAscii`/`unescapeUnicodeChars`/
//! `decodeUtf8Garbage`/`containsDiacritics`/`findDiacritics` are NOT ported — no ported
//! call site reaches them yet.

/// Unicode apostrophe / single-quote variants that normalise to the ASCII apostrophe `'`.
/// Verbatim transcription of Java `UnicodeUtils.SINGLE_QUOTES`
/// (`"`´ʹʻʼʽˊˋ‘’‚‛′‵❛❜＇"`).
/// Every codepoint is a 4-hex-digit Java `\u` escape, i.e. a single UTF-16 code unit in
/// the Basic Multilingual Plane outside the surrogate range — so iterating Rust `char`s
/// (Unicode scalar values) reproduces Java's `char`-at-a-time (UTF-16 code unit) loop
/// exactly for this fixed table.
const SINGLE_QUOTES: &[char] = &[
    '\u{0060}', '\u{00B4}', '\u{02B9}', '\u{02BB}', '\u{02BC}', '\u{02BD}', '\u{02CA}', '\u{02CB}',
    '\u{0091}', '\u{0092}', '\u{2018}', '\u{2019}', '\u{201A}', '\u{201B}', '\u{2032}', '\u{2035}',
    '\u{275B}', '\u{275C}', '\u{FF07}',
];

/// Unicode double-quote / double-prime variants that normalise to the ASCII double quote
/// `"`. Verbatim transcription of Java `UnicodeUtils.DOUBLE_QUOTES`
/// (`"“”„‟″‶❝❞〝〞〟＂"`).
const DOUBLE_QUOTES: &[char] = &[
    '\u{0093}', '\u{0094}', '\u{201C}', '\u{201D}', '\u{201E}', '\u{201F}', '\u{2033}', '\u{2036}',
    '\u{275D}', '\u{275E}', '\u{301D}', '\u{301E}', '\u{301F}', '\u{FF02}',
];

// ---------------------------------------------------------------------------------------
// Homoglyph table: Java `UnicodeUtils.HOMOGLYHPS`/`containsHomoglyphs`/`replaceHomoglyphs`
// (`UnicodeUtils.java:58-139` for the static-init loader, `:194-215` and `:239-276` for the
// public methods). Used by `pipeline::stripandstash`'s step 13 (`replaceHomoglyphs`) to
// fold Latin look-alike characters from other scripts (e.g. Cyrillic 'а' U+0430) — and a
// handful of archaic Latin typography, notably U+017F LATIN SMALL LETTER LONG S "ſ" — down
// to their canonical ASCII/Latin letter.
// ---------------------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::LazyLock;

/// The homoglyph table, embedded at compile time (copied verbatim from
/// `name-parser-api/src/main/resources/unicode/homoglyphs.txt`, itself sourced from
/// <https://raw.githubusercontent.com/codebox/homoglyph/master/raw_data/chars.txt> per the
/// resource file's own header comment — `diff`-verified byte-for-byte identical to the
/// Java classpath resource). Each line lists a "canonical" character (the line's first
/// char) followed by every codepoint the codebox project considers a visual look-alike of
/// it. 1861 lines total (`\n`-delimited; the file's own last line has no trailing
/// terminator, the same off-by-one `wc -l` undercounts documented on
/// `pipeline::blacklisted_epithets`'s `BLACKLIST_TXT`) — but see [`HOMOGLYPHS`]'s own doc
/// comment: Java's loader, and this port, only ever read a small prefix of it.
const HOMOGLYPHS_TXT: &str = include_str!("../resources/homoglyphs.txt");

/// U+00D7 MULTIPLICATION SIGN, the botanical hybrid marker ("Abies × Picea"). Java
/// `NameFormatter.HYBRID_MARKER` (`NameFormatter.java:14`: `public static final char
/// HYBRID_MARKER = '×';`). Excluded from the table [`HOMOGLYPHS`] builds below — even
/// though it appears, unremoved, as one of the look-alikes on `homoglyphs.txt`'s `x` row
/// (line 88) — so a genuine hybrid marker is never folded into a plain "x", which would
/// silently destroy hybrid-name semantics. Java's own `UnicodeUtilsTest.containsHomoglyphs`
/// asserts exactly this: `"Abies × Picea"` is NOT flagged as containing a homoglyph.
const HYBRID_MARKER: char = '\u{00D7}';

/// Java `ignoredCanonicals` (`UnicodeUtils.java:67`): `new CharArraySet(new char[]{' ',
/// '\'', '-', '﹘'})`. A `homoglyphs.txt` row whose canonical (first) character is one of
/// these four is skipped ENTIRELY — no homoglyph entries at all are taken from that row.
/// U+0020 SPACE, U+0027 APOSTROPHE, U+002D HYPHEN-MINUS, U+FE58 SMALL EM DASH.
const IGNORED_CANONICALS: [char; 4] = [' ', '\'', '-', '\u{FE58}'];

/// Java's local `ignore` set (`UnicodeUtils.java:93-103`, rebuilt from the literal
/// `"‘’“”"` on every loop iteration but with identical fixed content
/// each time — a code-style quirk, not per-row-varying data). Curly single/double-quote
/// variants are never captured as a homoglyph VALUE on ANY row — quote normalisation is
/// [`normalize_quotes`]'s job, not this table's.
const QUOTE_IGNORE: [char; 4] = ['\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}'];

/// Java `DIACRITICS` (`UnicodeUtils.java:33-56`, static-init block): standalone
/// combining/spacing diacritic marks, excluded from ever being captured as a homoglyph
/// VALUE (a bare combining accent isn't meaningfully "the same as" some base letter, even
/// where the auto-generated codebox table lists it as a look-alike on some canonical's
/// row). Verbatim transcription of the Java string literal
/// `"´˝\` ̏ˆˇ˘ ̑¸¨· ̡ ̢ ̉ ̛ˉ˛ ˚˳῾᾿"` with its 7 embedded literal spaces filtered out
/// (`.filter(cp -> cp != 32)`, exactly like the Java) — 21 codepoints, extracted
/// programmatically from the Java source (not hand-transcribed from the hard-to-read
/// rendered glyphs, several of which are zero-width combining marks that visually vanish
/// or merge with a neighbour when displayed).
const DIACRITICS: [char; 21] = [
    '\u{00B4}', '\u{02DD}', '\u{0060}', '\u{030F}', '\u{02C6}', '\u{02C7}', '\u{02D8}', '\u{0311}',
    '\u{00B8}', '\u{00A8}', '\u{00B7}', '\u{0321}', '\u{0322}', '\u{0309}', '\u{031B}', '\u{02C9}',
    '\u{02DB}', '\u{02DA}', '\u{02F3}', '\u{1FFE}', '\u{1FBF}',
];

/// Java `UnicodeUtils.HOMOGLYHPS` (`UnicodeUtils.java:59-139`, static-init block): a
/// codepoint -> canonical-char map, built once from [`HOMOGLYPHS_TXT`]. Faithful port of
/// the loader's exact algorithm, verified against a compiled run of the real Java
/// `LineReader` over the real resource file (see the loop body comments below for the
/// specific findings that verification surfaced):
///
/// 1. `'ſ'` (U+017F LATIN SMALL LETTER LONG S) -> `'s'` is inserted FIRST, before any table
///    row loads, so it wins over row `f`'s own listing of `'ſ'` as one of ITS look-alikes
///    (`homoglyphs.txt` line 72, canonical `f`) — the loop below never overwrites an
///    existing key. Java `UnicodeUtils.java:71`: `homoglyphs.put(toCodePoint('ſ'), 's');`.
/// 2. Each remaining `homoglyphs.txt` line is a candidate row, in file order. Java's
///    `LineReader` (`skipBlank=true, skipComments=true`, the defaults its 1-arg
///    `InputStream` constructor chains to) skips blank and `#`-comment lines before they
///    ever reach this loop. A plain `line.is_empty()` check stands in for `LineReader`'s
///    fuller `StringUtils.isBlank` (which additionally treats any all-whitespace line as
///    blank): the ONE line in this file that's all-whitespace-but-not-empty (line 9, a run
///    of Unicode space variants: U+0020 U+0020 U+1680 U+2000-U+200A U+2028 U+2029 U+202F
///    U+205F) is NOT actually blank under Java's exact `Character.isWhitespace` (which
///    excludes U+2007 FIGURE SPACE and U+202F NARROW NO-BREAK SPACE, both present on that
///    line) — so Java's `LineReader` does NOT skip it either, and it reaches
///    `UnicodeUtils`'s own loop body same as any other row. It's discarded there instead,
///    by step 3's `IGNORED_CANONICALS` check (its canonical, the first char, is U+0020
///    SPACE) — so the outcome is identical either way; `line.is_empty()` here is not a
///    faithfulness gap.
/// 3. The row's canonical is its first char (`.chars().next()` on a non-empty `str` is
///    exactly Java's `line.charAt(0)` here — the canonical is never a surrogate pair, and a
///    Rust `char` is already a full Unicode scalar value). If the canonical is one of
///    [`IGNORED_CANONICALS`], the entire row is skipped.
/// 4. Otherwise, every remaining char on the row becomes a candidate homoglyph VALUE,
///    filtered exactly as `UnicodeUtils.java:104-111` filters it: codepoint > 128 (skip
///    ASCII/Latin-1-control), not [`HYBRID_MARKER`], not in [`DIACRITICS`], not already a
///    map key (first row wins — relevant beyond just the `'ſ'` pre-seed whenever the same
///    look-alike appears on more than one row), not in [`QUOTE_IGNORE`].
/// 5. After a (non-ignored-canonical) row is fully processed, Java breaks the loop
///    (`UnicodeUtils.java:128`: `if (lr.getRow() > 175 || 'ɸ' == canonical) break;`,
///    comment "skip all rare chars"). `lr.getRow()` is the row's own 1-indexed PHYSICAL
///    line number in the source file (confirmed by compiling and running the real
///    `LineReader` against the real resource file) — i.e. exactly this loop's `row`
///    variable below, a straight `enumerate()` index, not a count of rows actually
///    accepted. The `> 175` numeric fallback is dead code against the actual resource file
///    — canonical `'ɸ'` (U+0278, physical line 151) always fires first — but ported
///    verbatim anyway, matching this port's established precedent of preserving
///    apparently-dead Java guards faithfully (see e.g. `stash_trailing_strain_code`'s
///    `DIGITS_ONLY` guard in `pipeline::stripandstash`). Net effect: only `homoglyphs.txt`
///    lines 1-151 are ever consulted — lines 1-8 are the file's own comment header (all
///    `#`-prefixed), line 9 is the ignored space-canonical row, 2 more rows in range (14,
///    canonical `'`; 20, canonical `-`) are also [`IGNORED_CANONICALS`], and the remaining
///    140 rows (lines 10-151, minus those 2) each contribute at least one map entry — lines
///    152-1861 (the file's long CJK "duplicate codepoint" tail, e.g. `"𦰶𦰶"`) are never
///    read into the map at all — by Java's own design, not a deferral this port introduces.
static HOMOGLYPHS: LazyLock<HashMap<char, char>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert('\u{017F}', 's'); // 'ſ' -> 's', pre-seeded (see step 1 above)

    for (idx, line) in HOMOGLYPHS_TXT.lines().enumerate() {
        let row = idx + 1; // 1-indexed physical line number == Java `lr.getRow()`
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let canonical = line.chars().next().expect("checked non-empty above");
        if IGNORED_CANONICALS.contains(&canonical) {
            continue;
        }
        for cp in line.chars().skip(1) {
            if (cp as u32) > 128
                && cp != HYBRID_MARKER
                && !DIACRITICS.contains(&cp)
                && !map.contains_key(&cp)
                && !QUOTE_IGNORE.contains(&cp)
            {
                map.insert(cp, canonical);
            }
        }
        if row > 175 || canonical == 'ɸ' {
            break;
        }
    }
    map
});

/// Java `UnicodeUtils.containsHomoglyphs(CharSequence)` (`UnicodeUtils.java:194-196`,
/// delegating to `findHomoglyph` at `:203-215`). Returns true if `s` contains at least one
/// character that is a known homoglyph of a Latin letter. Java's version short-circuits via
/// a codepoint lower/upper-bound check before the map lookup (an `Int2CharMap`-specific
/// micro-optimisation); a `HashMap<char, _>` lookup here is already O(1) average without
/// needing that bound check reproduced.
pub fn contains_homoglyphs(s: &str) -> bool {
    s.chars().any(|c| HOMOGLYPHS.contains_key(&c))
}

/// Java `UnicodeUtils.replaceHomoglyphs(CharSequence, boolean)` / its 3-arg sibling
/// (`UnicodeUtils.java:239-276`), specialised to the only shape any ported call site ever
/// needs: `inclHyphens=false` and no `keep` set. `pipeline::stripandstash`'s step 13 (the
/// sole caller) always passes `inclHyphens=false` — Unicode hyphen variants are already
/// normalised to ASCII `-` by the immediately-preceding step 12 (`normaliseHyphens`), per
/// that Java method's own comment ("Hyphen homoglyphs are intentionally excluded — those
/// are normalised above already") — so `HYPHEN_HOMOGLYHPS` (`UnicodeUtils.java:141-167`)
/// and the `keep`-list parameter are both unreachable dead weight for this port's actual
/// call graph and are not ported. Replaces every character present as a key in
/// [`HOMOGLYPHS`] with its mapped canonical; every other character passes through
/// unchanged.
pub fn replace_homoglyphs(s: &str) -> String {
    s.chars()
        .map(|c| *HOMOGLYPHS.get(&c).unwrap_or(&c))
        .collect()
}

/// Faithful port of Java `String.trim()`: strips only leading/trailing chars whose
/// codepoint is <= U+0020 (NOT the full Unicode White_Space set that Rust's str::trim uses).
/// Use this everywhere the Java source calls `.trim()`.
///
/// Java's `String.trim()` only removes characters with codepoint values 0–32 (space through
/// control chars), while Rust's `str::trim()` removes the entire Unicode `White_Space` category.
/// This means non-breaking space (U+00A0) and other Unicode spaces are trimmed by Rust but NOT
/// by Java, causing empty/length guard logic to diverge.
pub fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c as u32) <= 0x20)
}

/// Normalises the many unicode apostrophe / single-quote variants to the ASCII apostrophe
/// (') and the unicode double-quote variants to the ASCII double quote ("). Author names
/// and quoted/provisional names routinely arrive with curly, prime, modifier-letter,
/// low-9, fullwidth or Windows-1252 quotes; collapsing them to ASCII keeps tokenisation
/// and the parsed output consistent regardless of the input's quote style.
///
/// Faithful port of Java `UnicodeUtils.normalizeQuotes(String)`. Java's `null`-in/`null`-out
/// case is not ported: this takes and returns non-nullable Rust types, so callers threading
/// an optional authorship string (e.g. `Option<&str>`) map over it instead
/// (`authorship.map(normalize_quotes)`), which reproduces the same null-forwarding at the
/// `Option` level.
pub fn normalize_quotes(x: &str) -> String {
    x.chars()
        .map(|c| {
            if SINGLE_QUOTES.contains(&c) {
                '\''
            } else if DOUBLE_QUOTES.contains(&c) {
                '"'
            } else {
                c
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Homoglyph table ----

    /// `homoglyphs.txt` lines 10-151 (142 physical lines) are the candidate data rows once
    /// the 8-line `#`-comment header and line 9's ignored space-canonical row are behind us
    /// (see [`HOMOGLYPHS`]'s own doc comment); 2 more of those 142 (line 14, canonical `'`;
    /// line 20, canonical `-`) are also [`IGNORED_CANONICALS`], leaving 140 rows that each
    /// contribute at least one homoglyph codepoint (independently verified against the
    /// source file — no row contributes zero) — plus the `'ſ'` pre-seed, for an exact total
    /// of 1751 map entries. Asserted exactly (not just "a lot"), as a direct regression
    /// guard against a bad resource copy or a loader loop that silently stops early/never
    /// reaches `'ɸ'`/double-counts/under-filters.
    #[test]
    fn resource_loads_exactly_1751_entries() {
        assert_eq!(
            HOMOGLYPHS.len(),
            1751,
            "loaded homoglyph count drifted — see this test's own doc comment"
        );
    }

    /// Java `UnicodeUtilsTest.replaceHomoglyphs` (`UnicodeUtilsTest.java:117`): long-s
    /// folds to plain "s" (NOT "f", even though `ſ` sits on `homoglyphs.txt`'s `f`-canonical
    /// row too — the manual pre-seed wins), while `æ` — a genuine Latin ligature, not a
    /// look-alike — is untouched; this table is not `foldToAscii`. Same shape as the
    /// corpus's `specificEpithet`-affecting rows ("Musca domeſtica", "Amphisbæna
    /// fuliginoſa") — see the sibling test below and `pipeline::stripandstash`'s golden
    /// harness `ALLOWLIST` (now empty, see that file's own history).
    #[test]
    fn long_s_folds_to_s_but_ae_ligature_is_untouched() {
        assert_eq!(
            replace_homoglyphs("Coccinella 2-puſtulata Linnæus, 1758"),
            "Coccinella 2-pustulata Linnæus, 1758"
        );
    }

    /// The two golden-corpus names this port's `replace_homoglyphs` stub used to leave
    /// untouched (`crates/nameparser/tests/parse_golden.rs`, `specificEpithet`/`warnings`
    /// mismatches, now 0). `genus="Amphisbæna"` keeps its `æ` — only the specific epithet's
    /// `ſ` is a homoglyph here.
    #[test]
    fn corpus_long_s_names_fold_to_s() {
        assert_eq!(replace_homoglyphs("Musca domeſtica"), "Musca domestica");
        assert_eq!(
            replace_homoglyphs("Amphisbæna fuliginoſa"),
            "Amphisbæna fuliginosa"
        );
    }

    /// Java `UnicodeUtilsTest.containsHomoglyphs` (`UnicodeUtilsTest.java:77`, comment
    /// "hybrid marker is fine in out domain!"): U+00D7 '×' is excluded from the table even
    /// though it sits on the `x`-canonical row, so a genuine hybrid-formula name survives
    /// `replace_homoglyphs` untouched.
    #[test]
    fn hybrid_marker_is_not_a_homoglyph() {
        assert!(!contains_homoglyphs("Abies × Picea"));
        assert_eq!(replace_homoglyphs("Abies × Picea"), "Abies × Picea");
    }

    /// Java `UnicodeUtilsTest.replaceHomoglyphs` (`UnicodeUtilsTest.java:119-120`, the
    /// `composed` string): digraphs/ligatures round-trip unchanged — this table only
    /// touches genuine cross-script look-alikes, never legitimate distinct Latin letters.
    #[test]
    fn ligatures_and_composed_letters_are_untouched() {
        let composed = "æÆœŒĲĳǈǉȸȹßﬆﬅﬀﬁﬂﬃﬄ";
        assert!(!contains_homoglyphs(composed));
        assert_eq!(replace_homoglyphs(composed), composed);
    }

    /// A Cyrillic 'а' (U+0430, `homoglyphs.txt` line 67's `a`-canonical row) look-alike
    /// mid-word folds to plain "a", flagging as a homoglyph.
    #[test]
    fn cyrillic_a_lookalike_folds_to_latin_a() {
        assert!(contains_homoglyphs("Aus \u{0430}bus"));
        assert_eq!(replace_homoglyphs("Aus \u{0430}bus"), "Aus abus");
    }

    /// A plain-ASCII scientific name contains no homoglyphs and round-trips byte-identical.
    #[test]
    fn plain_ascii_name_has_no_homoglyphs() {
        assert!(!contains_homoglyphs("Abies alba Mill."));
        assert_eq!(replace_homoglyphs("Abies alba Mill."), "Abies alba Mill.");
    }

    /// `¡` (U+00A1) is a Win-1252 transcription artefact handled by the SEPARATE
    /// `repairWin1252Artefacts` step (`pipeline::stripandstash`'s `repair_win1252_artefacts`
    /// step 14) — it is not in the homoglyph table itself. Java
    /// `UnicodeUtilsTest.replaceHomoglyphs` (`UnicodeUtilsTest.java:115`, comment "should
    /// this be a homoglyph?") asserts the same non-membership.
    #[test]
    fn win1252_artefacts_are_not_homoglyphs() {
        assert!(!contains_homoglyphs("¡i"));
        assert_eq!(replace_homoglyphs("¡i"), "¡i");
    }

    #[test]
    fn folds_curly_single_quotes_to_ascii_apostrophe() {
        assert_eq!(normalize_quotes("d\u{2019}Urville"), "d'Urville");
        assert_eq!(normalize_quotes("\u{2018}Aus\u{2019}"), "'Aus'");
    }

    #[test]
    fn folds_curly_double_quotes_to_ascii_double_quote() {
        assert_eq!(normalize_quotes("\u{201C}Aus bus\u{201D}"), "\"Aus bus\"");
    }

    #[test]
    fn folds_backtick_acute_and_fullwidth_apostrophe_variants() {
        assert_eq!(normalize_quotes("\u{0060}"), "'");
        assert_eq!(normalize_quotes("\u{00B4}"), "'");
        assert_eq!(normalize_quotes("\u{FF07}"), "'");
    }

    #[test]
    fn folds_low9_and_prime_double_quote_variants() {
        assert_eq!(normalize_quotes("\u{201E}Aus\u{201F}"), "\"Aus\"");
        assert_eq!(normalize_quotes("\u{2033}"), "\"");
    }

    #[test]
    fn leaves_plain_ascii_and_already_normalised_quotes_untouched() {
        assert_eq!(normalize_quotes("O'Brien"), "O'Brien");
        assert_eq!(normalize_quotes("\"Aus bus\" Smith"), "\"Aus bus\" Smith");
        assert_eq!(normalize_quotes("Abies alba Mill."), "Abies alba Mill.");
    }

    #[test]
    fn java_trim_strips_only_up_to_u0020() {
        assert_eq!(java_trim("  x\t"), "x"); // ASCII space + tab (<=0x20) stripped
        assert_eq!(java_trim("\n x \r"), "x"); // newline/CR stripped
        assert_eq!(java_trim("\u{00A0}x\u{00A0}"), "\u{00A0}x\u{00A0}"); // NBSP (>0x20) NOT stripped
        assert_eq!(java_trim("\u{00A0}"), "\u{00A0}"); // NBSP-only stays non-empty, matching Java
    }
}
