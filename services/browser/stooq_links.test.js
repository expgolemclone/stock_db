import assert from "node:assert/strict";
import test from "node:test";

import {
  STOOQ_DAILY_LINK_SELECTOR,
  findLatestStooqDailyLink,
  pickLatestStooqDailyLink,
} from "./stooq_links.js";

test("pickLatestStooqDailyLink ignores non-daily Stooq bundles", () => {
  const latest = pickLatestStooqDailyLink([
    {
      downloadUrl: "https://stooq.com/db/d/?d=20260607&t=dh",
      label: "0607_dh",
      listedDate: "7 Jun, 10:30",
    },
    {
      downloadUrl: "https://stooq.com/db/d/?d=20260606&t=d",
      label: "0606_d",
      listedDate: "7 Jun, 10:00",
    },
  ]);

  assert.deepEqual(latest, {
    date: "20260606",
    label: "0606_d",
    downloadUrl: "https://stooq.com/db/d/?d=20260606&t=d",
  });
});

test("pickLatestStooqDailyLink uses listed date when the anchor label is empty", () => {
  const latest = pickLatestStooqDailyLink([
    {
      downloadUrl: "db/d/?d=20260605&t=d",
      label: "",
      listedDate: "6 Jun, 12:00",
    },
  ]);

  assert.deepEqual(latest, {
    date: "20260605",
    label: "6 Jun, 12:00",
    downloadUrl: "https://stooq.com/db/d/?d=20260605&t=d",
  });
});

test("findLatestStooqDailyLink waits for the delayed daily table", async () => {
  const calls = [];
  const page = {
    async waitForFunction(_predicate, options, selector) {
      calls.push(["waitForFunction", options.timeout, selector]);
    },
    async evaluate(_mapper, selector) {
      calls.push(["evaluate", selector]);
      return [
        {
          downloadUrl: "https://stooq.com/db/d/?d=20260607&t=d5",
          label: "0607_d5",
          listedDate: "7 Jun, 10:30",
        },
        {
          downloadUrl: "https://stooq.com/db/d/?d=20260606&t=d",
          label: "0606_d",
          listedDate: "7 Jun, 10:00",
        },
      ];
    },
  };

  const latest = await findLatestStooqDailyLink(page, 1234);

  assert.deepEqual(calls, [
    ["waitForFunction", 1234, STOOQ_DAILY_LINK_SELECTOR],
    ["evaluate", STOOQ_DAILY_LINK_SELECTOR],
  ]);
  assert.deepEqual(latest, {
    date: "20260606",
    label: "0606_d",
    downloadUrl: "https://stooq.com/db/d/?d=20260606&t=d",
  });
});

test("findLatestStooqDailyLink keeps the existing not-found error message", async () => {
  const page = {
    async waitForFunction() {
      throw new Error("timeout");
    },
    async evaluate() {
      throw new Error("unexpected evaluate");
    },
  };

  await assert.rejects(
    () => findLatestStooqDailyLink(page, 1),
    /Latest Stooq daily link not found/,
  );
});
