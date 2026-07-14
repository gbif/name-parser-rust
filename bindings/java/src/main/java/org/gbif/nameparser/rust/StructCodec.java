// SPDX-License-Identifier: Apache-2.0
package org.gbif.nameparser.rust;

import org.gbif.nameparser.api.Authorship;
import org.gbif.nameparser.api.CombinedAuthorship;
import org.gbif.nameparser.api.NamePart;
import org.gbif.nameparser.api.NameType;
import org.gbif.nameparser.api.NomCode;
import org.gbif.nameparser.api.ParseResult;
import org.gbif.nameparser.api.ParsedName;
import org.gbif.nameparser.api.Rank;
import org.gbif.nameparser.api.UnparsableNameException;

import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

/**
 * Reader for the flat fixed-layout binary wire format {@code np_parse_struct} writes -- the
 * exact offsets/slots/run-tables documented in {@code crates/nameparser-ffi/src/layout.rs}'s
 * module doc comment. This class owns every offset/slot constant for that wire
 * format plus the one-time startup guard that makes trusting those constants safe (see the
 * static initializer below) -- {@link Ffi} stays pure FFM plumbing (arena, downcall, retry) and
 * defers to this class for everything struct-shaped.
 *
 * <p><b>Byte order is little-endian throughout</b> -- {@link #LE_INT} is built with an explicit
 * {@link ByteOrder#LITTLE_ENDIAN}, never the platform-default/{@code ByteBuffer}-default
 * (big-endian) layout. Every multi-byte scalar this class reads goes through it.
 *
 * <p><b>Setter semantics matter here</b> (see {@link #decode}): {@code ParsedName}'s epithet
 * setters (e.g. {@code setGenus}) strip a leading {@code ×} and call {@code addNotho(...)} as a
 * side effect, and {@code addAuthor}/{@code addExAuthor} have an inverted-blank-check bug that
 * makes them no-ops for real (non-blank) authors -- so authors/ex-authors are populated via
 * {@code setAuthors}/{@code setExAuthors}, and {@code notho} is rebuilt explicitly and exactly
 * from the wire's {@code notho_bits} after the epithet setters have already run, rather than
 * trusted to those setters' side effects.
 */
final class StructCodec {

  // ================================================================================================
  // Layout constants -- mirror crates/nameparser-ffi/src/layout.rs's `pub const`s 1:1. Keep in
  // sync with that file (the canonical source; see its module doc) if the wire format ever
  // changes -- a bump there without a matching change here is exactly the class of bug the
  // enum-ordinal guard below cannot catch (it only guards enum ordinals, not byte offsets).
  // ================================================================================================

  static final int HEADER_SIZE = 36;
  private static final int OFF_STATUS = 4;
  private static final int OFF_RANK = 8;
  private static final int OFF_CODE = 12;
  private static final int OFF_NAME_TYPE = 16;
  private static final int OFF_STATE = 20;
  private static final int OFF_CANDIDATUS = 24;
  private static final int OFF_DOUBTFUL = 25;
  private static final int OFF_MANUSCRIPT = 26;
  private static final int OFF_EXTINCT = 27;
  private static final int OFF_ORIGINAL_SPELLING = 28;
  private static final int OFF_NOTHO_BITS = 29;
  private static final int OFF_PUBLISHED_IN_YEAR = 32;

  private static final int STATUS_SUCCESS = 0;

  private static final int ABSENT_ENUM = -1;

  private static final int ORIGINAL_SPELLING_FALSE = 0;
  private static final int ORIGINAL_SPELLING_TRUE = 1;
  private static final int ORIGINAL_SPELLING_UNKNOWN = 2;

  private static final int STRING_TABLE_OFFSET = HEADER_SIZE; // 36
  // Unparsable path only (ABI 3): a u32 name length at HEADER_SIZE, then the inline UTF-8 name
  // bytes -- the same post-header region the success path uses for its string table, told apart by
  // status. Lets the binding return the core's (possibly canonicalized) ParseError.name.
  private static final int OFF_UNPARSABLE_NAME_LEN = HEADER_SIZE; // 36
  private static final int NUM_STRING_SLOTS = 17;
  private static final int STRING_REF_SIZE = 8;
  private static final int STRING_TABLE_SIZE = 4 + NUM_STRING_SLOTS * STRING_REF_SIZE; // 140

