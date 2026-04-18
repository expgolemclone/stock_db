import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";

import { connectBrowser } from "./browser_pool.js";

function chromeAvailable() {
  for (const executable of [
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
  ]) {
    if (spawnSync("which", [executable], { stdio: "ignore" }).status === 0) {
      return true;
    }
  }
  return false;
}

test("connectBrowser keeps the seed page open and closes only the probe page", async () => {
  const events = [];
  let receivedOptions = null;

  const seedPage = {
    closed: false,
    isClosed() {
      return this.closed;
    },
    url() {
      return "chrome://newtab/";
    },
    async goto(url, options) {
      events.push(["seed.goto", url, options.waitUntil]);
    },
    async close() {
      this.closed = true;
      events.push(["seed.close"]);
    },
  };

  const probePage = {
    closed: false,
    async close() {
      this.closed = true;
      events.push(["probe.close"]);
    },
  };

  const browser = {
    closed: false,
    async newPage() {
      events.push(["browser.newPage"]);
      return probePage;
    },
    async close() {
      this.closed = true;
      events.push(["browser.close"]);
    },
  };

  const entry = await connectBrowser({
    customConfig: { chromePath: "/tmp/fake-chrome" },
  }, async (options) => {
    receivedOptions = options;
    return { browser, page: seedPage };
  });

  assert.equal(entry.browser, browser);
  assert.equal(entry.seedPage, seedPage);
  assert.equal(seedPage.closed, false);
  assert.equal(probePage.closed, true);
  assert.equal(browser.closed, false);
  assert.deepEqual(receivedOptions.customConfig, {
    chromePath: "/tmp/fake-chrome",
    handleSIGINT: false,
  });
  assert.deepEqual(events, [
    ["browser.newPage"],
    ["seed.goto", "about:blank", "domcontentloaded"],
    ["probe.close"],
  ]);
});

test("headful browser entry remains usable after startup probe cleanup", {
  skip: !process.env.DISPLAY || !chromeAvailable(),
  timeout: 30_000,
}, async () => {
  const entry = await connectBrowser({
    headless: false,
    disableXvfb: true,
    turnstile: true,
    args: [
      "--no-sandbox",
      "--disable-setuid-sandbox",
      "--ignore-certificate-errors",
    ],
  });

  try {
    assert.equal(entry.seedPage.isClosed(), false);

    const page1 = await entry.browser.newPage();
    await page1.goto("about:blank", { waitUntil: "domcontentloaded" });
    await page1.close();

    const page2 = await entry.browser.newPage();
    await page2.goto("about:blank", { waitUntil: "domcontentloaded" });
    assert.equal(page2.isClosed(), false);
    await page2.close();

    assert.equal(entry.seedPage.isClosed(), false);
  } finally {
    await entry.browser.close().catch(() => {});
  }
});
