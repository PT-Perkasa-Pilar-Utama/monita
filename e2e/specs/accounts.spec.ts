import { createHmac } from "node:crypto";
import { type Browser, type BrowserContext, expect, type Page, test } from "@playwright/test";
import { apiSend } from "../lib/api";
import { authFile } from "../lib/roles";

/** The RFC 6238 code an authenticator app shows for `secret` at Unix second `at`. */
function totp(secret: string, at: number): string {
  const bits = [...secret].map((c) => "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".indexOf(c).toString(2).padStart(5, "0")).join("");
  const key = Buffer.from(bits.match(/.{8}/g)!.map((b) => parseInt(b, 2)));
  const step = Buffer.alloc(8);
  step.writeBigUInt64BE(BigInt(Math.floor(at / 30)));
  const h = createHmac("sha1", key).update(step).digest();
  return String((h.readUInt32BE(h[19]! & 15) & 0x7fffffff) % 1e6).padStart(6, "0");
}

test.describe("as the super admin", () => {
  test.use({ storageState: authFile("admin") });

  test("@US-02 @US-03 super admin invites a TL, who sets a password and lands on projects", async ({ page, browser }) => {
    const email = `tl-${Date.now()}@monita.test`;
    await page.goto("/admin/users");
    await page.getByRole("button", { name: "Invite new user" }).click();
    await page.locator("#invite-email").fill(email);
    await page.getByRole("button", { name: "Send invite" }).click();
    await expect(page.getByRole("status")).toContainText(`Invite sent to ${email}`);
    const link = (await page.locator("#invite-link").textContent())!.trim();

    const tl = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const tlPage = await tl.newPage();
    await tlPage.goto(link);
    await tlPage.getByLabel("New password").fill("a-long-enough-password");
    await tlPage.getByLabel("Repeat password").fill("a-long-enough-password");
    await tlPage.getByRole("button", { name: "Save password" }).click();
    await expect(tlPage.getByRole("heading", { name: "Projects", exact: true })).toBeVisible({ timeout: 10_000 });
    await expect(tlPage.getByRole("button", { name: `Account: ${email}` })).toBeVisible();

    await tlPage.goto(link);
    await expect(tlPage.getByRole("heading", { name: "Link not valid" })).toBeVisible();
    await tl.close();
  });

  test("a user turns two-factor on, saves recovery codes, and signs in with a code", async ({ page, browser }) => {
    const email = `mfa-${Date.now()}@monita.test`;
    await page.goto("/admin/users");
    await page.getByRole("button", { name: "Invite new user" }).click();
    await page.locator("#invite-email").fill(email);
    await page.getByRole("button", { name: "Send invite" }).click();
    const link = (await page.locator("#invite-link").textContent())!.trim();

    const ctx = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const me = await ctx.newPage();
    await me.goto(link);
    await me.getByLabel("New password").fill("a-long-enough-password");
    await me.getByLabel("Repeat password").fill("a-long-enough-password");
    await me.getByRole("button", { name: "Save password" }).click();
    const note = me.getByRole("note", { name: "Two-factor sign-in" });
    await expect(note).toContainText("Two-factor sign-in is off");
    await me.getByRole("button", { name: `Account: ${email}` }).click();
    await me.getByRole("menuitem", { name: "User settings" }).click();
    await expect(me).toHaveURL(/\/account\/profile$/);
    await me.getByRole("navigation", { name: "User settings" }).getByRole("link", { name: "Two-factor sign-in" }).click();
    await expect(me).toHaveURL(/\/account\/two-factor$/);
    await me.getByRole("button", { name: "Turn on" }).click();
    await expect(me.getByRole("img", { name: "QR code for your authenticator app" })).toBeVisible();
    const secret = (await me.locator("#totp-secret").textContent())!.trim();
    const at = Date.now() / 1000;
    await me.getByPlaceholder("Code from your app").fill(totp(secret, at));
    await me.getByRole("button", { name: "Turn on" }).click();
    await expect(me.locator("#recovery-codes li")).toHaveCount(10);
    await me.getByRole("button", { name: "I saved them" }).click();
    await expect(me.getByText("Two-factor sign-in is on")).toBeVisible();
    await me.goto("/");
    await expect(note, "the note goes once two-factor is on").toHaveCount(0);
    await ctx.clearCookies();

    await me.goto("/login");
    await me.getByLabel("Email").fill(email);
    await me.getByLabel("Password").fill("a-long-enough-password");
    await me.getByRole("button", { name: "Sign in" }).click();
    await expect(me.getByRole("heading", { name: "Enter your code" })).toBeVisible();
    // The confirm used this step's code, and no code works twice.
    await me.getByLabel("Code").fill(totp(secret, at + 30));
    await me.getByRole("button", { name: "Verify" }).click();
    await expect(me.getByRole("heading", { name: "Projects", exact: true })).toBeVisible({ timeout: 10_000 });

    await page.reload();
    await expect(page.getByRole("row", { name: new RegExp(email) }).getByText("Two-factor")).toBeVisible();
    await ctx.close();
  });
});