  /** {@code u32::MAX} read back as a little-endian signed {@code i32} -- conveniently just
   *  {@code -1}, so the absent-string sentinel needs no unsigned comparison. */
  private static final int ABSENT_STRING_OFFSET = -1;

  private static final int SLOT_UNINOMIAL = 0;
  private static final int SLOT_GENUS = 1;
  private static final int SLOT_INFRAGENERIC = 2;
  private static final int SLOT_SPECIFIC = 3;
  private static final int SLOT_INFRASPECIFIC = 4;
  private static final int SLOT_CULTIVAR = 5;
  private static final int SLOT_PHRASE = 6;
  private static final int SLOT_TAXONOMIC_NOTE = 7;
  private static final int SLOT_NOMENCLATURAL_NOTE = 8;
  private static final int SLOT_PUBLISHED_IN = 9;
  private static final int SLOT_PUBLISHED_IN_PAGE = 10;
  private static final int SLOT_UNPARSED = 11;
  private static final int SLOT_SANCTIONING_AUTHOR = 12;
  private static final int SLOT_YEAR_COMB = 13;
  private static final int SLOT_YEAR_BAS = 14;
  private static final int SLOT_IMPRINT_YEAR_COMB = 15;
  private static final int SLOT_IMPRINT_YEAR_BAS = 16;

  private static final int RUN_SLOTS_OFFSET = STRING_TABLE_OFFSET + STRING_TABLE_SIZE; // 176
  private static final int EPITHET_QUALIFIER_ENTRY_SIZE = 12;

  private static final int GROUP_ABSENT = 0;
  private static final int GROUP_PRESENT = 1;

  private static final ValueLayout.OfInt LE_INT =
      ValueLayout.JAVA_INT_UNALIGNED.withOrder(ByteOrder.LITTLE_ENDIAN);

  // ================================================================================================
  // Enum-ordinal consistency guard -- runs once, the first time this class is touched (i.e. the
  // first parse, since the flat struct is the binding's only decode path). The wire format maps
  // five Java enums
  // (Rank/NomCode/NameType/NamePart/ParsedName.State) by raw i32 ordinal -- see the class doc --
  // so if a future name-parser-api release ever reorders/inserts/removes a constant, silently
  // trusting `.values()[ordinal]` would misdecode every enum-typed field without any crash. This
  // block fails fast with a clear message instead. Referencing Ffi.nativeAbiVersion() below is a
  // real method call (not a compile-time-constant field read), so it also forces Ffi's own
  // static initializer -- which independently verifies np_abi_version() itself -- to run to
  // completion first; if that already failed, this class is never even reached.
  // ================================================================================================

