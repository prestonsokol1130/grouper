/** The first-run steps, in order (ticket S6-01 part B). */
export const STEPS = ["welcome", "scan", "group", "pin"] as const;

export type Step = (typeof STEPS)[number];

/** The step after `step`, or `null` after the last one (finished). */
export function nextStep(step: Step): Step | null {
  return STEPS[STEPS.indexOf(step) + 1] ?? null;
}

/** The step before `step`, or `null` on the first one. */
export function previousStep(step: Step): Step | null {
  const index = STEPS.indexOf(step);
  return index > 0 ? (STEPS[index - 1] ?? null) : null;
}

/** "Step 2 of 4". */
export function stepLabel(step: Step): string {
  return `Step ${STEPS.indexOf(step) + 1} of ${STEPS.length}`;
}
