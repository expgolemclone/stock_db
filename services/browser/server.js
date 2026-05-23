import express from "express";
import { randomUUID } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { connectBrowser } from "./browser_pool.js";

// puppeteer-real-browser's targetcreated listener races with page.close() on
// the error path: its async page setup throws TargetCloseError against a page
// we've already closed, which Node 24 treats as a fatal unhandled rejection.
process.on("unhandledRejection", (reason) => {
  console.error("unhandledRejection:", reason?.message || reason);
});

const POOL_SIZE = parseInt(process.env.BROWSER_POOL_SIZE || "10", 10);
const PAGE_TIMEOUT = parseInt(process.env.BROWSER_PAGE_TIMEOUT || "30000", 10);
const IDLE_TIMEOUT = parseInt(process.env.BROWSER_IDLE_TIMEOUT || "300", 10) * 1000;
const BROWSER_HEADLESS = parseBoolean(process.env.BROWSER_HEADLESS, false);
const BROWSER_DISABLE_XVFB = parseBoolean(process.env.BROWSER_DISABLE_XVFB, false);
const CHALLENGE_POLL_INTERVAL_MS = parseInt(
  process.env.BROWSER_CHALLENGE_POLL_INTERVAL_MS || "250",
  10,
);
const CHALLENGE_CLEAR_STABLE_MS = parseInt(
  process.env.BROWSER_CHALLENGE_CLEAR_STABLE_MS || "1000",
  10,
);
const STOOQ_SESSION_TTL_MS = 5 * 60 * 1000;
const HAS_NATIVE_DISPLAY = Boolean(process.env.DISPLAY);
const HAS_XVFB = detectXvfb();

const CHALLENGE_TITLE_PATTERNS = [
  "just a moment...",
  "attention required!",
];

const CHALLENGE_BODY_PATTERNS = [
  "performing security verification",
  "verification successful",
  "checking your browser before accessing",
];

/** @typedef {{
 *   browser: import('puppeteer-core').Browser,
 *   seedPage: import('puppeteer-core').Page,
 *   lastUsed: number,
 * }} BrowserEntry */

/** @type {Map<string, BrowserEntry>} pool key -> browser instance */
const browserPool = new Map();

/** @type {Map<string, Promise<BrowserEntry>>} in-flight connect() calls */
const pendingConnections = new Map();
const stooqSessions = new Map();
let xvfbFallbackWarned = false;

function parseBoolean(rawValue, defaultValue) {
  if (rawValue === undefined) {
    return defaultValue;
  }
  return rawValue.toLowerCase() === "true";
}

function poolKey(proxyAddr, username) {
  return `${proxyAddr}|${username || ""}`;
}

function detectXvfb() {
  if (HAS_NATIVE_DISPLAY) {
    return true;
  }
  const result = spawnSync("which", ["Xvfb"], { stdio: "ignore" });
  return result.status === 0;
}

function shouldForceHeadless() {
  return !BROWSER_DISABLE_XVFB && !HAS_NATIVE_DISPLAY && !HAS_XVFB;
}

function buildBrowserOptions(proxyAddr, proxyType, username, password) {
  const useHeadless = BROWSER_HEADLESS || shouldForceHeadless();
  if (!BROWSER_HEADLESS && shouldForceHeadless() && !xvfbFallbackWarned) {
    console.warn("Xvfb unavailable, starting browser in headless mode");
    xvfbFallbackWarned = true;
  }
  const options = {
    headless: useHeadless ? "new" : false,
    disableXvfb: useHeadless ? true : BROWSER_DISABLE_XVFB,
    turnstile: true,
    args: [
      "--no-sandbox",
      "--disable-setuid-sandbox",
      "--ignore-certificate-errors",
    ],
  };

  const isDirect = !proxyAddr || proxyAddr.startsWith("direct");
  if (!isDirect) {
    if (proxyType === "socks5") {
      options.args.push(`--proxy-server=socks5://${proxyAddr}`);
    } else {
      const [host, portStr] = proxyAddr.split(":");
      options.proxy = { host, port: parseInt(portStr, 10) };
      if (username && password !== undefined) {
        options.proxy.username = username;
        options.proxy.password = password;
      }
    }
  }

  return options;
}