  static {
    int abi = Ffi.nativeAbiVersion();
    if (abi != 4) {
      throw new ExceptionInInitializerError(new IllegalStateException(
          "Rust/Java enum ABI desync -- nameparser-ffi np_abi_version()=" + abi
              + ", StructCodec was written against 4 -- rebuild the cdylib "
              + "(`cargo build -p nameparser-ffi --release`) or update StructCodec"));
    }
    requireEnumShape("Rank", Rank.values().length, 117);
    requireEnumShape("NameType", NameType.values().length, 6);
    requireEnumShape("NomCode", NomCode.values().length, 7);
    requireEnumShape("NamePart", NamePart.values().length, 4);
    requireEnumShape("ParsedName.State", ParsedName.State.values().length, 3);

    // Spot-checks pinned against the same Java-oracle values layout.rs's own tests independently
    // re-verified on the Rust side (see task-5-report.md's "Enum ordinal mapping" section).
    requireOrdinal("Rank.KINGDOM", Rank.KINGDOM.ordinal(), 8);
    requireOrdinal("Rank.FAMILY", Rank.FAMILY.ordinal(), 64);
    requireOrdinal("Rank.GENUS", Rank.GENUS.ordinal(), 73);
    requireOrdinal("Rank.SPECIES", Rank.SPECIES.ordinal(), 85);
    requireOrdinal("Rank.SUBSPECIES", Rank.SUBSPECIES.ordinal(), 89);
    requireOrdinal("Rank.CULTIVAR", Rank.CULTIVAR.ordinal(), 112);
    requireOrdinal("Rank.OTHER", Rank.OTHER.ordinal(), 115);
    requireOrdinal("Rank.UNRANKED", Rank.UNRANKED.ordinal(), 116);
    requireOrdinal("NameType.IDENTIFIER", NameType.IDENTIFIER.ordinal(), 4);
    requireOrdinal("NameType.OTHER", NameType.OTHER.ordinal(), 5);
    requireOrdinal("NamePart.GENERIC", NamePart.GENERIC.ordinal(), 0);
    requireOrdinal("NamePart.INFRAGENERIC", NamePart.INFRAGENERIC.ordinal(), 1);
    requireOrdinal("NamePart.SPECIFIC", NamePart.SPECIFIC.ordinal(), 2);
    requireOrdinal("NamePart.INFRASPECIFIC", NamePart.INFRASPECIFIC.ordinal(), 3);
    requireOrdinal("NomCode.BACTERIAL", NomCode.BACTERIAL.ordinal(), 0);
    requireOrdinal("NomCode.PHYLO", NomCode.PHYLO.ordinal(), 6);
    requireOrdinal("ParsedName.State.COMPLETE", ParsedName.State.COMPLETE.ordinal(), 0);
    requireOrdinal("ParsedName.State.NONE", ParsedName.State.NONE.ordinal(), 2);
  }

  private static void requireEnumShape(String enumName, int actualLength, int expectedLength) {
    if (actualLength != expectedLength) {
      throw new ExceptionInInitializerError(new IllegalStateException(
          "Rust/Java enum ABI desync -- " + enumName + ".values().length=" + actualLength
              + ", the nameparser-ffi struct wire format was built against " + expectedLength
              + " -- rebuild the cdylib (`cargo build -p nameparser-ffi --release`) or update StructCodec"));
    }
  }

  private static void requireOrdinal(String label, int actualOrdinal, int expectedOrdinal) {
    if (actualOrdinal != expectedOrdinal) {
      throw new ExceptionInInitializerError(new IllegalStateException(
          "Rust/Java enum ABI desync -- " + label + ".ordinal()=" + actualOrdinal
              + ", expected " + expectedOrdinal
              + " -- rebuild the cdylib (`cargo build -p nameparser-ffi --release`) or update StructCodec"));
    }
  }

  private StructCodec() {
  }

  // ================================================================================================
  // Unparsable-path header decode (np_parse_struct returned -1)
  // ================================================================================================

  /**
   * Reads the {@code name_type}/{@code code} header fields plus the error {@code name} from a
   * {@code -1} (unparsable) result buffer and builds the exception {@link Ffi#callParseStruct}
   * throws for it. The name comes off the wire (ABI 3) so a canonicalized form (OTU {@code
   * sh…}→{@code SH…}, an extracted substring) is preserved; {@code originalName} is only a fallback
   * for the empty-name case (see {@link #unparsableName}).
   */
  static UnparsableNameException unparsableException(MemorySegment seg, String originalName) {
    int nameTypeOrd = seg.get(LE_INT, OFF_NAME_TYPE);
    int codeOrd = seg.get(LE_INT, OFF_CODE);
    NameType type = enumByOrdinal(NameType.values(), nameTypeOrd, "name_type");
    NomCode code = codeOrd == ABSENT_ENUM ? null : enumByOrdinal(NomCode.values(), codeOrd, "code");
    return new UnparsableNameException(type, code, unparsableName(seg, originalName));
  }

