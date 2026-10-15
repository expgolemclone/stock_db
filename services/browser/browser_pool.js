import { connect } from "puppeteer-real-browser";

/**
 * @typedef {{
 *   browser: import("puppeteer-core").Browser,
 *   seedPage: import("puppeteer-core").Page,
 *   lastUsed: number,
 * }} BrowserEntry
 */

async function closeBrowserQuietly(browser, context) {
  try {
    await browser.close();
  } catch (error) {
    console.debug(`browser.close() failed during ${context}:`, error?.message || error);
  }
}

async function closePageQuietly(page, context) {
  try {
    await page.close();
  } catch (error) {
    console.debug(`${context} failed:`, error?.message || error);
  }
}

async function normalizeSeedPage(seedPage) {
  if (seedPage.isClosed()) {
    throw new Error("Seed page closed during browser startup");
  }
  if (seedPage.url() !== "about:blank") {
    await seedPage.goto("about:blank", { waitUntil: "domcontentloaded" });
  }
}

function buildConnectOptions(options) {
  return {
    ...options,
    connectOption: {
      ...(options.connectOption || {}),
      protocolTimeout: 600_000,
    },
    customConfig: {
      ...(options.customConfig || {}),
      handleSIGINT: false,
    },
  };
}

// Keep the initial page returned by puppeteer-real-browser alive. In headful
// mode, closing both that seed page and a probe page breaks later newPage()
// calls with Target.createTarget errors.
export async function connectBrowser(options, connectImpl = connect) {
  const { browser, page: seedPage } = await connectImpl(buildConnectOptions(options));
  let probePage = null;
  try {
    probePage = await browser.newPage();
    await normalizeSeedPage(seedPage);
    return { browser, seedPage, lastUsed: Date.now() };
  } catch (error) {
    await closeBrowserQuietly(browser, "connect cleanup");
    throw error;
  } finally {
    if (probePage) {
      await closePageQuietly(probePage, "probePage.close()");
    }
  }
}
