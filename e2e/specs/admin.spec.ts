import { expect, test } from "@playwright/test";
import { apiProject, apiSend } from "../lib/api";
import { authFile } from "../lib/roles";

test.use({ storageState: authFile("admin") });

test("a limit can be discarded, then saved, and holds after a reload", async ({ page }) => {
  await page.goto("/admin/limits");
  const cap = page.locator("#limit-tickets_per_project_per_day");
  const before = await cap.inputValue();
  await cap.fill("150");
  await page.getByRole("button", { name: "Discard changes" }).click();
  await expect(cap).toHaveValue(before);
  await cap.fill("150");
  // A reload before the save's answer would read the old value.
  const saved = page.waitForResponse((r) => r.url().endsWith("/api/ui/admin/limits") && r.request().method() === "PUT");
  await page.getByRole("button", { name: "Save limits" }).click();
  expect((await saved).status()).toBe(204);
  await page.reload();
  await expect(cap).toHaveValue("150");
  // Other specs share this server: put the limit back.
  await cap.fill(before);
  await page.getByRole("button", { name: "Save limits" }).click();
  await expect(page.getByRole("button", { name: "Save limits" })).toBeDisabled();
});

test("the time zone moves every time shown, until it is set back", async ({ page }) => {
  await page.goto("/admin/time-zone");
  await page.locator("#tz").click();
  await page.getByRole("option", { name: /Jakarta/ }).click();
  await page.locator("#time-zone").getByRole("button", { name: "Save changes" }).click();
  await expect(page.locator("#time-zone")).toContainText(/Now it is .* UTC\+7\./);
  await apiSend(page, "PUT", "/api/ui/admin/time-zone", { utc_offset_min: 0 }, 204);
});

test("Test connection names the mail server that did not answer", async ({ page }) => {
  await page.goto("/admin/email");
  await page.locator("#mail-host").fill("127.0.0.1");
  await page.locator("#mail-port").fill("9");
  await page.locator("#mail-from").fill("Monita <monita@monita.test>");
  await page.getByRole("button", { name: "Test connection" }).click();
  await expect(page.getByRole("status")).toContainText("Monita could not connect.", { timeout: 15_000 });
});

test("with no mail server saved, a test email goes to the log and says so", async ({ page }) => {
  await page.goto("/admin/email");
  await page.locator("#test-to").fill("oncall@monita.test");
  await page.getByRole("button", { name: "Send test email" }).click();
  await expect(page.locator("#test-send").getByRole("status")).toContainText("No mail server is set, so the test went to Monita's log.");
});

test("a supervisor is invited, and a project owner cannot become one", async ({ page }) => {
  const email = `sup-${Date.now()}@monita.test`;
  await page.goto("/admin/users");
  await page.getByRole("button", { name: "Invite new user" }).click();
  await page.locator("#invite-email").fill(email);
  await page.getByRole("radio", { name: /Supervisor/ }).click();
  await page.getByRole("button", { name: "Send invite" }).click();
  await expect(page.getByRole("status")).toContainText(`Invite sent to ${email}`);
  await expect(page.getByRole("row", { name: new RegExp(email) }).getByText("Supervisor")).toBeVisible();

  // tl@monita.test owns at least this project, so the role change is refused.
  await apiSend(page, "POST", "/api/ui/projects", { name: "Owned", homepage_url: "https://app.example.com", owner_email: "tl@monita.test" }, 201);
  await page.getByRole("button", { name: "Actions for tl@monita.test" }).click();
  await page.getByRole("menuitem", { name: "Make supervisor" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Make supervisor" }).click();
  await expect(page.getByText(/owns projects/)).toBeVisible();
  await expect(page.getByRole("row", { name: /tl@monita\.test/ }).getByText("Tech lead")).toBeVisible();
});

test("Admin in the header opens the admin page opened last", async ({ page }) => {
  await page.goto("/admin/email");
  await expect(page.getByRole("heading", { name: "Outgoing email" })).toBeVisible();
  await page.goto("/");
  await page.getByRole("link", { name: "Admin", exact: true }).click();
  await expect(page).toHaveURL(/\/admin\/email$/);
});

test("a location is added with its token shown once, a monitor checks from it, and it is deleted", async ({ page }) => {
  const name = `Jakarta ${Date.now() % 100000}`;
  await page.goto("/admin/locations");
  await page.getByRole("button", { name: "Add location" }).click();
  await page.getByLabel("Name").fill(name);
  await page.getByRole("dialog").getByRole("button", { name: "Add location" }).click();
  await expect(page.getByText(`${name} is ready`)).toBeVisible();
  await expect(page.locator("#probe-command")).toContainText("MONITA_PROBE_TOKEN='ml_");
  const row = page.getByRole("row").filter({ hasText: name });
  await expect(row).toContainText("Never");

  const slug = await apiProject(page, "Places");
  await page.goto(`/projects/${slug}/monitors/new`);
  await page.locator("#m-name").fill("Checkout");
  await page.locator("#m-url").fill("http://127.0.0.1:9/");
  await page.getByLabel(name).check();
  await page.locator("#m-quorum").fill("2");
  await page.getByRole("button", { name: "Save monitor" }).click();
  await expect(page.getByText("Places: a check fails when 2 of 2 fail")).toBeVisible();
  await expect(page.getByRole("cell", { name, exact: true })).toBeVisible();

  await page.goto("/admin/locations");
  await page.getByRole("button", { name: `Delete ${name}` }).click();
  await page.getByRole("button", { name: "Delete location" }).click();
  await expect(row).toHaveCount(0);
});

test("the Telegram page refuses a token that is not a bot token, before asking Telegram", async ({ page }) => {
  await page.goto("/admin/telegram");
  await expect(page.getByRole("heading", { name: "Telegram", level: 1 })).toBeVisible();
  await page.getByLabel("Bot token").fill("not-a-token");
  await page.getByRole("button", { name: "Save token" }).click();
  await expect(page.getByText("Telegram refused this token, so copy it again from @BotFather.")).toBeVisible();
});