  /**
   * Builds the {@link ParseResult.Unparsable} for a {@code -1} result buffer — the 5.0.0
   * exceptionless counterpart to {@link #unparsableException}. The Rust FFI already clamped a
   * parsable error type (INFORMAL/SCIENTIFIC) to OTHER before encoding (see
   * {@code ParseError::clamped_to_unparsable}), so the {@code name_type} read here is always a
   * non-parsable type and the {@link ParseResult.Unparsable} record's own {@code isParsable()}
   * guard never trips.
   */
  static ParseResult unparsableResult(MemorySegment seg, String originalName) {
    int nameTypeOrd = seg.get(LE_INT, OFF_NAME_TYPE);
    int codeOrd = seg.get(LE_INT, OFF_CODE);
    NameType type = enumByOrdinal(NameType.values(), nameTypeOrd, "name_type");
    NomCode code = codeOrd == ABSENT_ENUM ? null : enumByOrdinal(NomCode.values(), codeOrd, "code");
    return new ParseResult.Unparsable(type, code, unparsableName(seg, originalName));
  }

  /**
   * The ABI-3 unparsable-path error name: a {@code u32} length at {@link #OFF_UNPARSABLE_NAME_LEN}
   * then that many inline UTF-8 bytes -- the core's {@code ParseError.name}, which may be
   * canonicalized (OTU {@code sh…}→{@code SH…}, an extracted substring) and so differ from the
   * caller's input. Falls back to {@code originalName} only when the wire name is empty (a
   * null/blank input the core reports with no name text).
   */
  private static String unparsableName(MemorySegment seg, String originalName) {
    long len = Integer.toUnsignedLong(seg.get(LE_INT, OFF_UNPARSABLE_NAME_LEN));
    if (len == 0) {
      return originalName;
    }
    byte[] bytes = seg.asSlice(OFF_UNPARSABLE_NAME_LEN + 4, len).toArray(ValueLayout.JAVA_BYTE);
    return new String(bytes, StandardCharsets.UTF_8);
  }

  // ================================================================================================
  // 5.0.0 three-way split — applied to a decoded ParsedName. A DELIBERATE, minimal mirror of the
  // Rust core's `is_informal` + `to_informal` (crates/nameparser/src/lib.rs): both are pure
  // functions of the ParsedName fields the wire already carries (type, specificEpithet, genus,
  // uninomial, infragenericEpithet, rank, phrase, code), so no dedicated wire payload is needed for
  // `Informal`. KEEP IN SYNC with lib.rs; NameParserRustSmokeTest locks the outputs against the
  // Rust-authoritative values, from the corpus study that drove them.
  // ================================================================================================

  /**
   * Splits a successfully decoded {@link ParsedName} into the 5.0.0 {@link ParseResult}: an
   * {@link ParseResult.Informal} for a supraspecific taxon carrying a provisional designation with
   * no species epithet, else a {@link ParseResult.Parsed}. A name with a species epithet (incl.
   * cf./aff. and infraspecific-indeterminate binomials) stays {@code Parsed} so its
   * {@code specificAuthorship} — unrepresentable by a flat anchor — is preserved.
   */
  static ParseResult toParseResult(ParsedName pn) {
    if (isInformal(pn)) {
      String taxon;
      Rank taxonRank;
      if (pn.getGenus() != null) {
        // the overwhelming "Genus sp. <tag>" majority — anchor sits in the genus slot
        taxon = pn.getGenus();
        taxonRank = Rank.GENUS;
      } else if (pn.getUninomial() != null) {
        taxon = pn.getUninomial();
        taxonRank = pn.getRank();
      } else {
        taxon = pn.getInfragenericEpithet();
        taxonRank = pn.getRank();
      }
      return new ParseResult.Informal(taxon, taxonRank, pn.getRank(), pn.getPhrase(), pn.getCode());
    }
    return new ParseResult.Parsed(pn);
  }

  /**
   * The informal discriminator — mirror of Rust {@code lib.rs::is_informal}: an INFORMAL-typed name
   * with a real supraspecific anchor but no species epithet. Keying on the specific epithet routes
   * cf./aff. and infraspecific-indeterminate binomials to {@code Parsed} automatically.
   */
  private static boolean isInformal(ParsedName pn) {
    return pn.getType() == NameType.INFORMAL
        && pn.getSpecificEpithet() == null
        && (pn.getGenus() != null
            || pn.getUninomial() != null
            || pn.getInfragenericEpithet() != null);
  }

  // ================================================================================================
  // Success-path decode
  // ================================================================================================