function shouldRetryHeadless(error, options) {
  void error;
  return options.headless === false && options.disableXvfb !== true;
}

async function launchBrowser(proxyAddr, proxyType, username, password) {
  const options = buildBrowserOptions(proxyAddr, proxyType, username, password);
  try {
    return await connectBrowser(options);
  } catch (error) {
    if (!shouldRetryHeadless(error, options)) {
      throw error;
    }

    console.warn("Xvfb unavailable, retrying browser in headless mode");
    const fallbackOptions = {
      ...options,
      headless: "new",
      disableXvfb: true,
    };
    return connectBrowser(fallbackOptions);
  }
}

async function getBrowser(proxyAddr, proxyType, username, password) {
  const key = poolKey(proxyAddr, username);
  const existing = browserPool.get(key);
  if (existing) {
    existing.lastUsed = Date.now();
    return existing;
  }

  const pending = pendingConnections.get(key);
  if (pending) {
    return pending;
  }

  if (browserPool.size >= POOL_SIZE) {
    let oldestKey = null;
    let oldestTime = Infinity;
    for (const [candidateKey, entry] of browserPool) {
      if (entry.lastUsed < oldestTime) {
        oldestTime = entry.lastUsed;
        oldestKey = candidateKey;
      }
    }
    if (oldestKey) {
      await closeBrowser(oldestKey);
    }
  }

  const promise = launchBrowser(proxyAddr, proxyType, username, password).then((entry) => {
    browserPool.set(key, entry);
    pendingConnections.delete(key);
    return entry;
  }).catch((error) => {
    pendingConnections.delete(key);
    throw error;
  });

  pendingConnections.set(key, promise);
  return promise;
}

async function closeBrowser(key) {
  const entry = browserPool.get(key);
  if (entry) {
    browserPool.delete(key);
    try {
      await entry.browser.close();
    } catch (error) {
      console.debug("browser.close() failed during pool cleanup:", error?.message || error);
    }
  }
}

async function openRequestPage(proxyAddr, proxyType, proxyUsername, proxyPassword) {
  const key = poolKey(proxyAddr, proxyUsername);
  let lastError = null;

  for (let attempt = 1; attempt <= 2; attempt += 1) {
    const entry = await getBrowser(proxyAddr, proxyType, proxyUsername, proxyPassword);
    try {
      const page = await entry.browser.newPage();
      entry.lastUsed = Date.now();
      return { key, page };
    } catch (error) {
      lastError = error;
      console.warn(
        `browser.newPage() failed for ${key} on attempt ${attempt}:`,
        error?.message || error,
      );
      await closeBrowser(key);
    }
  }

  throw lastError;
}

async function closeAllBrowsers() {
  const closeTasks = [...browserPool.keys()].map((key) => closeBrowser(key));
  await Promise.allSettled(closeTasks);
}

async function closeStooqSession(sessionId) {
  const session = stooqSessions.get(sessionId);
  if (!session) {
    return;
  }

  stooqSessions.delete(sessionId);
  try {
    await session.page.close();
  } catch (error) {
    console.debug("page.close() failed during Stooq session cleanup:", error?.message || error);
  }
}

async function closeAllStooqSessions() {
  const closeTasks = [...stooqSessions.keys()].map((sessionId) => closeStooqSession(sessionId));
  await Promise.allSettled(closeTasks);
}

function touchStooqSession(sessionId) {
  const session = stooqSessions.get(sessionId);
  if (session) {
    session.lastUsed = Date.now();
  }
  return session;
}

setInterval(async () => {
  const now = Date.now();

  // Map イテレーション中の変更を避けるため keys を配列化
  for (const key of [...browserPool.keys()]) {
    const entry = browserPool.get(key);
    if (entry && now - entry.lastUsed > IDLE_TIMEOUT) {
      await closeBrowser(key);
    }
  }

  for (const [sessionId, session] of [...stooqSessions.entries()]) {
    if (now - session.lastUsed > STOOQ_SESSION_TTL_MS) {
      await closeStooqSession(sessionId);
    }
  }
}, 30_000);

