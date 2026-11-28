import { test, expect } from "./fixtures";
import { expectNoHorizontalOverflow, seedAuth, waitForHydrated } from "./fixtures";

test.describe("pw-meson", () => {
  test("pw-meson-my-files-populated-happy", async ({ page }) => {
    await seedAuth(page, "owner");
    await page.goto("/meson");
    await waitForHydrated(page);

    await expect(page.getByTestId("meson-app-root")).toBeAttached();
    await expect(page.getByTestId("meson-my-files")).toBeAttached();
    await expect(page.getByText("receipt.png", { exact: true })).toBeVisible();
    await expect(page.getByText("notes.txt", { exact: true })).toBeVisible();
    await expect(page.getByText(/Showing \d+ files/)).toBeVisible();
  });

  // Phone-width viewport + dark scheme so each preview is also checked at 390px.
  test.describe("narrow dark", () => {
    test.use({ viewport: { width: 390, height: 844 }, colorScheme: "dark" });

    test("meson-preview-image-happy", async ({ page }) => {
      const seeded = await seedAuth(page, "owner");
      await page.goto(`/meson/files/${encodeURIComponent(seeded.fixtures.owner_image_file_id)}`);
      await waitForHydrated(page);

      const detail = page.getByTestId("meson-file-detail");
      await expect(detail.getByTestId("meson-file-title")).toHaveText("receipt.png");
      const img = detail.getByTestId("meson-img").getByRole("img", { name: "receipt.png" });
      await expect(img).toBeVisible();
      await expect(img).toHaveAttribute("src", /^\/api\/files\/owner-image$/);
      await expect
        .poll(() => img.evaluate((el: HTMLImageElement) => el.naturalWidth))
        .toBeGreaterThan(0);
      await expect(detail.getByTestId("meson-detail-type")).toContainText("PNG image");
      await expect(page.getByText(/not_found:/)).toHaveCount(0);
      await expectNoHorizontalOverflow(page);
    });

    test("meson-preview-text-happy", async ({ page }) => {
      const seeded = await seedAuth(page, "owner");
      await page.goto(`/meson/files/${encodeURIComponent(seeded.fixtures.owner_text_file_id)}`);
      await waitForHydrated(page);

      const detail = page.getByTestId("meson-file-detail");
      await expect(detail.getByTestId("meson-file-title")).toHaveText("notes.txt");
      await expect(detail.getByTestId("meson-detail-type")).toContainText("Text");
      await expect(detail.getByText("Meson preview fixture")).toBeVisible();
      await expect(detail.getByText("line two: seeded text")).toBeVisible();
      await expect(detail.getByTestId("meson-img")).toHaveCount(0);
      await expect(page.getByText(/not_found:|io:/)).toHaveCount(0);
      await expectNoHorizontalOverflow(page);
    });

    test("meson-preview-forbidden-sad", async ({ page }) => {
      const seeded = await seedAuth(page, "owner");
      await page.goto(`/meson/files/${encodeURIComponent(seeded.fixtures.peer_file_id)}`);
      await waitForHydrated(page);

      const detail = page.getByTestId("meson-file-detail");
      await expect(detail.getByText("This file isn't in your files")).toBeVisible();
      await expect(detail.getByTestId("meson-img")).toHaveCount(0);
      await expect(page.getByText("peer-only.png")).toHaveCount(0);

      // The byte route collapses a foreign id into the same 404 as a missing one.
      const foreign = await page.request.get(`/api/files/${seeded.fixtures.peer_file_id}`);
      expect(foreign.status()).toBe(404);
      const own = await page.request.get(`/api/files/${seeded.fixtures.owner_image_file_id}`);
      expect(own.status()).toBe(200);
      expect(own.headers()["content-type"]).toBe("image/png");

      await seedAuth(page, "anonymous");
      const anon = await page.request.get(`/api/files/${seeded.fixtures.owner_image_file_id}`);
      expect(anon.status()).toBe(401);
    });
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
    await expect(page.getByText("Sign in again to see your files")).toBeVisible();
    await expect(page.getByRole("link", { name: "Sign in again" })).toBeVisible();
    await expect(page.getByText(/auth:|error running server function/)).toHaveCount(0);
  });

  test("pw-meson-no-files-read-denied-sad", async ({ page }) => {
    await seedAuth(page, "no_files_read");
    await page.goto("/meson");
    await waitForHydrated(page);

    const denied = page.getByTestId("meson-files-read-denied");
    await expect(denied.getByText("You can't open My Files yet")).toBeVisible();
    await expect(denied.getByText(/Files read permission/)).toBeVisible();
    await expect(denied.getByRole("link", { name: "Ask a site owner" })).toBeVisible();
    await expect(page.getByTestId("meson-my-files")).toHaveCount(0);

    await page.goto("/meson/files/owner-image");
    await waitForHydrated(page);
    await expect(page.getByTestId("meson-files-read-denied")).toBeVisible();
    await expect(page.getByTestId("meson-file-detail")).toHaveCount(0);
  });

  test("pw-meson-file-not-in-your-files-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    const ids = [
      seeded.fixtures.peer_file_id,
      "file:doesnotexist",
      "not-a-record-id",
    ];
    for (const id of ids) {
      await page.goto(`/meson/files/${encodeURIComponent(id)}`);
      await waitForHydrated(page);
      const empty = page.getByTestId("meson-file-not-in-your-files");
      await expect(empty.getByText("This file isn't in your files")).toBeVisible();
      await expect(empty.getByRole("link", { name: "Back to My Files" })).toBeVisible();
      await expect(page.getByText(/not_found:|error running server function/)).toHaveCount(0);
    }
  });

  test("pw-meson-keeps-dialog-happy", async ({ page }) => {
    await seedAuth(page, "owner");
    await page.goto("/meson");
    await waitForHydrated(page);

    await page.getByTestId("meson-keeps-link").click();
    const dialog = page.getByTestId("meson-keeps-dialog");
    await expect(dialog.getByText(/they don't see the file names/)).toBeVisible();
    await expect(dialog.getByText(/Close your account and Meson deletes your files within 30 days/)).toBeVisible();

    await page.getByTestId("meson-keeps-close").click();
    await expect(dialog).toBeHidden();
    await expect(page.getByText("receipt.png", { exact: true })).toBeVisible();
  });
});
