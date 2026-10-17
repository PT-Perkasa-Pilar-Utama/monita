import { expect, type Page } from "@playwright/test";

/** Creates a project from the home page's New project dialog and returns the slug Monita made. */
export async function createProject(page: Page, name: string): Promise<string> {
  await page.goto("/");
  await page.getByRole("button", { name: "New project" }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Name").fill(name);
  await dialog.getByRole("button", { name: "Create project" }).click();
  await expect(page).toHaveURL(/\/projects\/[a-z0-9-]+-[a-z2-7]{6}\/monitors$/);
  return new URL(page.url()).pathname.split("/")[2];
}