test.describe("as a new tech lead", () => {
  test.use({ storageState: authFile("admin") });

  /** A new tech lead, signed in through their invite in a browser of their own. */
  async function newUser(page: Page, browser: Browser, prefix: string): Promise<{ me: Page; ctx: BrowserContext; email: string }> {
    const email = `${prefix}-${Date.now()}@monita.test`;
    const { link } = await apiSend<{ link: string }>(page, "POST", "/api/ui/admin/users", { email, role: "tl" }, 201);
    const ctx = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const me = await ctx.newPage();
    await me.goto(link);
    await me.getByLabel("New password").fill("a-long-enough-password");
    await me.getByLabel("Repeat password").fill("a-long-enough-password");
    await me.getByRole("button", { name: "Save password" }).click();
    await expect(me.getByRole("heading", { name: "Projects", exact: true })).toBeVisible();
    return { me, ctx, email };
  }

  test("a user changes their password in User settings", async ({ page, browser }) => {
    const { me, ctx, email } = await newUser(page, browser, "pw");

    await me.goto("/account/password");
    await me.locator("#pw-current").fill("not-the-password");
    await me.locator("#pw-new").fill("another-long-password");
    await me.locator("#pw-repeat").fill("another-long-password");
    await me.getByRole("button", { name: "Change password" }).click();
    await expect(me.getByText("This is not your current password.")).toBeVisible();
    await me.locator("#pw-current").fill("a-long-enough-password");
    await me.getByRole("button", { name: "Change password" }).click();
    await expect(me.getByText("Password changed")).toBeVisible();

    await ctx.clearCookies();
    await me.goto("/login");
    await me.getByLabel("Email").fill(email);
    await me.getByLabel("Password").fill("another-long-password");
    await me.getByRole("button", { name: "Sign in" }).click();
    await expect(me.getByRole("heading", { name: "Projects", exact: true })).toBeVisible();
    await ctx.close();
  });

  test("a user sets and removes their picture in User settings", async ({ page, browser }) => {
    const { me, ctx } = await newUser(page, browser, "pic");
    await me.goto("/account");
    await expect(me).toHaveURL(/\/account\/profile$/);
    const png = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==", "base64");
    await me.getByLabel("Picture").setInputFiles({ name: "me.png", mimeType: "image/png", buffer: png });
    await me.getByRole("button", { name: "Save picture" }).click();
    await expect(me.getByText("Picture saved.")).toBeVisible();
    const avatar = me.getByRole("button", { name: /^Account:/ }).locator("img");
    await expect(avatar).toHaveAttribute("src", /\/users\/\d+\/avatar\?v=\d+/);
    await me.getByRole("button", { name: "Remove picture" }).click();
    await expect(me.getByText("Picture removed.")).toBeVisible();
    await expect(avatar).toHaveCount(0);
    await ctx.close();
  });
});

test.describe("as a tech lead", () => {
  test.use({ storageState: authFile("tl") });

  test("the two-factor note stays closed in this browser once closed", async ({ page }) => {
    await page.goto("/");
    const note = page.getByRole("note", { name: "Two-factor sign-in" });
    await expect(note).toBeVisible();
    await page.getByRole("button", { name: "Close the two-factor note" }).click();
    await expect(note).toHaveCount(0);
    await page.reload();
    await expect(page.getByRole("heading", { name: "Projects", exact: true })).toBeVisible();
    await expect(note).toHaveCount(0);
  });
});

test("@US-03 a wrong password shows one generic error", async ({ page }) => {
  await page.goto("/login");
  await page.getByLabel("Email").fill("tl@monita.test");
  await page.getByLabel("Password").fill("not-the-password");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("alert")).toHaveText("Wrong email or password.");
});