function normalizeText(text) {
  return text.replace(/\s+/g, " ").trim();
}

function remainingTimeout(deadline) {
  const remaining = deadline - Date.now();
  if (remaining <= 0) {
    throw new Error("Timed out while waiting for the page to become ready");
  }
  return remaining;
}

function sleep(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

async function readPageState(page) {
  const url = page.url();
  const title = await page.title().catch(() => "");
  const bodyText = await page.evaluate(
    () => (document.body ? document.body.innerText : ""),
  ).catch(() => "");

  return {
    url: url.toLowerCase(),
    title: normalizeText(title).toLowerCase(),
    bodyText: normalizeText(bodyText).toLowerCase(),
  };
}

function isChallengeState(pageState) {
  if (pageState.url.includes("__cf_chl")) {
    return true;
  }
  if (pageState.url.includes("/cdn-cgi/challenge-platform/")) {
    return true;
  }
  if (CHALLENGE_TITLE_PATTERNS.some((pattern) => pageState.title.includes(pattern))) {
    return true;
  }
  return CHALLENGE_BODY_PATTERNS.some((pattern) => pageState.bodyText.includes(pattern));
}

async function waitForPageReady(page, deadline, url) {
  let clearedAt = null;
  while (true) {
    const pageState = await readPageState(page);
    if (isChallengeState(pageState)) {
      clearedAt = null;
    } else if (clearedAt === null) {
      clearedAt = Date.now();
    } else if (Date.now() - clearedAt >= CHALLENGE_CLEAR_STABLE_MS) {
      return;
    }

    if (deadline - Date.now() <= 0) {
      throw new Error(`Challenge did not clear before timeout for ${url}`);
    }
    await sleep(Math.min(CHALLENGE_POLL_INTERVAL_MS, remainingTimeout(deadline)));
  }
}

async function navigateWithChallengeWait(page, url, deadline, allowDownloadAbort = false) {
  try {
    await page.goto(url, {
      waitUntil: "domcontentloaded",
      timeout: remainingTimeout(deadline),
    });
  } catch (error) {
    const message = String(error?.message || error);
    if (allowDownloadAbort && message.includes("net::ERR_ABORTED")) {
      return { downloadStarted: true };
    }
    throw error;
  }

  await waitForPageReady(page, deadline, url);
  return { downloadStarted: false };
}

async function waitForDownload(dir, filesBefore, timeout) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    await sleep(Math.min(500, deadline - Date.now()));
    const current = readdirSync(dir);
    const newFiles = current.filter(
      (fileName) => !filesBefore.has(fileName) && !fileName.endsWith(".crdownload"),
    );
    const stillDownloading = current.some((fileName) => fileName.endsWith(".crdownload"));
    if (newFiles.length > 0 && !stillDownloading) {
      return join(dir, newFiles[0]);
    }
  }
  throw new Error(`Download timed out after ${timeout}ms`);
}

async function findLatestStooqDailyLink(page) {
  const latest = await page.evaluate(() => {
    const anchor = document.querySelector('a[href*="db/d/?d="][href*="&t=d"]');
    if (!(anchor instanceof HTMLAnchorElement)) {
      return null;
    }

    const row = anchor.closest("tr");
    const label = anchor.textContent?.trim() || "";
    const listedDate = row?.cells?.[0]?.textContent?.trim() || "";
    return {
      downloadUrl: anchor.href,
      label,
      listedDate,
    };
  });

  if (!latest) {
    throw new Error("Latest Stooq daily link not found");
  }

  const match = latest.downloadUrl.match(/[?&]d=(\d{8})\b/);
  if (!match) {
    throw new Error(`Unexpected Stooq daily URL: ${latest.downloadUrl}`);
  }

  return {
    date: match[1],
    label: latest.label || latest.listedDate,
    downloadUrl: latest.downloadUrl,
  };
}

