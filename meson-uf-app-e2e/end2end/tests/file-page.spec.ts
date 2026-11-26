import { test, expect } from "./fixtures";
import { expectNoHorizontalOverflow, seedAuth, waitForHydrated } from "./fixtures";
import type { Page } from "@playwright/test";

async function openFile(page: Page, id: string) {
  await page.goto(`/meson/files/${encodeURIComponent(id)}`);
  await waitForHydrated(page);
  return page.getByTestId("meson-file-detail");
}

test.describe("pw-meson-file-page", () => {
  test("pw-meson-file-page-breadcrumb-nav-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    const detail = await openFile(page, seeded.fixtures.owner_image_file_id);

    const crumb = detail.getByTestId("meson-file-breadcrumb");
    await expect(crumb.getByText("receipt.png")).toBeVisible();
    await expect(page.locator("#meson-nav-my-files [aria-current='page']")).toHaveCount(1);
    await expect(detail.getByTestId("meson-file-scan-badge")).toHaveText("Clean");

    // Open in its app stays off until the app registers where to open files.
    await expect(
      detail.getByTestId("meson-file-open").getByRole("button", { name: "Open in Meson test files" }),
    ).toBeDisabled();
    await expect(detail.getByTestId("meson-file-download")).toBeEnabled();

    const details = detail.getByTestId("meson-file-details");
    await expect(details.getByTestId("meson-detail-app")).toContainText("Meson test files");
    await expect(details.getByTestId("meson-detail-type")).toContainText("PNG image");
    await expect(details.getByTestId("meson-detail-size")).toContainText(/\(\d[\d,]* bytes\)/);
    await expect(details.getByTestId("meson-detail-scan")).toContainText("Clean");
    const uploaded = details.getByTestId("meson-detail-uploaded").locator("time");
    await expect(uploaded).toHaveAttribute("datetime", /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
    await expect(uploaded).toHaveText(/^[A-Z][a-z]{2} \d{1,2}, \d{4}, \d{1,2}:\d{2} [AP]M UTC$/);

    await crumb.getByText("My Files").click();
    await expect(page).toHaveURL(/\/meson$/);
    await expect(page.getByTestId("meson-my-files")).toBeVisible();
  });

  test("pw-meson-file-page-scanning-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    const detail = await openFile(page, seeded.fixtures.owner_scanning_file_id);

    await expect(detail.getByTestId("meson-file-scan-badge")).toHaveText("Scanning");
    await expect(detail.getByTestId("meson-file-scanning")).toContainText("Checking this file for viruses");
    const preview = detail.getByTestId("meson-preview-scanning");
    await expect(preview.getByText("Scanning since")).toBeVisible();
    await expect(preview.locator("time")).toHaveAttribute("datetime", /T/);
    await expect(detail.getByTestId("meson-file-download")).toBeDisabled();
    await expect(detail.getByTestId("meson-file-delete")).toBeEnabled();
    await expect(detail.getByTestId("meson-img")).toHaveCount(0);
  });

  test("pw-meson-file-page-slow-scan-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    const detail = await openFile(page, seeded.fixtures.owner_slow_scan_file_id);

    const bar = detail.getByTestId("meson-file-scan-slow");
    await expect(bar).toContainText("taking longer than usual");
    await expect(detail.getByTestId("meson-preview-waiting")).toContainText("Preview opens after the virus check");
    await bar.getByRole("button", { name: "Delete and upload again" }).click();
    await expect(page.getByTestId("meson-delete-dialog")).toContainText("can't be undone");
    await page.getByTestId("meson-delete-cancel").click();
    await expect(page.getByTestId("meson-delete-dialog")).toBeHidden();
  });

  test("pw-meson-file-page-quarantined-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    const detail = await openFile(page, seeded.fixtures.owner_quarantined_file_id);

    await expect(detail.getByTestId("meson-file-scan-badge")).toHaveText("Quarantined");
    await expect(detail.getByTestId("meson-file-quarantined")).toContainText("The virus scan flagged this file");
    await expect(detail.getByTestId("meson-preview-blocked")).toContainText("Preview blocked");
    await expect(detail.getByTestId("meson-file-download")).toBeDisabled();
    await expect(detail.getByTestId("meson-file-delete")).toHaveAttribute("data-appearance", "danger");
    await expect(detail.getByTestId("meson-img")).toHaveCount(0);

    await detail.getByTestId("meson-file-delete").click();
    const dialog = page.getByTestId("meson-delete-dialog");
    await expect(dialog).toContainText("removes the locked copy from storage");
    await page.getByTestId("meson-delete-cancel").click();
    await expect(dialog).toBeHidden();
  });

  test("pw-meson-file-page-delete-quarantined-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "owner", ["quarantined"]);
    const id = seeded.fresh.quarantined!;
    const detail = await openFile(page, id);

    await detail.getByTestId("meson-file-quarantined-delete").click();
    const dialog = page.getByTestId("meson-delete-dialog");
    await expect(dialog).toContainText("This frees");
    await page.getByTestId("meson-delete-confirm").click();
    await expect(page).toHaveURL(/\/meson$/);
    await expect(page.getByTestId("meson-my-files")).toBeVisible();

    await openFile(page, id);
    await expect(page.getByTestId("meson-file-not-in-your-files")).toBeVisible();
  });

  test("pw-meson-file-page-delete-refused-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "owner", ["keep"]);
    const detail = await openFile(page, seeded.fresh.keep!);

    await detail.getByTestId("meson-file-delete").click();
    await page.getByTestId("meson-delete-confirm").click();
    const refused = page.getByTestId("meson-delete-refused");
    await expect(refused).toContainText("This test file is marked keep.");
    await expect(refused).toContainText("Nothing was deleted.");
    await expect(page.getByTestId("meson-delete-confirm")).toHaveCount(0);
    await page.getByTestId("meson-delete-cancel").click();
    await expect(page).toHaveURL(/\/meson\/files\//);
    await expect(detail.getByTestId("meson-file-title")).toHaveText("keep-receipt.png");
  });

  test("pw-meson-file-page-download-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    const detail = await openFile(page, seeded.fixtures.owner_text_file_id);

    const downloadEvent = page.waitForEvent("download");
    await detail.getByTestId("meson-file-download").click();
    const download = await downloadEvent;
    expect(download.suggestedFilename()).toBe("notes.txt");
    await expect(page.getByTestId("meson-download-failed")).toHaveCount(0);
  });

  test("pw-meson-file-page-too-big-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    const detail = await openFile(page, seeded.fixtures.owner_big_file_id);

    const preview = detail.getByTestId("meson-preview-too-big");
    await expect(preview.getByText("Too big to preview")).toBeVisible();
    await expect(preview.getByText("Previews stop at 5 MB. Download it to read it.")).toBeVisible();
    await expect(preview.getByText("This file is 6.3 MB.")).toBeVisible();
    await expect(preview.getByRole("button", { name: "Download" })).toBeVisible();
    await expect(detail.getByTestId("meson-detail-size")).toContainText("6,291,456 bytes");
  });

  test("pw-meson-file-page-no-preview-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    const detail = await openFile(page, seeded.fixtures.owner_pdf_file_id);

    const preview = detail.getByTestId("meson-preview-unsupported");
    await expect(preview.getByText("No preview for this file type")).toBeVisible();
    await expect(preview.getByText("PDF files can't be shown here.")).toBeVisible();
    await expect(preview.getByRole("button", { name: "Download" })).toBeVisible();
  });

  test.describe("phone width", () => {
    test.use({ viewport: { width: 390, height: 844 } });

    test("pw-meson-file-page-390-happy", async ({ page }) => {
      const seeded = await seedAuth(page, "owner");
      const detail = await openFile(page, seeded.fixtures.owner_image_file_id);

      await expect(detail.getByTestId("meson-file-columns")).toHaveAttribute("data-narrow", "true");
      await expect(detail.getByTestId("meson-file-delete")).toBeVisible();
      await expect(detail.getByTestId("meson-file-details")).toBeVisible();
      await expectNoHorizontalOverflow(page);
    });
  });
});
