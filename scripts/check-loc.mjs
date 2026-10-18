#!/usr/bin/env node
/**
 * Fail if any .rs / .ts / .tsx file has 250 or more lines
 * (blank lines and comments count).
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, extname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(fileURLToPath(new URL(".", import.meta.url)), "..");
const LIMIT = 250;
const EXTENSIONS = new Set([".rs", ".ts", ".tsx"]);
const SKIP_DIRS = new Set([
  "node_modules",
  "target",
  "dist",
  ".git",
  "gen",
]);

/** Skip src-tauri/gen and any nested gen/, plus listed dir names. */
function shouldSkipDir(name, absPath) {
  if (SKIP_DIRS.has(name)) return true;
  const rel = relative(ROOT, absPath).replace(/\\/g, "/");
  if (rel === "src-tauri/gen" || rel.startsWith("src-tauri/gen/")) return true;
  return false;
}

function walk(dir, out) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    const abs = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (shouldSkipDir(entry.name, abs)) continue;
      walk(abs, out);
      continue;
    }
    if (!entry.isFile()) continue;
    const ext = extname(entry.name);
    if (!EXTENSIONS.has(ext)) continue;
    out.push(abs);
  }
}

function countLines(path) {
  const text = readFileSync(path, "utf8");
  if (text.length === 0) return 0;
  const parts = text.split(/\r?\n/);
  // Trailing newline yields an empty final segment — do not count it as a line.
  if (parts.length > 0 && parts[parts.length - 1] === "") {
    return parts.length - 1;
  }
  return parts.length;
}

const files = [];
walk(ROOT, files);
files.sort();

const offenders = [];
for (const abs of files) {
  const lines = countLines(abs);
  if (lines >= LIMIT) {
    offenders.push({ path: relative(ROOT, abs).replace(/\\/g, "/"), lines });
  }
}

if (offenders.length === 0) {
  console.log(
    `check-loc: ok (${files.length} file(s) under ${LIMIT} lines)`,
  );
  process.exit(0);
}

console.error(`check-loc: ${offenders.length} file(s) at or over ${LIMIT} lines:`);
for (const { path, lines } of offenders) {
  console.error(`  ${path}: ${lines}`);
}
process.exit(1);
