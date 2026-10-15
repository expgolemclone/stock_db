import { readdirSync, statSync } from "node:fs";
import { join } from "node:path";

function sleep(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

export function snapshotDownloadDir(dir) {
  const snapshot = new Map();
  for (const fileName of readdirSync(dir)) {
    try {
      const stat = statSync(join(dir, fileName));
      if (stat.isFile()) {
        snapshot.set(fileName, {
          mtimeMs: stat.mtimeMs,
          size: stat.size,
        });
      }
    } catch {
      // Files can appear or disappear while Chrome is moving downloads.
    }
  }
  return snapshot;
}

function changedSince(snapshot, fileName, stat) {
  const previous = snapshot.get(fileName);
  return (
    !previous
    || previous.size !== stat.size
    || stat.mtimeMs > previous.mtimeMs
  );
}

function readDownloadState(dir, snapshot) {
  const completed = [];
  let hasActiveDownload = false;

  for (const fileName of readdirSync(dir)) {
    let stat;
    try {
      stat = statSync(join(dir, fileName));
    } catch {
      continue;
    }
    if (!stat.isFile() || !changedSince(snapshot, fileName, stat)) {
      continue;
    }

    if (fileName.endsWith(".crdownload")) {
      hasActiveDownload = true;
      continue;
    }

    completed.push({
      fileName,
      mtimeMs: stat.mtimeMs,
    });
  }

  completed.sort((a, b) => b.mtimeMs - a.mtimeMs || a.fileName.localeCompare(b.fileName));
  return { completed, hasActiveDownload };
}

export async function waitForDownload(
  dir,
  snapshot,
  timeout,
  { pollIntervalMs = 500 } = {},
) {
  const deadline = Date.now() + timeout;
  while (true) {
    const { completed, hasActiveDownload } = readDownloadState(dir, snapshot);
    if (completed.length > 0 && !hasActiveDownload) {
      return join(dir, completed[0].fileName);
    }

    const remaining = deadline - Date.now();
    if (remaining <= 0) {
      break;
    }
    await sleep(Math.min(pollIntervalMs, remaining));
  }

  throw new Error(`Download timed out after ${timeout}ms`);
}
