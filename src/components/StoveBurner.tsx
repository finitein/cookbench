import type { StoveWire } from "../types/stove";
import { motionForStoveTransition, type StoveMotionPreferences, type StoveMotionState } from "../animation/stoveMotion";
import { HarnessMark } from "./HarnessMark";
import { HostBadge } from "./HostBadge";
import { ProgressRing, stoveStateLabel } from "./ProgressRing";
import { StoveTooltip } from "./StoveTooltip";

export type StoveBurnerProps = {
  stove: StoveWire;
  onActivate?: (stove: StoveWire) => void;
  onDetach?: (stove: StoveWire) => void;
  onClear?: (stove: StoveWire) => void;
  previousState?: StoveMotionState;
  isInitialSnapshot?: boolean;
  motionPreferences?: StoveMotionPreferences;
};

export function StoveBurner({ stove, onActivate, onDetach, onClear, previousState, isInitialSnapshot, motionPreferences }: StoveBurnerProps) {
  const tooltipId = `stove-tooltip-${stove.id}`;
  const stateLabel = stoveStateLabel(stove.state);
  const sessionLabel = stove.taskTitle ?? "Current session";
  const projectLabel = stove.projectLabel;
  const motion = motionForStoveTransition(
    { previousState, nextState: stove.state, isInitialSnapshot },
    motionPreferences ?? { reducedMotion: false, soundEnabled: false },
  );

  return (
    <div className="stove-burner-wrap">
      <button
        className="stove-burner"
        data-testid="stove"
        data-state={stove.state}
        data-completion={motion.completion}
        data-motion={motion.completion === "none" ? "system" : motion.completion}
        type="button"
        aria-describedby={tooltipId}
        aria-label={`${stove.harness.label}: ${projectLabel}, ${sessionLabel}, ${stateLabel}`}
        onClick={() => onActivate?.(stove)}
      >
        <span className="stove-burner__ring"><ProgressRing stove={stove} /></span>
        <span className="stove-burner__identity"><HarnessMark harness={stove.harness} /></span>
        <HostBadge stove={stove} />
      </button>
      <StoveTooltip stove={stove} id={tooltipId} />
      {onDetach ? (
        <button
          className="stove-burner__control stove-burner__control--detach"
          type="button"
          aria-label={`Detach ${stove.harness.label} Stove`}
          title={`Detach ${stove.harness.label} Stove`}
          onClick={() => onDetach(stove)}
        >
          <span aria-hidden="true" />
        </button>
      ) : null}
      {onClear && stove.retainedCompletion ? (
        <button
          className="stove-burner__control stove-burner__control--clear"
          type="button"
          aria-label={`Clear ${stove.harness.label} Stove`}
          title={`Clear ${stove.harness.label} Stove`}
          onClick={() => onClear(stove)}
        >
          <span aria-hidden="true" />
        </button>
      ) : null}
    </div>
  );
}