function validateStooqDailyDate(rawDate) {
  if (rawDate === undefined || rawDate === null || rawDate === "") {
    return null;
  }

  const requestedDate = String(rawDate).trim();
  if (!/^\d{8}$/.test(requestedDate)) {
    throw new Error("Invalid Stooq daily date");
  }
  return requestedDate;
}

function validateStooqBundle(rawBundle) {
  if (rawBundle === undefined || rawBundle === null || rawBundle === "") {
    return null;
  }

  const bundle = String(rawBundle).trim();
  if (!/^[0-9a-z_]+$/.test(bundle)) {
    throw new Error("Invalid Stooq bundle");
  }
  return bundle;
}

function buildStooqDailyDownloadUrl(date) {
  return `https://stooq.com/db/d/?d=${date}&t=d`;
}

function buildStooqBundleDownloadUrl(bundle) {
  return `https://stooq.com/db/d/?b=${bundle}`;
}

async function openStooqCaptcha(page, downloadUrl, deadline) {
  await page.evaluate((href) => window.cpt_g(href, 1, 1), downloadUrl);

  await page.waitForFunction(
    () => {
      const dialog = document.getElementById("cpt");
      const captcha = document.querySelector("#cpt_cd img");
      if (!dialog || !captcha) {
        return false;
      }
      const visible = window.getComputedStyle(dialog).display !== "none";
      return visible && captcha.complete && captcha.naturalWidth > 0 && captcha.naturalHeight > 0;
    },
    { timeout: remainingTimeout(deadline) },
  );

  const captchaHandle = await page.$("#cpt_cd img");
  if (!captchaHandle) {
    throw new Error("Stooq CAPTCHA image not found");
  }

  const captchaImageBase64 = await captchaHandle.screenshot({ encoding: "base64" });
  await captchaHandle.dispose();
  return captchaImageBase64;
}

async function approveStooqCaptcha(page, captchaCode, deadline) {
  const normalizedCode = String(captchaCode || "").trim().toLowerCase();
  if (!/^[0-9a-z]{4}$/.test(normalizedCode)) {
    throw new Error("Invalid Stooq CAPTCHA code");
  }

  await page.waitForSelector('input[name="cpt_t"]', { timeout: remainingTimeout(deadline) });
  await page.$eval(
    'input[name="cpt_t"]',
    (input, value) => {
      input.value = value;
    },
    normalizedCode,
  );
  await page.evaluate(() => {
    const alertRow = document.getElementById("cpt_al");
    if (alertRow) {
      alertRow.style.display = "none";
    }
  });
  await page.evaluate(() => window.cpt_a());

  await page.waitForFunction(
    () => {
      const approved = window.ap === 1 && document.getElementById("cpt_gh");
      const alertRow = document.getElementById("cpt_al");
      const rejected = window.ap !== 1
        && alertRow
        && window.getComputedStyle(alertRow).display !== "none";
      return approved || rejected;
    },
    { timeout: remainingTimeout(deadline) },
  );

  const approved = await page.evaluate(() => window.ap === 1);
  if (!approved) {
    throw new Error("Stooq CAPTCHA rejected");
  }
}

const app = express();
app.use(express.json({ limit: "1mb" }));

app.get("/health", (_req, res) => {
  res.json({ status: "ok", pool_size: browserPool.size });
});

app.post("/fetch", async (req, res) => {
  const { url, proxy, proxyType, proxyUsername, proxyPassword, timeout } = req.body;
  if (!url) {
    return res.status(400).json({ error: "url is required" });
  }

  const proxyAddr = proxy || "direct";
  const key = poolKey(proxyAddr, proxyUsername);
  const pageTimeout = timeout || PAGE_TIMEOUT;
  const deadline = Date.now() + pageTimeout;

  let page = null;
  try {
    ({ page } = await openRequestPage(proxyAddr, proxyType, proxyUsername, proxyPassword));

    if (proxyUsername && proxyPassword !== undefined) {
      await page.authenticate({ username: proxyUsername, password: proxyPassword });
    }

    await navigateWithChallengeWait(page, url, deadline);
    const html = await page.content();

    res.json({ html, status: 200 });
  } catch (error) {
    await closeBrowser(key);
    res.status(502).json({
      error: error.message,
      status: 502,
      html: null,
    });
  } finally {
    if (page) {
      try {
        await page.close();
      } catch (error) {
        console.debug("page.close() failed after fetch:", error?.message || error);
      }
    }
  }
});

