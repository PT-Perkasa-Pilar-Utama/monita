import { expect, test } from "bun:test";
import { OFF, assetLinks, fileFor, pageLinks, relative, relativeFonts, rewrite } from "./demo-site";

test("a page is saved as its folder's index.html", () => {
  expect(fileFor("/")).toBe("index.html");
  expect(fileFor("/projects/portal/monitors")).toBe("projects/portal/monitors/index.html");
});

test("links are relative to the page's folder", () => {
  expect(relative("/", "/status/portal", true)).toBe("status/portal/");
  expect(relative("/projects/portal/monitors", "/", true)).toBe("../../../");
  expect(relative("/projects/portal/monitors", "/projects/portal/monitors/2", true)).toBe("../../../projects/portal/monitors/2/");
  expect(relative("/status/portal", "/static/app.css", false)).toBe("../../static/app.css");
  expect(relative("/", "/", true)).toBe("./");
});

test("the crawler follows plain hrefs only", () => {
  const html = `<a href="/a">a</a><a href="/b?before=1.2">next</a><form action="/c"></form>
    <link href="/static/app.css?v=1"><a href="https://monita.example.com/status/x">x</a><a href="https://other.example.com/y">y</a>`;
  expect(pageLinks(html).sort()).toEqual(["/a", "/status/x"]);
  expect(assetLinks(html)).toEqual(["/static/app.css"]);
});

test("a rewrite keeps crawled pages and assets and turns everything else off", () => {
  const html = `<a href="/projects/portal/monitors">m</a><a href="/missing">x</a><a href="/a?days=7">q</a>
    <form action="/logout"></form><use href="/static/icons.svg?v=9#i-users"/><a href="https://other.example.com/">o</a>`;
  const got = rewrite(html, "/status/portal", new Set(["/projects/portal/monitors", "/a"]));
  expect(got).toBe(`<a href="../../projects/portal/monitors/">m</a><a href="${OFF}">x</a><a href="${OFF}">q</a>
    <form action="${OFF}"></form><use href="../../static/icons.svg?v=9#i-users"/><a href="https://other.example.com/">o</a>`);
});

test("the app's built files and image sources are assets, never pages", () => {
  const html = `<link href="/assets/app-1a2b.css"><img src="/projects/portal/icon?v=7"><a href="/assets/x.js">x</a>`;
  expect(pageLinks(html)).toEqual([]);
  expect(assetLinks(html).sort()).toEqual(["/assets/app-1a2b.css", "/assets/x.js", "/projects/portal/icon"]);
  expect(rewrite(`<img src="/projects/portal/icon?v=7">`, "/projects/portal", new Set())).toBe(`<img src="../../projects/portal/icon?v=7">`);
});

test("a built stylesheet finds its fonts one folder up", () => {
  expect(relativeFonts(`src:url("/static/fonts/a.woff2") url(/static/fonts/b.woff2)`)).toBe(`src:url("../static/fonts/a.woff2") url(../static/fonts/b.woff2)`);
});
