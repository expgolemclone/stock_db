export const STOOQ_DAILY_LINK_SELECTOR = 'a[href*="db/d/?d="]';

function parseStooqDailyUrl(downloadUrl) {
  const url = new URL(downloadUrl, "https://stooq.com");
  const date = url.searchParams.get("d") || "";
  if (
    url.pathname !== "/db/d/"
    || !/^\d{8}$/.test(date)
    || url.searchParams.get("t") !== "d"
  ) {
    return null;
  }
  return { date, downloadUrl: url.href };
}

export function pickLatestStooqDailyLink(candidates) {
  for (const candidate of candidates) {
    const parsed = parseStooqDailyUrl(candidate.downloadUrl);
    if (!parsed) {
      continue;
    }

    return {
      date: parsed.date,
      label: candidate.label || candidate.listedDate,
      downloadUrl: parsed.downloadUrl,
    };
  }

  return null;
}

export async function findLatestStooqDailyLink(page, timeout) {
  try {
    await page.waitForFunction(
      (selector) => Array.from(document.querySelectorAll(selector)).some((anchor) => {
        if (!(anchor instanceof HTMLAnchorElement)) {
          return false;
        }
        const url = new URL(anchor.href, window.location.href);
        const date = url.searchParams.get("d") || "";
        return (
          url.pathname === "/db/d/"
          && /^\d{8}$/.test(date)
          && url.searchParams.get("t") === "d"
        );
      }),
      { timeout },
      STOOQ_DAILY_LINK_SELECTOR,
    );
  } catch {
    throw new Error("Latest Stooq daily link not found");
  }

  const candidates = await page.evaluate(
    (selector) => Array.from(document.querySelectorAll(selector)).flatMap((anchor) => {
      if (!(anchor instanceof HTMLAnchorElement)) {
        return [];
      }

      const row = anchor.closest("tr");
      return [{
        downloadUrl: anchor.href,
        label: anchor.textContent?.trim() || "",
        listedDate: row?.cells?.[0]?.textContent?.trim() || "",
      }];
    }),
    STOOQ_DAILY_LINK_SELECTOR,
  );
  const latest = pickLatestStooqDailyLink(candidates);
  if (!latest) {
    throw new Error("Latest Stooq daily link not found");
  }

  return latest;
}