app.post("/download", async (req, res) => {
  const { url, downloadDir, selector, proxy, proxyType, proxyUsername, proxyPassword, timeout } = req.body;
  if (!url || !downloadDir) {
    return res.status(400).json({ error: "url and downloadDir are required" });
  }

  const pageTimeout = timeout || PAGE_TIMEOUT;
  const proxyAddr = proxy || "direct";
  const key = poolKey(proxyAddr, proxyUsername);
  const deadline = Date.now() + pageTimeout;
  let page = null;

  try {
    if (!existsSync(downloadDir)) {
      mkdirSync(downloadDir, { recursive: true });
    }
    const filesBefore = new Set(readdirSync(downloadDir));

    ({ page } = await openRequestPage(proxyAddr, proxyType, proxyUsername, proxyPassword));

    if (proxyUsername && proxyPassword !== undefined) {
      await page.authenticate({ username: proxyUsername, password: proxyPassword });
    }

    const client = await page.createCDPSession();
    await client.send("Page.setDownloadBehavior", {
      behavior: "allow",
      downloadPath: downloadDir,
    });

    const navigationResult = await navigateWithChallengeWait(
      page,
      url,
      deadline,
      selector == null,
    );
    if (selector) {
      if (navigationResult.downloadStarted) {
        throw new Error("Download started before selector interaction");
      }
      await page.waitForSelector(selector, { timeout: remainingTimeout(deadline) });
      await page.click(selector);
    }

    const filePath = await waitForDownload(downloadDir, filesBefore, remainingTimeout(deadline));
    res.json({ filePath, status: 200 });
  } catch (error) {
    await closeBrowser(key);
    res.status(502).json({ error: error.message, status: 502, filePath: null });
  } finally {
    if (page) {
      try {
        await page.close();
      } catch (error) {
        console.debug("page.close() failed after download:", error?.message || error);
      }
    }
  }
});

app.post("/evaluate", async (req, res) => {
  const { url, script, proxy, proxyType, proxyUsername, proxyPassword, timeout } = req.body;
  if (!url || !script) {
    return res.status(400).json({ error: "url and script are required" });
  }

  const proxyAddr = proxy || "direct";
  const key = poolKey(proxyAddr, proxyUsername);
  const pageTimeout = timeout || PAGE_TIMEOUT;
  const deadline = Date.now() + pageTimeout;

  let page = null;
  try {
    ({ page } = await openRequestPage(proxyAddr, proxyType, proxyUsername, proxyPassword));

    if (proxyUsername && proxyPassword !== undefined) {
      await page.authenticate({ username: proxyUsername, password: proxyPassword });
    }

    await navigateWithChallengeWait(page, url, deadline);

    // popup（window.open）のURLをキャプチャ
    let capturedPopupUrl = null;
    const popupHandler = async (popup) => {
      capturedPopupUrl = popup.url();
      try { await popup.close(); } catch (_) { /* ignore */ }
    };
    page.on("popup", popupHandler);

    const result = await page.evaluate(script);
    page.off("popup", popupHandler);

    // popupでキャプチャしたURLを優先して返す
    res.json({ result: capturedPopupUrl || result, status: 200 });
  } catch (error) {
    await closeBrowser(key);
    res.status(502).json({ error: error.message, status: 502 });
  } finally {
    if (page) {
      try {
        await page.close();
      } catch (error) {
        console.debug("page.close() failed after evaluate:", error?.message || error);
      }
    }
  }
});

