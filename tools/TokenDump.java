// SPDX-License-Identifier: Apache-2.0
import java.io.*;
import java.nio.charset.StandardCharsets;
import org.gbif.nameparser.token.Token;
import org.gbif.nameparser.token.Tokenizer;

/** Reads names (one per line; text before the first TAB is the name), emits
 *  "name<TAB>KIND:text\u001FKIND:text..." so the Rust golden test can diff token streams. */
public class TokenDump {
  public static void main(String[] args) throws Exception {
    try (BufferedReader r = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
         BufferedWriter w = new BufferedWriter(new OutputStreamWriter(System.out, StandardCharsets.UTF_8))) {
      String line;
      while ((line = r.readLine()) != null) {
        int tab = line.indexOf('\t');
        String name = tab >= 0 ? line.substring(0, tab) : line;
        if (name.isBlank() || name.startsWith("#")) continue;
        StringBuilder sb = new StringBuilder();
        for (Token t : Tokenizer.tokenize(name)) {
          if (sb.length() > 0) sb.append('\u001F');
          sb.append(t.kind).append(':').append(t.text);
        }
        w.write(name);
        w.write('\t');
        w.write(sb.toString());
        w.write('\n');
      }
    }
  }
}
