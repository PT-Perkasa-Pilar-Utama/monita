import { expect, test } from "bun:test";
import { envName, headerText, parseHeaderLines, signInWords, targetField, whenItFails } from "@/lib/checks";

test("each pair of failure boxes has its own sentence", () => {
  expect(whenItFails(true, true)).toBe("Sends alerts and opens an incident");
  expect(whenItFails(true, false)).toBe("Sends alerts only");
  expect(whenItFails(false, true)).toBe("Opens an incident only");
  expect(whenItFails(false, false)).toBe("Nothing: Monita only records it");
});

test("header lines become pairs, blank and colon-less lines skipped", () => {
  expect(parseHeaderLines("Authorization: Bearer a:b\n\nno colon\nX-Env: prod")).toEqual([
    ["Authorization", "Bearer a:b"],
    ["X-Env", "prod"],
  ]);
});

test("header rows become lines; empty rows drop, half rows stay for the server to refuse", () => {
  expect(headerText([{ name: " X-Env ", value: "prod " }, { name: "", value: "" }, { name: "", value: "orphan" }])).toBe("X-Env: prod\n: orphan");
  expect(headerText([])).toBe("");
});

test("a secret name becomes a shell variable", () => {
  expect(envName("api-token")).toBe("API_TOKEN");
  expect(envName("2fa")).toBe("SECRET_2FA");
  expect(envName(null)).toBe("SECRET");
});

test("sign-in words name the secret, never a value", () => {
  const m = { auth_kind: "none", auth_secret_name: "API_TOKEN", auth_user: "svc", auth_header: "X-Api-Key" };
  expect(signInWords(m)).toBe("None");
  expect(signInWords({ ...m, auth_kind: "bearer" })).toBe("Bearer token from secret API_TOKEN");
  expect(signInWords({ ...m, auth_kind: "basic" })).toBe("User svc, password from secret API_TOKEN");
  expect(signInWords({ ...m, auth_kind: "header", auth_secret_name: null })).toBe("X-Api-Key header from secret (deleted)");
});

test("the target field names what each kind of check needs", () => {
  expect(targetField("http").label).toBe("URL");
  expect(targetField("tcp")).toEqual({ label: "Host and port", placeholder: "db.internal:5432" });
  expect(targetField("dns").label).toBe("Host name");
});
