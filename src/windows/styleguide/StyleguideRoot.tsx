import { isTauri } from "@tauri-apps/api/core";
import { useEffect } from "react";
import { setPreviewIconUrls } from "../../lib/iconUrl";
import { useAppStore } from "../../store/appStore";
import { showToast } from "../../store/toastStore";
import { ComponentsSection } from "./ComponentsSection";
import { SAMPLE_STATE, sampleIconUrls } from "./sampleData";
import { TokensSection } from "./TokensSection";
import { MotionSection, TypeSection } from "./TypeMotionSection";

// Sample data before the first render. Outside Tauri the sample icons are
// data URLs; inside Tauri the CSP blocks those, so tiles show placeholders.
if (!isTauri()) setPreviewIconUrls(sampleIconUrls());
useAppStore.getState().applyState(SAMPLE_STATE);

/**
 * Dev-only style guide (`?window=styleguide`): every DESIGN.md token, the
 * type ramp, the motion presets and the shared components in each state.
 * Switch Windows (or the browser) between light and dark to see both.
 */
export default function StyleguideRoot() {
  useEffect(() => {
    if (new URLSearchParams(window.location.search).has("toast")) {
      showToast({
        kind: "error",
        title: "Couldn't rename the group.",
        message: "Please try again.",
      });
    }
  }, []);

  return (
    <main className="min-h-screen bg-layer p-8 text-primary">
      <h1 className="type-title-large mb-2">Grouper style guide</h1>
      <p className="type-body mb-8 text-secondary">
        Theme: {document.documentElement.dataset.theme}. Values come from DESIGN.md.
      </p>
      <TokensSection />
      <TypeSection />
      <MotionSection />
      <ComponentsSection />
    </main>
  );
}
