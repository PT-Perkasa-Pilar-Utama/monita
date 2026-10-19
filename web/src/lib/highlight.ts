// A small syntax highlighter for the snippets Monita shows: shell, JavaScript and Python. It marks
// comments, strings, keywords, shell flags and numbers; everything else stays plain.
// ponytail: a regex scan, not a parser; a string holding a comment marker is still read right
// because strings and comments are matched in one pass, left to right.

export type Lang = "sh" | "js" | "py";
export type Kind = "plain" | "comment" | "string" | "keyword" | "flag" | "number";
export type Token = { text: string; kind: Kind };

const KEYWORDS: Record<Lang, string[]> = {
  sh: ["if", "then", "else", "elif", "fi", "for", "do", "done", "while", "case", "esac", "in", "function", "return", "exit"],
  js: ["const", "let", "var", "await", "async", "function", "return", "try", "catch", "finally", "throw", "new", "if", "else", "import", "from", "export"],
  py: ["import", "from", "try", "except", "finally", "raise", "def", "return", "if", "elif", "else", "for", "in", "while", "with", "as", "None", "True", "False"],
};

const COMMENT: Record<Lang, string> = { sh: "#[^\\n]*", js: "//[^\\n]*", py: "#[^\\n]*" };
const STRING = `"(?:\\\\.|[^"\\\\\\n])*"|'(?:\\\\.|[^'\\\\\\n])*'`;
const TEMPLATE = "`(?:\\\\.|[^`\\\\])*`";

function pattern(lang: Lang): RegExp {
  const parts = [
    `(?<comment>${COMMENT[lang]})`,
    `(?<string>${lang === "js" ? `${STRING}|${TEMPLATE}` : STRING})`,
    // A bare URL stays plain, so its port is not a number.
    "(?<plain>https?://[^\\s\"'`]+)",
    ...(lang === "sh" ? ["(?<flag>(?<=\\s)--?[A-Za-z][\\w-]*)"] : []),
    `(?<keyword>\\b(?:${KEYWORDS[lang].join("|")})\\b)`,
    "(?<number>\\b\\d[\\d_]*\\b)",
  ];
  return new RegExp(parts.join("|"), "g");
}

/** The code as tokens; joined, their text is the code unchanged. */
export function highlight(code: string, lang: Lang): Token[] {
  const out: Token[] = [];
  const push = (text: string, kind: Kind): void => {
    if (!text) return;
    const last = out[out.length - 1];
    if (last && last.kind === kind) last.text += text;
    else out.push({ text, kind });
  };
  let at = 0;
  for (const m of code.matchAll(pattern(lang))) {
    push(code.slice(at, m.index), "plain");
    const kind = (Object.entries(m.groups ?? {}).find(([, v]) => v !== undefined)?.[0] ?? "plain") as Kind;
    push(m[0], kind);
    at = m.index + m[0].length;
  }
  push(code.slice(at), "plain");
  return out;
}
