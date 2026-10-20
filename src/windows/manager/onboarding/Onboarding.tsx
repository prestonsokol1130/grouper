import { useState } from "react";
import { Button } from "../../../components/Button";
import { useGroup } from "../../../store/hooks";
import { saveSettings } from "../settings/settingsActions";
import { nextStep, previousStep, STEPS, stepLabel, type Step } from "./onboardingSteps";
import { GroupStep, PinStep, ScanStep, WelcomeStep } from "./StepPages";

/** The step's main button: what it says, and whether it finishes. */
function nextLabel(step: Step, ready: boolean): string {
  if (step === "pin") return "Done";
  return ready ? "Next" : "Skip";
}

/**
 * The first-run welcome (shown while `Settings.first_run_done` is false):
 * what Grouper does, scanning the Start Menu, making a first group with
 * the Add apps picker, and pinning its shortcut. Every step can be
 * skipped, and so can the whole thing; finishing or skipping saves
 * `first_run_done`, which swaps this for the normal Manager.
 */
export function Onboarding() {
  const [step, setStep] = useState<Step>("welcome");
  const [groupId, setGroupId] = useState<string | null>(null);
  const [found, setFound] = useState<number | null>(null);
  const [finishing, setFinishing] = useState(false);
  const pinned = useGroup(groupId ?? "")?.hasShortcut === true;

  const finish = () => {
    setFinishing(true);
    void saveSettings({ firstRunDone: true }).then((ok) => {
      if (!ok) setFinishing(false);
    });
  };
  const forward = () => {
    const next = nextStep(step);
    if (next === null) finish();
    else setStep(next);
  };
  const back = previousStep(step);
  const ready =
    step === "welcome" || (step === "scan" && found !== null) || (step === "group" && pinned);

  return (
    <div className="flex h-screen items-center justify-center overflow-y-auto bg-layer p-6 text-primary">
      <section
        aria-label="Welcome to Grouper"
        className="flex max-h-full w-[640px] max-w-full flex-col rounded-window border border-stroke-card bg-card shadow-raised"
      >
        <header className="flex items-center gap-3 px-8 pt-6">
          <p className="flex-1 type-caption text-secondary">{stepLabel(step)}</p>
          <div className="flex items-center gap-1.5" aria-hidden="true">
            {STEPS.map((dot) => (
              <span
                key={dot}
                className="step-dot"
                aria-current={dot === step ? "step" : undefined}
              />
            ))}
          </div>
        </header>
        <div className="min-h-[320px] flex-1 overflow-y-auto px-8 py-6">
          {step === "welcome" && <WelcomeStep />}
          {step === "scan" && <ScanStep found={found} onFound={setFound} />}
          {step === "group" && (
            <GroupStep groupId={groupId} onGroup={setGroupId} onShortcut={() => setStep("pin")} />
          )}
          {step === "pin" && <PinStep groupId={pinned ? groupId : null} />}
        </div>
        <footer className="flex items-center gap-2 border-t border-stroke-card px-8 py-4">
          {step !== "pin" && (
            <Button variant="ghost" onClick={finish} disabled={finishing}>
              Skip setup
            </Button>
          )}
          <div className="flex-1" />
          {back !== null && <Button onClick={() => setStep(back)}>Back</Button>}
          <Button variant="primary" onClick={forward} disabled={finishing}>
            {nextLabel(step, ready)}
          </Button>
        </footer>
      </section>
    </div>
  );
}