  /**
   * Decodes a successful {@code np_parse_struct} result (header + string table + run-slots +
   * nested authorship groups, per the class doc) into a fresh {@link ParsedName}. {@code len} is
   * the exact byte count {@code np_parse_struct} reported (not {@code seg}'s own allocated
   * capacity, which may be larger) -- used both as the up-front floor/trailing sanity bounds and,
   * critically, as the ceiling every variable wire {@code count} is validated against BEFORE it
   * sizes an allocation (see {@link #checkCount}), so a corrupt/truncated buffer is rejected with
   * a clear message rather than triggering a huge allocation from a bogus count.
   */
  static ParsedName decode(MemorySegment seg, int len) {
    // Reject a buffer too small to even hold the fixed header + string-table region before any
    // fixed-offset read below can walk past the reported length. Every legitimate success-path
    // buffer is >= 208 bytes (RUN_SLOTS_OFFSET is 176), so this only ever fires on a
    // corrupt/truncated buffer -- and makes the fixed-region reads that follow safe by construction.
    if (len < RUN_SLOTS_OFFSET) {
      throw new IllegalStateException("corrupt struct buffer: reported length " + len
          + " is below the fixed header + string-table size " + RUN_SLOTS_OFFSET);
    }

    int status = seg.get(LE_INT, OFF_STATUS);
    if (status != STATUS_SUCCESS) {
      throw new IllegalStateException(
          "StructCodec.decode called on a non-success buffer (status=" + status + ")");
    }

    int rankOrd = seg.get(LE_INT, OFF_RANK);
    int codeOrd = seg.get(LE_INT, OFF_CODE);
    int nameTypeOrd = seg.get(LE_INT, OFF_NAME_TYPE);
    int stateOrd = seg.get(LE_INT, OFF_STATE);
    boolean candidatus = seg.get(ValueLayout.JAVA_BYTE, OFF_CANDIDATUS) != 0;
    boolean doubtful = seg.get(ValueLayout.JAVA_BYTE, OFF_DOUBTFUL) != 0;
    boolean manuscript = seg.get(ValueLayout.JAVA_BYTE, OFF_MANUSCRIPT) != 0;
    boolean extinct = seg.get(ValueLayout.JAVA_BYTE, OFF_EXTINCT) != 0;
    int originalSpellingByte = seg.get(ValueLayout.JAVA_BYTE, OFF_ORIGINAL_SPELLING) & 0xFF;
    int nothoBits = seg.get(ValueLayout.JAVA_BYTE, OFF_NOTHO_BITS) & 0xFF;
    int publishedInYear = seg.get(LE_INT, OFF_PUBLISHED_IN_YEAR);

    // The string-table count is a fixed constant, not a length-driving wire value: it must be
    // exactly NUM_STRING_SLOTS (rejected below otherwise), and the array it fills is sized by that
    // compile-time constant, never by the wire count -- so there is no unbounded-allocation risk
    // here, and the 17 fixed-offset entry reads (ending at RUN_SLOTS_OFFSET) are already covered
    // by the len >= RUN_SLOTS_OFFSET floor check above.
    int stringTableCount = seg.get(LE_INT, STRING_TABLE_OFFSET);
    if (stringTableCount != NUM_STRING_SLOTS) {
      throw new IllegalStateException(
          "nameparser-ffi: string table count=" + stringTableCount + ", expected " + NUM_STRING_SLOTS);
    }
    String[] strings = new String[NUM_STRING_SLOTS];
    for (int i = 0; i < NUM_STRING_SLOTS; i++) {
      long entryOff = STRING_TABLE_OFFSET + 4L + (long) i * STRING_REF_SIZE;
      strings[i] = readOptString(seg, entryOff);
    }

    Cursor cur = new Cursor(RUN_SLOTS_OFFSET);
    List<String> authorsComb = readStringRun(seg, cur, len, "combination authors");
    List<String> exAuthorsComb = readStringRun(seg, cur, len, "combination ex-authors");
    List<String> authorsBas = readStringRun(seg, cur, len, "basionym authors");
    List<String> exAuthorsBas = readStringRun(seg, cur, len, "basionym ex-authors");
    List<String> warnings = readStringRun(seg, cur, len, "warnings");

    int eqCount = seg.get(LE_INT, cur.pos);
    cur.pos += 4;
    // Bound the wire count against the bytes remaining BEFORE sizing any allocation off it, so a
    // corrupt/truncated buffer carrying a bogus huge count is rejected here rather than turned
    // into a massive new int[]/new String[] first.
    checkCount("epithetQualifier", eqCount, EPITHET_QUALIFIER_ENTRY_SIZE, cur.pos, len);
    int[] eqParts = new int[eqCount];
    String[] eqValues = new String[eqCount];
    for (int i = 0; i < eqCount; i++) {
      eqParts[i] = seg.get(LE_INT, cur.pos);
      String value = readOptString(seg, cur.pos + 4);
      if (value == null) {
        throw new IllegalStateException("nameparser-ffi: epithetQualifier entry decoded as absent");
      }
      eqValues[i] = value;
      cur.pos += EPITHET_QUALIFIER_ENTRY_SIZE;
    }

    CombinedAuthorship genericAuthorship = readNestedGroup(seg, cur, len);
    CombinedAuthorship specificAuthorship = readNestedGroup(seg, cur, len);

    if (cur.pos > len) {
      throw new IllegalStateException("nameparser-ffi: decode cursor (" + cur.pos
          + ") ran past the reported buffer length (" + len + ") -- codec/encoder offset mismatch");
    }

    // ---- populate the ParsedName -- setter order/choice matters, see the class doc ----
    ParsedName pn = new ParsedName();
    pn.setRank(enumByOrdinal(Rank.values(), rankOrd, "rank"));
    pn.setCode(codeOrd == ABSENT_ENUM ? null : enumByOrdinal(NomCode.values(), codeOrd, "code"));
    pn.setType(enumByOrdinal(NameType.values(), nameTypeOrd, "name_type"));
    pn.setState(enumByOrdinal(ParsedName.State.values(), stateOrd, "state"));

    pn.setCandidatus(candidatus);
    pn.setDoubtful(doubtful);
    pn.setManuscript(manuscript);
    pn.setExtinct(extinct);

    pn.setOriginalSpelling(switch (originalSpellingByte) {
      case ORIGINAL_SPELLING_FALSE -> Boolean.FALSE;
      case ORIGINAL_SPELLING_TRUE -> Boolean.TRUE;
      case ORIGINAL_SPELLING_UNKNOWN -> null;
      default -> throw new IllegalStateException(
          "nameparser-ffi: unexpected original_spelling byte " + originalSpellingByte);
    });

    // Epithet setters strip a leading '×' and call addNotho as a side effect, but the wire's
    // epithet strings are already de-'×'d by the Rust encoder, so no side effect actually fires
    // here in practice; `notho` is (re)built explicitly and exactly from notho_bits right below
    // regardless, so the final value is correct even if that assumption were ever violated.
    pn.setUninomial(strings[SLOT_UNINOMIAL]);
    pn.setGenus(strings[SLOT_GENUS]);
    pn.setInfragenericEpithet(strings[SLOT_INFRAGENERIC]);
    pn.setSpecificEpithet(strings[SLOT_SPECIFIC]);
    pn.setInfraspecificEpithet(strings[SLOT_INFRASPECIFIC]);
    pn.setCultivarEpithet(strings[SLOT_CULTIVAR]);
    pn.setPhrase(strings[SLOT_PHRASE]);
    pn.setTaxonomicNote(strings[SLOT_TAXONOMIC_NOTE]);
    pn.setNomenclaturalNote(strings[SLOT_NOMENCLATURAL_NOTE]);
    pn.setPublishedIn(strings[SLOT_PUBLISHED_IN]); // auto-derives publishedInYear
    pn.setPublishedInYear(publishedInYear == ABSENT_ENUM ? null : publishedInYear); // ...pinned exactly here
    pn.setPublishedInPage(strings[SLOT_PUBLISHED_IN_PAGE]);
    pn.setUnparsed(strings[SLOT_UNPARSED]);
    pn.setSanctioningAuthor(strings[SLOT_SANCTIONING_AUTHOR]);

    for (int i = 0; i < NamePart.values().length; i++) {
      if ((nothoBits & (1 << i)) != 0) {
        pn.addNotho(NamePart.values()[i]);
      }
    }

    for (int i = 0; i < eqCount; i++) {
      pn.setEpithetQualifier(enumByOrdinal(NamePart.values(), eqParts[i], "epithetQualifier namePart"), eqValues[i]);
    }

    pn.setCombinationAuthorship(
        authorship(authorsComb, exAuthorsComb, strings[SLOT_YEAR_COMB], strings[SLOT_IMPRINT_YEAR_COMB]));
    pn.setBasionymAuthorship(
        authorship(authorsBas, exAuthorsBas, strings[SLOT_YEAR_BAS], strings[SLOT_IMPRINT_YEAR_BAS]));

    if (genericAuthorship != null) {
      pn.setGenericAuthorship(genericAuthorship);
    }
    if (specificAuthorship != null) {
      pn.setSpecificAuthorship(specificAuthorship);
    }

    if (!warnings.isEmpty()) {
      pn.addWarning(warnings.toArray(new String[0]));
    }

    return pn;
  }

