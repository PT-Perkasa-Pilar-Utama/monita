import { expect, test } from "@playwright/test";

test("@smoke health reports ok", async ({ request }) => {
  const res = await request.get("/healthz");
  expect(res.status()).toBe(200);
  expect((await res.json()).status).toBe("ok");
});

test("@smoke sign-in page renders with the design system and no CSP errors", async ({ page }) => {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.goto("/login");
  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
  await expect(page.getByLabel("Email")).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign in" })).toBeVisible();
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  expect(bg).toBe("rgb(8, 9, 10)");
  const font = await page.evaluate(async () => {
    await document.fonts.ready;
    return document.fonts.check('13px "Inter"');
  });
  expect(font).toBe(true);
  expect(errors).toEqual([]);
});

for (const [width, height, playground] of [[1280, 720, true], [375, 667, false]] as const) {
  test(`@smoke the sign-in page fits ${width}x${height} without scrolling`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    await page.goto("/login");
    await expect(page.getByRole("button", { name: "Sign in" })).toBeVisible();
    const fits = await page.evaluate(() => {
      const box = document.querySelector("main")!.parentElement!;
      return box.scrollHeight <= window.innerHeight && document.documentElement.scrollHeight <= window.innerHeight;
    });
    expect(fits).toBe(true);
    // Under 680 px high the playground steps aside so the card keeps its room.
    await expect(page.locator("main + div")).toBeVisible({ visible: playground });
  });
}
