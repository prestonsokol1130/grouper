import type { ReactNode } from "react";
import type { PreviewState } from "../../components/activation";
import { AppTile } from "../../components/AppTile";
import { Button, IconButton, type ButtonVariant } from "../../components/Button";
import { FolderTile } from "../../components/FolderTile";
import { useToast } from "../../store/toastStore";
import { Section } from "./TokensSection";

const STATES: (PreviewState | undefined)[] = [undefined, "hover", "pressed", "focus", "disabled"];

/** One labelled cell in a state row. */
function Cell({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col items-center gap-2">
      {children}
      <span className="type-caption text-secondary">{label}</span>
    </div>
  );
}

function Row({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="mb-6">
      <h3 className="type-body-strong mb-2">{title}</h3>
      <div className="flex flex-wrap items-end gap-6">{children}</div>
    </div>
  );
}

const VARIANTS: ButtonVariant[] = ["primary", "secondary", "ghost"];

/** Every shared component in every state. */
export function ComponentsSection() {
  const { show } = useToast();
  return (
    <Section title="Components">
      <Row title="AppTile: rest, hover, pressed, focus, disabled">
        {STATES.map((state) => (
          <Cell key={state ?? "rest"} label={state ?? "rest"}>
            <AppTile appId="mail" preview={state} disabled={state === "disabled"} />
          </Cell>
        ))}
      </Row>
      <Row title="AppTile: long name, missing app, broken icon, sizes">
        <Cell label="truncated">
          <AppTile appId="long" />
        </Cell>
        <Cell label="missing">
          <AppTile appId="gone" />
        </Cell>
        <Cell label="fallback icon">
          <AppTile appId="broken" />
        </Cell>
        {[40, 48, 56, 64, 96].map((size) => (
          <Cell key={size} label={`${size}px`}>
            <AppTile appId="music" size={size} />
          </Cell>
        ))}
        <Cell label="no label">
          <AppTile appId="maps" showLabel={false} />
        </Cell>
      </Row>
      <Row title="FolderTile: 5, 3, 2, 1, missing/hidden skipped, empty">
        {["work", "three", "two", "one", "skips", "empty"].map((id) => (
          <Cell key={id} label={id}>
            <FolderTile groupId={id} />
          </Cell>
        ))}
      </Row>
      <Row title="FolderTile: iPhone squircle option, states">
        {STATES.filter((state) => state !== "disabled").map((state) => (
          <Cell key={state ?? "rest"} label={state ?? "rest"}>
            <FolderTile groupId="work" shape="squircle" preview={state} />
          </Cell>
        ))}
      </Row>
      {VARIANTS.map((variant) => (
        <Row key={variant} title={`Button ${variant}`}>
          {STATES.map((state) => (
            <Cell key={state ?? "rest"} label={state ?? "rest"}>
              <Button variant={variant} preview={state}>
                Add apps
              </Button>
            </Cell>
          ))}
        </Row>
      ))}
      <Row title="IconButton (ghost, secondary)">
        {STATES.map((state) => (
          <Cell key={state ?? "rest"} label={state ?? "rest"}>
            <div className="flex gap-2">
              <IconButton label="More" preview={state}>
                <span aria-hidden="true">⋯</span>
              </IconButton>
              <IconButton label="Close" variant="secondary" preview={state}>
                <span aria-hidden="true">✕</span>
              </IconButton>
            </div>
          </Cell>
        ))}
      </Row>
      <Row title="Toasts (bottom-right)">
        <Button
          onClick={() =>
            show({
              kind: "error",
              title: "Couldn't open Spotify.",
              message: "It may have been moved or uninstalled. Click to find it.",
            })
          }
        >
          Error toast
        </Button>
        <Button
          onClick={() => show({ kind: "success", title: "Shortcut created.", message: "Pin it." })}
        >
          Success toast
        </Button>
        <Button
          onClick={() =>
            show({
              kind: "info",
              title: "Alt + ` is already used by another app.",
              message: "Pick a different shortcut in Settings.",
              action: { label: "Open Settings", run: () => undefined },
            })
          }
        >
          Toast with action
        </Button>
      </Row>
    </Section>
  );
}
