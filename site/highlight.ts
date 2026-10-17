/**
 * Build-time syntax highlighting for the site's code blocks: sh, json, yaml and ini. It reads the
 * escaped HTML that Bun.markdown writes, and wraps tokens in <span class="tok-*">.
 */

type Rule = [cls: string, re: RegExp];

const SH_KEYWORDS = new Set(["if", "then", "else", "elif", "fi", "for", "do", "done", "while", "in", "exit", "case", "esac"]);

const RULES: Record<string, Rule[]> = {
  json: [
    ["p", /"(?:\\.|[^"\\])*"(?=\s*:)/y],
    ["s", /"(?:\\.|[^"\\])*"/y],
    ["n", /-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|\b(?:true|false|null)\b/y],
  ],
  yaml: [
    ["c", /#.*/y],
    ["p", /[\w.-]+(?=:(?:\s|$))/y],
    ["s", /"(?:\\.|[^"\\])*"|'[^']*'/y],
    ["n", /\b\d+(?:\.\d+)?\b|\b(?:true|false|null)\b/y],
  ],
  ini: [
    ["c", /[#;].*/y],
    ["k", /\[[^\]\n]*\]/y],
    ["p", /[\w.-]+(?==)/y],
    ["s", /"(?:\\.|[^"\\])*"/y],
  ],
};

const unescape = (s: string) =>
  s.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&amp;/g, "&");
const escape = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
const span = (cls: string, text: string) => `<span class="tok-${cls}">${escape(text)}</span>`;

function generic(code: string, rules: Rule[]): string {
  let out = "";
  let i = 0;
  scan: while (i < code.length) {
    for (const [cls, re] of rules) {
      re.lastIndex = i;
      const m = re.exec(code);
      if (m && m[0]) {
        out += span(cls, m[0]);
        i += m[0].length;
        continue scan;
      }
    }
    out += escape(code[i]!);
    i++;
  }
  return out;
}

// A crontab schedule (0 2 * * *) is skipped. A word is a command when it starts a line or follows |, ;, &&, $( or a keyword such as then.
function shell(code: string): string {
  const re = /(#.*)|('[^']*'|"(?:\\.|[^"\\])*")|(\$\{?\w+\}?)|(--?[A-Za-z][\w-]*)|([^\s'"$#|;&()]+)/g;
  let out = "";
  let last = 0;
  let command = true;
  let continued = false;
  for (const m of code.matchAll(re)) {
    const gap = code.slice(last, m.index);
    if (gap.includes("\n")) command = !continued;
    if (/[|;(]|&&/.test(gap)) command = true;
    out += escape(gap);
    const [text, comment, str, variable, flag, word] = m;
    const atWordStart = m.index === 0 || /\s/.test(code[m.index - 1]!);
    if (comment !== undefined && atWordStart) out += span("c", text);
    else if (str !== undefined) out += span("s", text);
    else if (variable !== undefined) out += span("v", text);
    else if (flag !== undefined && atWordStart) {
      out += span("f", text);
      command = false;
    } else if (word !== undefined && SH_KEYWORDS.has(word)) {
      out += span("k", text);
      command = word !== "exit" && word !== "fi" && word !== "done" && word !== "in";
    } else if (word !== undefined && command && /^[\d*/,-]+$/.test(word)) {
      out += escape(text);
    } else if (word !== undefined && command) {
      const assign = /^(\w+)=(.*)$/.exec(word);
      if (assign) out += span("v", assign[1]!) + "=" + escape(assign[2]!);
      else {
        out += span("x", word);
        command = false;
      }
    } else out += escape(text);
    last = m.index + text.length;
    continued = text === "\\";
  }
  return out + escape(code.slice(last));
}

export function highlight(escaped: string, lang: string): string {
  const code = unescape(escaped);
  if (lang === "sh" || lang === "bash" || lang === "shell") return shell(code);
  const rules = RULES[lang];
  return rules ? generic(code, rules) : escaped;
}