  // ================================================================================================
  // Byte-level primitives
  // ================================================================================================

  /** Mutable byte cursor -- the run-slots and nested authorship groups are sequential/
   *  self-delimiting (not directory-addressed), so decoding them means walking forward and
   *  remembering where the last read left off; see the class doc / layout.rs's own doc comment. */
  private static final class Cursor {
    long pos;

    Cursor(long pos) {
      this.pos = pos;
    }
  }

  /** Resolves one {@code (offset, len)} string ref at {@code pos}, honoring the absent
   *  sentinel. Does not advance any cursor -- callers add {@link #STRING_REF_SIZE} themselves. */
  private static String readOptString(MemorySegment seg, long pos) {
    int offset = seg.get(LE_INT, pos);
    if (offset == ABSENT_STRING_OFFSET) {
      return null;
    }
    long off = Integer.toUnsignedLong(offset);
    long strLen = Integer.toUnsignedLong(seg.get(LE_INT, pos + 4));
    byte[] bytes = seg.asSlice(off, strLen).toArray(ValueLayout.JAVA_BYTE);
    return new String(bytes, StandardCharsets.UTF_8);
  }

  /** Reads one run-slot table ({@code u32 count} then {@code count} plain string refs) at
   *  {@code cur.pos}, advancing it past the table. {@code slot} labels it for diagnostics.
   *  Bounds the wire {@code count} against the bytes remaining ({@code len}) BEFORE it sizes the
   *  {@code ArrayList} -- a corrupt/truncated buffer with a bogus huge count is rejected here,
   *  not turned into a massive pre-allocation first. Run-slot entries are never the absent
   *  sentinel (they are real list elements) -- decoding one as absent is a wire-format defect,
   *  not a legitimate value, so it throws rather than silently inserting a null/blank. */
  private static List<String> readStringRun(MemorySegment seg, Cursor cur, int len, String slot) {
    int count = seg.get(LE_INT, cur.pos);
    cur.pos += 4;
    checkCount(slot, count, STRING_REF_SIZE, cur.pos, len);
    List<String> out = new ArrayList<>(count);
    for (int i = 0; i < count; i++) {
      String s = readOptString(seg, cur.pos);
      cur.pos += STRING_REF_SIZE;
      if (s == null) {
        throw new IllegalStateException(
            "nameparser-ffi: run-slot string entry (" + slot + ") decoded as the absent sentinel");
      }
      out.add(s);
    }
    return out;
  }

