import { test, expect, type Page } from "@playwright/test";
import { expectNoHorizontalOverflow, waitForHydrated } from "./fixtures";

const GB = 1_000_000_000;

type Row = {
  owner_kind: string;
  owner_id: string;
  app_table?: string;
  file_status?: string;
  files: number;
  bytes: number;
  missing_size_files?: number;
  copies_reported?: number;
};

type Partition = { hours_old: number; run_id: string; rows: Row[] };

type Settings = {
  copies_override: number | null;
  price_per_gb_month: number | null;
} | null;

/** One daily run: site rows, the `owner` member's rows and two other owners. */
function run(
  runId: string,
  hoursOld: number,
  opts: { ownerBytes: number; siteBytes?: number; copies?: number },
): Partition {
  const siteBytes = opts.siteBytes ?? 412 * GB;
  const finance = Math.round(opts.ownerBytes * 0.77);
  return {
    hours_old: hoursOld,
    run_id: runId,
    rows: [
      {
        owner_kind: "site",
        owner_id: "all",
        files: 48_693,
        bytes: siteBytes,
        copies_reported: opts.copies ?? 3,
      },
      { owner_kind: "site", owner_id: "all", app_table: "finance", files: 30_000, bytes: siteBytes - 100 * GB },
      { owner_kind: "site", owner_id: "all", app_table: "receipt_scan", files: 18_693, bytes: 100 * GB },
      { owner_kind: "user", owner_id: "owner", files: 12_486, bytes: opts.ownerBytes },
      { owner_kind: "user", owner_id: "owner", app_table: "finance", files: 9_812, bytes: finance },
      {
        owner_kind: "user",
        owner_id: "owner",
        app_table: "receipt_scan",
        files: 2_674,
        bytes: opts.ownerBytes - finance,
      },
      { owner_kind: "user", owner_id: "siteowner", files: 40, bytes: 2 * GB },
      { owner_kind: "service_user", owner_id: "backup-bot", files: 900, bytes: 60 * GB },
    ],
  };
}

async function seedStorage(
  page: Page,
  auth: "owner" | "site_owner",
  partitions: Partition[],
  settings: Settings,
) {
  const res = await page.request.post("/api/test/seed-data", {
    data: { auth, storage: { partitions, settings } },
  });
  expect(res.ok()).toBeTruthy();
}

async function openStorage(page: Page, query = "") {
  await page.goto(`/meson/storage${query}`);
  await waitForHydrated(page);
  await expect(page.getByTestId("meson-storage")).toBeVisible();
}

const PRICE = { copies_override: null, price_per_gb_month: 0.023 };

