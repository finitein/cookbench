import type { CSSProperties } from "react";
import type { StoveWire } from "../types/stove";

const statusLabels: Record<string, string> = {
  starting: "Starting",
  planning: "Planning",
  cooking: "Cooking",
  needsHuman: "Needs human",
  cooked: "Cooked",
  failed: "Failed",
  disconnected: "Disconnected",
};

const centerLabels: Record<string, string> = {
  needsHuman: "Needs human",
  disconnected: "Offline",
};

export function stoveStateLabel(state: string) {
  return statusLabels[state] ?? state;
}

export function ProgressRing({ stove }: { stove: StoveWire }) {
  const state = stove.state;
  const progress = stove.progress;
  const isCooking = state === "cooking";
  const hasStructuredProgress = Boolean(
    isCooking && progress && progress.total > 0,
  );
  const percentage = hasStructuredProgress
    ? Math.max(0, Math.min(100, Math.round(((progress?.completed ?? 0) / (progress?.total ?? 1)) * 100)))
    : 100;
  const stateLabel = stoveStateLabel(state);
  const ringMode = hasStructuredProgress ? "determinate" : isCooking ? "indeterminate" : "complete";
  const ringMotion = ringMode === "indeterminate" ? "rotate" : "static";
  const centerLabel = hasStructuredProgress ? `${percentage}%` : centerLabels[state] ?? stateLabel;

  return (
    <span
      className={`progress-ring progress-ring--${state} progress-ring--${ringMode}`}
      data-testid="progress-ring"
      data-ring-mode={ringMode}
      data-ring-motion={ringMotion}
      data-progress={hasStructuredProgress ? percentage : undefined}
      aria-label={hasStructuredProgress ? `${stateLabel}, ${percentage}% complete` : stateLabel}
    >
      <svg viewBox="0 0 56 56" aria-hidden="true" focusable="false">
        <circle className="progress-ring__track" cx="28" cy="28" r="22" pathLength="100" />
        <circle
          className="progress-ring__value"
          cx="28"
          cy="28"
          r="22"
          pathLength="100"
          style={{ "--ring-progress": percentage } as CSSProperties}
        />
      </svg>
      <span className="progress-ring__label" aria-hidden="true">{centerLabel}</span>
    </span>
  );
}
