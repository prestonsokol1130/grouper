import { useState } from "react";
import { iconUrl } from "../lib/iconUrl";
import { useIconVersion } from "../store/hooks";

type Props = {
  appId: string;
  /** Side in CSS pixels; the box never changes size, so nothing shifts. */
  size: number;
  /** Corner radius, e.g. the folder tile's 22% for mini icons. */
  radius?: string;
};

/**
 * An app's cached icon. Uses the store's `iconVersion` as the cache-buster
 * so it reloads when the icon changes, and switches to the fallback icon
 * when the app's own PNG fails to load. Decorative (`alt=""`): the tile
 * around it carries the name.
 */
export function AppIcon({ appId, size, radius }: Props) {
  const version = useIconVersion();
  // URLs that failed to load. A new version gives new URLs, so a bumped
  // icon is tried again.
  const [failed, setFailed] = useState<ReadonlySet<string>>(() => new Set());
  const candidates = [iconUrl(appId, version), iconUrl("_fallback", version)];
  const src = candidates.find((url): url is string => url !== null && !failed.has(url));
  const box = { width: size, height: size, borderRadius: radius };

  if (src === undefined) {
    return <span aria-hidden="true" className="inline-block shrink-0" style={box} />;
  }
  return (
    <img
      src={src}
      alt=""
      width={size}
      height={size}
      decoding="async"
      draggable={false}
      className="block shrink-0 select-none object-contain"
      style={box}
      onError={() => setFailed((previous) => new Set(previous).add(src))}
    />
  );
}
