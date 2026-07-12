import org.gbif.nameparser.NameParserImpl;
import org.gbif.nameparser.api.NameParser;
import org.gbif.nameparser.api.ParsedName;
import org.gbif.nameparser.util.NameFormatter;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;

/**
 * Formatter oracle: reads scientific names (one per line) on stdin, parses each with the
 * real Java {@link NameParserGBIF}, applies the five public {@link NameFormatter} renderings,
 * and prints one TSV row per name:
 *
 *   input \t ok \t canonical \t canonicalWithoutAuthorship \t canonicalMinimal \t canonicalComplete \t authorshipComplete
 *
 * `ok` is true when the name parsed; false (all rendering columns empty) when it was
 * unparsable. Tabs/newlines/backslashes in any field are backslash-escaped so every row is a
 * single physical line the Rust `format_golden.rs` harness can split on raw '\t'.
 */
public class FormatOracle {
  public static void main(String[] args) throws Exception {
    NameParser parser = new NameParserImpl();
    BufferedReader br = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
    PrintStream out = new PrintStream(System.out, true, "UTF-8");
    String line;
    while ((line = br.readLine()) != null) {
      String canonical = "", woAuth = "", minimal = "", complete = "", authorship = "";
      boolean ok = false;
      try {
        ParsedName pn = parser.parse(line, null, null, null);
        ok = true;
        canonical = nz(NameFormatter.canonical(pn));
        woAuth = nz(NameFormatter.canonicalWithoutAuthorship(pn));
        minimal = nz(NameFormatter.canonicalMinimal(pn));
        complete = nz(NameFormatter.canonicalComplete(pn));
        authorship = nz(NameFormatter.authorshipComplete(pn));
      } catch (Exception e) {
        ok = false;
      }
      out.println(esc(line) + "\t" + ok + "\t" + esc(canonical) + "\t" + esc(woAuth)
          + "\t" + esc(minimal) + "\t" + esc(complete) + "\t" + esc(authorship));
    }
  }

  static String nz(String s) {
    return s == null ? "" : s;
  }

  static String esc(String s) {
    return s.replace("\\", "\\\\").replace("\t", "\\t").replace("\r", "\\r").replace("\n", "\\n");
  }
}
