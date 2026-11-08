// CodeMirror keeps "\n" line ends: a file's own ("\r\n" on Windows) are put back when it is saved.

export type Eol = "\n" | "\r\n";

/** A file's text as the editor holds it ("\n" line ends), and the line ends most of its lines had. */
export function fromDisk(text: string): { text: string; eol: Eol } {
  const crlf = text.split("\r\n").length - 1;
  if (!crlf) return { text, eol: "\n" };
  const lf = text.split("\n").length - 1;
  return { text: text.replace(/\r\n/g, "\n"), eol: crlf * 2 > lf ? "\r\n" : "\n" };
}

/** The editor's text with the file's line ends, to write. */
export function toDisk(text: string, eol: Eol): string {
  return eol === "\r\n" ? text.replace(/\n/g, "\r\n") : text;
}