  /** Validates a wire-read {@code count} of fixed-size entries fits the bytes remaining in the
   *  reported buffer BEFORE it is used to size any allocation. {@code entryBytes} is the minimum
   *  on-wire size of one entry ({@link #STRING_REF_SIZE} for a plain string ref,
   *  {@link #EPITHET_QUALIFIER_ENTRY_SIZE} for an epithetQualifier entry); {@code pos} is the
   *  cursor AFTER the 4-byte count field was consumed; {@code len} is the reported buffer length.
   *  A negative {@code count} (a genuine {@code u32} &gt; {@code Integer.MAX_VALUE} read back as a
   *  signed {@code int}) is likewise rejected. Throwing here turns a corrupt/truncated buffer into
   *  a clear diagnostic instead of an {@code OutOfMemoryError} from a bogus huge allocation. */
  private static void checkCount(String slot, int count, long entryBytes, long pos, int len) {
    long remaining = len - pos;
    if (count < 0 || (long) count * entryBytes > remaining) {
      throw new IllegalStateException("corrupt struct buffer: " + slot + " count " + count
          + " exceeds remaining " + remaining + " byte(s)");
    }
  }

  /** Looks up an enum constant by its wire ordinal, throwing the class's standard
   *  clear-diagnostic {@code IllegalStateException} (rather than a bare {@code
   *  ArrayIndexOutOfBoundsException}) if the ordinal is out of range -- the enum-ordinal guard in
   *  this class's static initializer makes an out-of-range ordinal from a non-corrupt buffer
   *  impossible, so reaching this throw means a corrupt buffer or an undetected ABI drift. */
  private static <E extends Enum<E>> E enumByOrdinal(E[] values, int ordinal, String label) {
    if (ordinal < 0 || ordinal >= values.length) {
      throw new IllegalStateException("corrupt struct buffer: " + label + " ordinal " + ordinal
          + " out of range [0," + values.length + ")");
    }
    return values[ordinal];
  }

