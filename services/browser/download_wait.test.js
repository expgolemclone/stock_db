import assert from "node:assert/strict";
import { mkdtempSync, rmSync, unlinkSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import test from "node:test";

import { snapshotDownloadDir, waitForDownload } from "./download_wait.js";

function tempDir() {
  return mkdtempSync(join(tmpdir(), "stock-db-download-wait-"));
}

test("waitForDownload detects an overwritten existing file", async () => {
  const dir = tempDir();
  try {
    const filePath = join(dir, "20260607_d.txt");
    writeFileSync(filePath, "old");
    const snapshot = snapshotDownloadDir(dir);
    writeFileSync(filePath, "new");
    const future = new Date(Date.now() + 1000);
    utimesSync(filePath, future, future);

    const downloaded = await waitForDownload(dir, snapshot, 100, { pollIntervalMs: 1 });

    assert.equal(downloaded, filePath);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("waitForDownload ignores stale crdownload files from the snapshot", async () => {
  const dir = tempDir();
  try {
    writeFileSync(join(dir, "stale.crdownload"), "partial");
    const snapshot = snapshotDownloadDir(dir);
    const filePath = join(dir, "fresh.txt");
    writeFileSync(filePath, "fresh");

    const downloaded = await waitForDownload(dir, snapshot, 100, { pollIntervalMs: 1 });

    assert.equal(downloaded, filePath);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("waitForDownload waits for active crdownload files to disappear", async () => {
  const dir = tempDir();
  try {
    const snapshot = snapshotDownloadDir(dir);
    const partialPath = join(dir, "fresh.txt.crdownload");
    const filePath = join(dir, "fresh.txt");
    writeFileSync(partialPath, "partial");
    writeFileSync(filePath, "fresh");

    setTimeout(() => unlinkSync(partialPath), 10);
    const downloaded = await waitForDownload(dir, snapshot, 200, { pollIntervalMs: 1 });

    assert.equal(basename(downloaded), "fresh.txt");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