app.post("/stooq/prepare-daily-download", async (req, res) => {
  const pageTimeout = req.body.timeout || PAGE_TIMEOUT;
  let requestedDate = null;
  let requestedBundle = null;
  try {
    requestedDate = validateStooqDailyDate(req.body.date);
    requestedBundle = validateStooqBundle(req.body.bundle);
  } catch (error) {
    return res.status(400).json({ error: error.message, status: 400 });
  }
  if (requestedDate && requestedBundle) {
    return res.status(400).json({
      error: "date and bundle cannot both be provided",
      status: 400,
    });
  }

  const key = poolKey("direct");
  const deadline = Date.now() + pageTimeout;
  let page = null;

  try {
    ({ page } = await openRequestPage("direct"));
    await navigateWithChallengeWait(page, "https://stooq.com/db/", deadline);

    let dailyFile;
    if (requestedBundle) {
      dailyFile = {
        date: requestedBundle,
        label: requestedBundle,
        downloadUrl: buildStooqBundleDownloadUrl(requestedBundle),
      };
    } else if (requestedDate) {
      dailyFile = {
        date: requestedDate,
        label: `${requestedDate}_d`,
        downloadUrl: buildStooqDailyDownloadUrl(requestedDate),
      };
    } else {
      dailyFile = await findLatestStooqDailyLink(page);
    }
    const captchaImageBase64 = await openStooqCaptcha(page, dailyFile.downloadUrl, deadline);
    const sessionId = randomUUID();

    stooqSessions.set(sessionId, {
      key,
      page,
      lastUsed: Date.now(),
    });
    page = null;

    res.json({
      sessionId,
      date: dailyFile.date,
      label: dailyFile.label,
      downloadUrl: dailyFile.downloadUrl,
      captchaImageBase64,
      status: 200,
    });
  } catch (error) {
    await closeBrowser(key);
    res.status(502).json({ error: error.message, status: 502 });
  } finally {
    if (page) {
      try {
        await page.close();
      } catch (error) {
        console.debug("page.close() failed after Stooq prepare:", error?.message || error);
      }
    }
  }
});

app.post("/stooq/complete-daily-download", async (req, res) => {
  const {
    sessionId,
    captchaCode,
    downloadDir,
    timeout,
  } = req.body;
  if (!sessionId || !captchaCode || !downloadDir) {
    return res.status(400).json({
      error: "sessionId, captchaCode and downloadDir are required",
    });
  }

  const session = touchStooqSession(sessionId);
  if (!session) {
    return res.status(404).json({ error: `Unknown Stooq session: ${sessionId}` });
  }

  const pageTimeout = timeout || PAGE_TIMEOUT;
  const deadline = Date.now() + pageTimeout;
  const { key, page } = session;

  try {
    if (!existsSync(downloadDir)) {
      mkdirSync(downloadDir, { recursive: true });
    }
    const filesBefore = new Set(readdirSync(downloadDir));

    const client = await page.createCDPSession();
    await client.send("Page.setDownloadBehavior", {
      behavior: "allow",
      downloadPath: downloadDir,
    });

    await approveStooqCaptcha(page, captchaCode, deadline);
    await page.waitForSelector("#cpt_gh", { timeout: remainingTimeout(deadline) });
    await page.click("#cpt_gh");

    const filePath = await waitForDownload(downloadDir, filesBefore, remainingTimeout(deadline));
    res.json({ filePath, status: 200 });
  } catch (error) {
    await closeBrowser(key);
    res.status(502).json({ error: error.message, status: 502 });
  } finally {
    await closeStooqSession(sessionId);
  }
});

app.post("/stooq/close-session", async (req, res) => {
  const { sessionId } = req.body;
  if (!sessionId) {
    return res.status(400).json({ error: "sessionId is required" });
  }

  await closeStooqSession(sessionId);
  return res.json({ status: 200 });
});

app.post("/shutdown", async (_req, res) => {
  res.json({ status: "shutting_down" });
  await closeAllStooqSessions();
  await closeAllBrowsers();
  process.exit(0);
});

const server = app.listen(0, "127.0.0.1", () => {
  const { port } = server.address();
  console.log(`BROWSER_SERVICE_PORT=${port}`);
});

for (const signal of ["SIGTERM", "SIGINT"]) {
  process.on(signal, async () => {
    await closeAllStooqSessions();
    await closeAllBrowsers();
    server.close();
    process.exit(0);
  });
}