  /** Reads one nested authorship group ({@code generic_authorship}/{@code specific_authorship})
   *  at {@code cur.pos}, advancing past it: a {@code present} flag, and -- only if present --
   *  four run-slot tables and five fixed string refs, reconstructed here as a whole {@link
   *  CombinedAuthorship}. Returns {@code null} for an absent group. */
  private static CombinedAuthorship readNestedGroup(MemorySegment seg, Cursor cur, int len) {
    int present = seg.get(LE_INT, cur.pos);
    cur.pos += 4;
    if (present == GROUP_ABSENT) {
      return null;
    }
    if (present != GROUP_PRESENT) {
      throw new IllegalStateException(
          "nameparser-ffi: nested authorship group present flag=" + present + ", expected 0 or 1");
    }

    List<String> authorsComb = readStringRun(seg, cur, len, "nested combination authors");
    List<String> exAuthorsComb = readStringRun(seg, cur, len, "nested combination ex-authors");
    List<String> authorsBas = readStringRun(seg, cur, len, "nested basionym authors");
    List<String> exAuthorsBas = readStringRun(seg, cur, len, "nested basionym ex-authors");

    String yearComb = readOptString(seg, cur.pos);
    cur.pos += STRING_REF_SIZE;
    String imprintYearComb = readOptString(seg, cur.pos);
    cur.pos += STRING_REF_SIZE;
    String yearBas = readOptString(seg, cur.pos);
    cur.pos += STRING_REF_SIZE;
    String imprintYearBas = readOptString(seg, cur.pos);
    cur.pos += STRING_REF_SIZE;
    String sanctioningAuthor = readOptString(seg, cur.pos);
    cur.pos += STRING_REF_SIZE;

    CombinedAuthorship ca = new CombinedAuthorship();
    ca.setCombinationAuthorship(authorship(authorsComb, exAuthorsComb, yearComb, imprintYearComb));
    ca.setBasionymAuthorship(authorship(authorsBas, exAuthorsBas, yearBas, imprintYearBas));
    ca.setSanctioningAuthor(sanctioningAuthor);
    return ca;
  }

  /** Builds an {@link Authorship} via its plain setters -- NOT {@code addAuthor}/{@code
   *  addExAuthor}, which have an inverted-blank-check bug making them no-ops for real authors. */
  private static Authorship authorship(List<String> authors, List<String> exAuthors, String year, String imprintYear) {
    Authorship a = new Authorship();
    a.setAuthors(authors);
    a.setExAuthors(exAuthors);
    a.setYear(year);
    a.setImprintYear(imprintYear);
    return a;
  }
}
