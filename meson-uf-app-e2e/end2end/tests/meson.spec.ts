import { test, expect } from "./fixtures";
import { seedAuth, waitForHydrated } from "./fixtures";

test.describe("pw-meson", () => {
  test("pw-meson-my-files-populated-happy", async ({ page }) => {
    await seedAuth(page, "owner");
    await page.goto("/meson");
    await waitForHydrated(page);

    await expect(page.getByTestId("meson-app-root")).toBeAttached();
    await expect(page.getByTestId("meson-my-files")).toBeAttached();
    await expect(page.getByText("receipt.png")).toBeVisible();
    await expect(page.getByText("notes.txt")).toBeVisible();
    await expect(page.getByText(/Showing 2 files/)).toBeVisible();
  });

  test("pw-meson-file-detail-image-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    await page.goto(`/meson/files/${encodeURIComponent(seeded.fixtures.owner_image_file_id)}`);
    await waitForHydrated(page);

    await expect(page.getByTestId("meson-file-detail")).toBeAttached();
    await expect(page.getByText("Back to My Files")).toBeVisible();
    await expect(page.getByText(/not_found:/)).toHaveCount(0);
  });

  test("pw-meson-file-detail-text-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    await page.goto(`/meson/files/${encodeURIComponent(seeded.fixtures.owner_text_file_id)}`);
    await waitForHydrated(page);

    await expect(page.getByTestId("meson-file-detail")).toBeAttached();
    await expect(page.getByText(/not_found:/)).toHaveCount(0);
  });

  test("pw-meson-file-detail-peer-id-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    await page.goto(`/meson/files/${encodeURIComponent(seeded.fixtures.peer_file_id)}`);
    await waitForHydrated(page);

    await expect(page.getByTestId("meson-file-detail")).toBeAttached();
    await expect(page.getByText(/not_found:/)).toBeVisible();
  });

  test("pw-meson-unauth-gated-sad", async ({ page }) => {
    await seedAuth(page, "anonymous");
    await page.goto("/meson");
    await waitForHydrated(page);

    await expect(page.getByTestId("auth-required-empty-state")).toBeAttached();
    await expect(page.getByTestId("meson-my-files")).toHaveCount(0);
  });

  test("pw-meson-my-files-error-sad", async ({ page }) => {
    await seedAuth(page, "broken_session");
    await page.goto("/meson");
    await waitForHydrated(page);

    await expect(page.getByTestId("meson-my-files")).toBeAttached();
    await expect(page.getByText(/auth:/)).toBeVisible();
  });
});
