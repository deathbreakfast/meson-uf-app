import { test, expect } from "./fixtures";
import { expectNoHorizontalOverflow, seedAuth, waitForHydrated } from "./fixtures";

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

  // Phone-width viewport + dark scheme so each preview is also checked at 390px.
  test.describe("narrow dark", () => {
    test.use({ viewport: { width: 390, height: 844 }, colorScheme: "dark" });

    test("meson-preview-image-happy", async ({ page }) => {
      const seeded = await seedAuth(page, "owner");
      await page.goto(`/meson/files/${encodeURIComponent(seeded.fixtures.owner_image_file_id)}`);
      await waitForHydrated(page);

      const detail = page.getByTestId("meson-file-detail");
      await expect(detail.getByText("receipt.png", { exact: true })).toBeVisible();
      const img = detail.getByTestId("meson-img").getByRole("img", { name: "receipt.png" });
      await expect(img).toBeVisible();
      await expect(img).toHaveAttribute("src", /^\/api\/files\/owner-image$/);
      await expect
        .poll(() => img.evaluate((el: HTMLImageElement) => el.naturalWidth))
        .toBeGreaterThan(0);
      await expect(detail.getByText("Type: image/png")).toBeVisible();
      await expect(page.getByText(/not_found:/)).toHaveCount(0);
      await expectNoHorizontalOverflow(page);
    });

    test("meson-preview-text-happy", async ({ page }) => {
      const seeded = await seedAuth(page, "owner");
      await page.goto(`/meson/files/${encodeURIComponent(seeded.fixtures.owner_text_file_id)}`);
      await waitForHydrated(page);

      const detail = page.getByTestId("meson-file-detail");
      await expect(detail.getByText("notes.txt", { exact: true })).toBeVisible();
      await expect(detail.getByText("Type: text/plain")).toBeVisible();
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
      await expect(detail.getByText(/not_found:/)).toBeVisible();
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
    await expect(page.getByText(/auth:/)).toBeVisible();
  });
});
