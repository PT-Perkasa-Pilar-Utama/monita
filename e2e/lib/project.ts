import type { Page } from "@playwright/test";

/** Creates a project from the home page's New project dialog. Lands on its monitors page. */
export async function createProject(page: Page, name: string, slug: string): Promise<void> {
  await page.goto("/");
  await page.getByRole("button", { name: "New project" }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Name").fill(name);
  await dialog.getByLabel("Slug").fill(slug);
  await dialog.getByRole("button", { name: "Create project" }).click();
}
