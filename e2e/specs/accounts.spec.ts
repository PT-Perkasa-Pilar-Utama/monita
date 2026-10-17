import { expect, test } from "@playwright/test";
import { authFile } from "../lib/roles";

test.describe("as the super admin", () => {
  test.use({ storageState: authFile("admin") });

  test("@US-02 @US-03 super admin invites a TL, who sets a password and lands on projects", async ({ page, browser }) => {
    const email = `tl-${Date.now()}@monita.test`;
    await page.goto("/admin/users");
    await page.getByRole("button", { name: "Invite tech lead" }).click();
    await page.getByLabel("Email").fill(email);
    await page.getByRole("button", { name: "Send invite" }).click();
    await expect(page.getByRole("status")).toContainText(`Invite queued for ${email}`);
    const link = (await page.locator("#invite-link").textContent())!.trim();

    const tl = await browser.newContext({ storageState: { cookies: [], origins: [] } });
    const tlPage = await tl.newPage();
    await tlPage.goto(link);
    await tlPage.getByLabel("New password").fill("a-long-enough-password");
    await tlPage.getByLabel("Repeat password").fill("a-long-enough-password");
    await tlPage.getByRole("button", { name: "Save password" }).click();
    await expect(tlPage.getByRole("heading", { name: "Projects", exact: true })).toBeVisible({ timeout: 10_000 });
    await expect(tlPage.getByRole("button", { name: email })).toBeVisible();

    await tlPage.goto(link);
    await expect(tlPage.getByRole("heading", { name: "Link not valid" })).toBeVisible();
    await tl.close();
  });
});

test("@US-03 a wrong password shows one generic error", async ({ page }) => {
  await page.goto("/login");
  await page.getByLabel("Email").fill("tl@monita.test");
  await page.getByLabel("Password").fill("not-the-password");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("alert")).toHaveText("Wrong email or password.");
});
