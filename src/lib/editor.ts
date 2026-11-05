// CodeMirror's look in the Darkroom palette: code on the darkest surface,
// keywords in tungsten amber, strings in the safelight green.
import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

export const darkroomTheme = EditorView.theme(
  {
    "&": { height: "100%", backgroundColor: "var(--canvas)", color: "var(--text)" },
    ".cm-scroller": { fontFamily: "var(--mono)", lineHeight: "1.6" },
    ".cm-content": { caretColor: "var(--accent)", padding: "8px 0" },
    ".cm-gutters": { backgroundColor: "var(--canvas)", color: "#5d5547", border: "none", paddingRight: "4px" },
    ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--muted)" },
    ".cm-activeLine": { backgroundColor: "color-mix(in srgb, var(--panel-2) 55%, transparent)" },
    ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)", borderLeftWidth: "2px" },
    "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
      backgroundColor: "color-mix(in srgb, var(--accent) 24%, transparent) !important",
    },
    ".cm-selectionMatch": { backgroundColor: "color-mix(in srgb, var(--accent) 12%, transparent)" },
    ".cm-searchMatch": { backgroundColor: "color-mix(in srgb, var(--accent) 30%, transparent)", outline: "none" },
    ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "color-mix(in srgb, var(--accent) 55%, transparent)" },
    ".cm-matchingBracket, .cm-nonmatchingBracket": { backgroundColor: "color-mix(in srgb, var(--ok) 22%, transparent)", outline: "none" },
    ".cm-foldPlaceholder": { backgroundColor: "var(--panel-2)", border: "1px solid var(--line)", color: "var(--muted)" },
    ".cm-panels": { backgroundColor: "var(--panel)", color: "var(--text)", borderColor: "var(--line)" },
    ".cm-panels.cm-panels-top": { borderBottom: "1px solid var(--line)" },
    ".cm-panel input, .cm-panel button": { fontFamily: "var(--sans)", fontSize: "12px" },
    ".cm-textfield": { backgroundColor: "var(--bg)", border: "1px solid var(--line)", borderRadius: "5px", color: "var(--text)" },
    ".cm-button": { backgroundImage: "none", backgroundColor: "var(--panel-2)", border: "1px solid var(--line)", borderRadius: "5px", color: "var(--text)" },
    ".cm-tooltip": { backgroundColor: "var(--panel-2)", border: "1px solid var(--line)", borderRadius: "6px", color: "var(--text)" },
    ".cm-tooltip-autocomplete > ul > li[aria-selected]": { backgroundColor: "var(--selection)", color: "var(--text)" },
  },
  { dark: true },
);

const style = HighlightStyle.define([
  { tag: [t.keyword, t.controlKeyword, t.moduleKeyword, t.definitionKeyword, t.modifier], color: "#f0a83a" },
  { tag: [t.string, t.special(t.string), t.regexp], color: "#a9bd74" },
  { tag: [t.number, t.bool, t.null, t.atom], color: "#e27b57" },
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], color: "#7a705f", fontStyle: "italic" },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.macroName], color: "#f2d29b" },
  { tag: [t.typeName, t.className, t.namespace, t.definition(t.typeName)], color: "#c4b5fd" },
  { tag: [t.propertyName, t.attributeName], color: "#e3c48f" },
  { tag: [t.tagName, t.angleBracket], color: "#e78a6a" },
  { tag: [t.operator, t.punctuation, t.separator, t.bracket], color: "#b0a690" },
  { tag: [t.variableName, t.definition(t.variableName)], color: "#ede6d8" },
  { tag: [t.heading, t.strong], color: "#f0a83a", fontWeight: "600" },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: [t.link, t.url], color: "#a9bd74", textDecoration: "underline" },
  { tag: t.meta, color: "#9a917f" },
  { tag: t.invalid, color: "#e5484d" },
]);

export const darkroomHighlight = syntaxHighlighting(style);