test.describe("pw-storage", () => {
  test("storage-latest-partition-only-happy", async ({ page }) => {
    await seedStorage(
      page,
      "owner",
      [run("new", 2, { ownerBytes: 18.4 * GB }), run("old", 26, { ownerBytes: 9 * GB })],
      PRICE,
    );
    await openStorage(page);

    await expect(page.getByTestId("nav-meson-storage")).toBeVisible();
    await expect(page.getByTestId("storage-stat-used")).toContainText("18.4 GB");
    await expect(page.getByTestId("storage-stat-cost")).toContainText("Estimated cost a month");
    await expect(page.getByTestId("storage-stat-cost")).toContainText("$1.27");
    await expect(page.getByTestId("storage-stat-files")).toContainText("12,486");
    await expect(page.getByTestId("storage-stat-change")).toContainText("+9.4 GB");
    await expect(page.getByTestId("storage-history-chart")).toHaveAttribute("data-points", "2");
    await expect(page.getByTestId("storage-by-app")).toContainText("Finance");
    await expect(page.getByTestId("storage-worked-out")).toHaveText(
      "18.4 GB × 3 copies × $0.023 = $1.27 a month",
    );
    await expect(page.getByTestId("storage-banner-stale")).toHaveCount(0);
    await expect(page.getByTestId("storage-banner-live")).toHaveCount(0);
    // Biggest files come from the member's own files.
    await expect(page.getByTestId("storage-biggest-files")).toContainText("receipt.png");
    // Members don't get the Everyone tab.
    await expect(page.getByRole("tab", { name: "Everyone" })).toHaveCount(0);
  });

  test("storage-no-partitions-live-estimate-sad", async ({ page }) => {
    await seedStorage(page, "owner", [], null);
    await openStorage(page);

    await expect(page.getByTestId("storage-banner-live")).toContainText("Live estimate");
    await expect(page.getByTestId("storage-banner-size-only")).toContainText("Size only");
    await expect(page.getByTestId("storage-stat-used")).toContainText("About");
    await expect(page.getByTestId("storage-stat-files")).toContainText("2");
    await expect(page.getByTestId("storage-stat-change")).toContainText("No history yet");
    await expect(page.getByTestId("storage-history-none-yet")).toBeVisible();
    await expect(page.getByTestId("storage-how")).toContainText(
      "Added up just now from the sizes recorded at upload.",
    );
    // Members get the sentence, not the owner's action.
    await expect(
      page.getByTestId("storage-banner-size-only").getByRole("button", { name: "Set a price" }),
    ).toHaveCount(0);
    await expect(page.getByText(/io:|Spectra/)).toHaveCount(0);
  });

  test("storage-stale-partition-banner-sad", async ({ page }) => {
    await seedStorage(page, "owner", [run("stale", 40, { ownerBytes: 18.4 * GB })], PRICE);
    await openStorage(page);

    await expect(page.getByTestId("storage-banner-stale")).toContainText(
      "These numbers are 1 day old",
    );
    await expect(page.getByTestId("storage-banner-stale")).toContainText(
      "files added since aren't counted",
    );
    await expect(page.getByTestId("storage-stat-used")).toContainText("18.4 GB");
    await expect(page.getByTestId("storage-how")).toContainText("The daily run has failed since then.");
  });

  test("storage-no-settings-size-only-sad", async ({ page }) => {
    await seedStorage(page, "owner", [run("r1", 3, { ownerBytes: 18.4 * GB })], null);
    await openStorage(page);

    await expect(page.getByTestId("storage-banner-size-only")).toContainText(
      "A site owner hasn't set a storage price",
    );
    await expect(page.getByTestId("storage-stat-cost")).toContainText("Stored with copies");
    await expect(page.getByTestId("storage-stat-cost")).toContainText("55.2 GB");
    await expect(page.getByTestId("storage-by-app")).not.toContainText("Cost a month");
    await expect(page.getByTestId("storage-worked-out")).toHaveText(
      "18.4 GB × 3 copies = 55.2 GB stored",
    );
  });

  test("storage-copies-override-happy", async ({ page }) => {
    await seedStorage(page, "owner", [run("r1", 3, { ownerBytes: 4 * GB, copies: 3 })], {
      copies_override: 2,
      price_per_gb_month: 0.5,
    });
    await openStorage(page);

    await expect(page.getByTestId("storage-how")).toContainText("2, set by a site owner.");
    await expect(page.getByTestId("storage-worked-out")).toHaveText(
      "4.0 GB × 2 copies × $0.50 = $4.00 a month",
    );
    await expect(page.getByTestId("storage-stat-cost")).toContainText("$4.00");
    await expect(page.getByTestId("storage-banner-copies")).toHaveCount(0);
  });

  test("storage-site-owner-everyone-and-settings-happy", async ({ page }) => {
    await seedStorage(page, "site_owner", [run("r1", 3, { ownerBytes: 18.4 * GB })], null);
    await openStorage(page, "?view=everyone");

    const everyone = page.getByTestId("storage-everyone");
    await expect(everyone).toBeVisible();
    await expect(page.getByRole("tab", { name: "Everyone" })).toBeVisible();
    await expect(page.getByTestId("storage-site-stat-size")).toContainText("412 GB");
    await expect(page.getByTestId("storage-site-stat-stored")).toContainText("Stored with 3 copies");
    await expect(page.getByTestId("storage-site-stat-stored")).toContainText("1.2 TB");
    await expect(page.getByTestId("storage-site-stat-cost")).toContainText("Set a price");
    await expect(page.getByTestId("storage-site-stat-people")).toContainText("3");
    await expect(page.getByTestId("storage-by-person")).toContainText("owner");
    await expect(page.getByTestId("storage-by-person")).toContainText("backup-bot");
    await expect(page.getByTestId("storage-run-card")).toHaveAttribute("data-status", "Healthy");
    await expect(page.getByTestId("storage-run-card")).toContainText("meson.storage-daily");

    await page.getByTestId("storage-change-settings").click();
    const dialog = page.getByTestId("storage-settings-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText("Use what the storage service reports (3 copies)");

    const price = page.getByTestId("storage-settings-price").locator("input");
    await price.fill("abc");
    await expect(dialog).toContainText("Enter a price like 0.023");
    await expect(page.getByTestId("storage-settings-save")).toBeDisabled();

    await price.fill("0.023");
    await expect(page.getByTestId("storage-settings-preview")).toContainText(
      "412 GB × 3 copies × $0.023 = $28.43 a month for the whole site.",
    );
    await expect(page.getByTestId("storage-settings-save")).toBeEnabled();
    await page.getByTestId("storage-settings-save").click();

    await expect(dialog).toBeHidden();
    await expect(page.getByTestId("storage-site-stat-cost")).toContainText("$28.43");
    await expect(page.getByTestId("storage-banner-size-only")).toHaveCount(0);
  });

  test("storage-member-everyone-link-denied-sad", async ({ page }) => {
    await seedStorage(page, "owner", [run("r1", 3, { ownerBytes: 18.4 * GB })], PRICE);
    await openStorage(page, "?view=everyone");

    await expect(page.getByTestId("storage-everyone-denied")).toContainText(
      "Only site owners can see everyone's storage",
    );
    await expect(page.getByTestId("storage-everyone")).toHaveCount(0);
    await expect(page.getByRole("tab", { name: "Everyone" })).toHaveCount(0);

    await page.getByRole("button", { name: "Go to Your storage" }).click();
    await expect(page.getByTestId("storage-stat-used")).toContainText("18.4 GB");
    await expect(page).toHaveURL(/\/meson\/storage$/);
  });

  test.describe("narrow", () => {
    test.use({ viewport: { width: 390, height: 844 } });

    test("storage-phone-width-single-column", async ({ page }) => {
      await seedStorage(page, "owner", [run("r1", 3, { ownerBytes: 18.4 * GB })], PRICE);
      await openStorage(page);

      await expect(page.getByTestId("storage-stat-used")).toContainText("18.4 GB");
      const byApp = await page.getByTestId("storage-by-app").boundingBox();
      const how = await page.getByTestId("storage-how").boundingBox();
      expect(byApp).not.toBeNull();
      expect(how).not.toBeNull();
      // One column: the How card sits below the tables, not beside them.
      expect(how!.y).toBeGreaterThan(byApp!.y);
      await expectNoHorizontalOverflow(page);
    });
  });
});
