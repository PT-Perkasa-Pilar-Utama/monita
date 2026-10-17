import { expect, test } from "bun:test";
import { highlight } from "./highlight";

const strip = (html: string) => html.replace(/<[^>]+>/g, "");

test("a shell line keeps its text and marks the command, flags, strings and variables", () => {
  const src = `curl -fsS -m 10 --data-binary @- "$URL/fail" # report`;
  const html = highlight(src.replace(/"/g, "&quot;"), "sh");
  expect(html).toContain('<span class="tok-x">curl</span>');
  expect(html).toContain('<span class="tok-f">--data-binary</span>');
  expect(html).toContain('<span class="tok-s">&quot;$URL/fail&quot;</span>'.replace(/&quot;/g, '"'));
  expect(html).toContain('<span class="tok-c"># report</span>');
  expect(strip(html)).toBe(src);
});

test("an escaped angle bracket survives highlighting", () => {
  const html = highlight("curl https://monita.example.com/ping/&lt;token&gt;", "sh");
  expect(html).toContain("&lt;token&gt;");
  expect(html).not.toContain("<token>");
});

test("a word after a pipe or then is a command, and an assignment marks only its name", () => {
  const html = highlight("URL=https://x\nif ! out=$(backup.sh 2&gt;&amp;1); then\n  printf x | curl -s", "sh");
  expect(html).toContain('<span class="tok-v">URL</span>=https://x');
  expect(html).toContain('<span class="tok-x">backup.sh</span>');
  expect(html).toContain('<span class="tok-x">printf</span>');
  expect(html).toContain('<span class="tok-x">curl</span>');
});

test("a continuation line does not start a new command", () => {
  const html = highlight("curl -s https://x \\\n  -H 'a: b'", "sh");
  expect(html).not.toContain('<span class="tok-x">-H</span>');
  expect(html).toContain('<span class="tok-f">-H</span>');
});

test("json keys and values get different classes", () => {
  const html = highlight('{ &quot;event&quot;: &quot;incident.opened&quot;, &quot;at&quot;: 1792137600 }', "json");
  expect(html).toContain('<span class="tok-p">"event"</span>');
  expect(html).toContain('<span class="tok-s">"incident.opened"</span>');
  expect(html).toContain('<span class="tok-n">1792137600</span>');
});

test("an unknown language comes back unchanged", () => {
  expect(highlight("a &lt; b", "text")).toBe("a &lt; b");
});

test("a crontab schedule is not a command, and the program after it is", () => {
  const html = highlight("0 2 * * * /opt/jobs/backup.sh && curl -fsS https://x", "sh");
  expect(html).toContain('<span class="tok-x">/opt/jobs/backup.sh</span>');
  expect(html).not.toContain('<span class="tok-x">0</span>');
});
