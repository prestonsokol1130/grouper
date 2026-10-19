import { attachLogger, error as logError, LogLevel } from "@tauri-apps/plugin-log";

/** The real console methods, captured before `console.error` is wrapped. */
const original = {
  log: console.log.bind(console),
  debug: console.debug.bind(console),
  info: console.info.bind(console),
  warn: console.warn.bind(console),
  error: console.error.bind(console),
};

/** Turns any logged value into readable text. */
function describe(value: unknown): string {
  if (value instanceof Error) {
    return value.stack ?? `${value.name}: ${value.message}`;
  }
  if (typeof value === "string") {
    return value;
  }
  try {
    return JSON.stringify(value) ?? String(value);
  } catch {
    return String(value);
  }
}

/** Sends one error line to the Rust log; ignores failures (e.g. outside Tauri). */
function forward(label: string, message: string): void {
  logError(`[${label}] ${message}`).catch(() => undefined);
}

/** Prints Rust log records in the devtools console using the unwrapped methods. */
function mirrorToConsole(level: LogLevel, message: string): void {
  if (level === LogLevel.Error) original.error(message);
  else if (level === LogLevel.Warn) original.warn(message);
  else if (level === LogLevel.Info) original.info(message);
  else if (level === LogLevel.Debug) original.debug(message);
  else original.log(message);
}

/**
 * Forwards this window's console errors, uncaught errors and unhandled
 * rejections to `grouper.log`. In dev, Rust log records are also shown in the
 * devtools console (the plugin's `attachConsole`, rebuilt on the unwrapped
 * console methods so forwarded errors cannot echo back in a loop).
 */
export function installErrorForwarding(label: string): void {
  console.error = (...args: unknown[]) => {
    original.error(...args);
    forward(label, args.map(describe).join(" "));
  };
  window.addEventListener("error", (event) => {
    forward(label, `uncaught: ${describe(event.error ?? event.message)}`);
  });
  window.addEventListener("unhandledrejection", (event) => {
    forward(label, `unhandled rejection: ${describe(event.reason)}`);
  });
  if (import.meta.env.DEV) {
    attachLogger(({ level, message }) => mirrorToConsole(level, message)).catch(() => undefined);
  }
}
