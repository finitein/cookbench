import { stoveDisplayIdentity, stoveSessionIdentity, type StoveWire } from "../types/stove";
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
  onTooltipVisibilityChange?: (visible: boolean, stove: StoveWire) => void;
  tooltipId?: string;
  renderTooltip?: boolean;
  showHarnessMark?: boolean;
};

export function StoveBurner({
  stove,
  onActivate,
  onDetach,
  onClear,
  previousState,
  isInitialSnapshot,
  motionPreferences,
  onTooltipVisibilityChange,
  tooltipId: suppliedTooltipId,
  renderTooltip = true,
  showHarnessMark = true,
}: StoveBurnerProps) {
  const tooltipId = suppliedTooltipId ?? `stove-tooltip-${stove.id}`;
  const hasTooltip = renderTooltip || Boolean(onTooltipVisibilityChange);
  const stateLabel = stoveStateLabel(stove.state);
  const sessionLabel = stove.taskTitle ?? "Current session";
  const projectLabel = stove.projectLabel ?? "Session";
  const sessionIdentity = stoveSessionIdentity(stove);
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
        aria-describedby={hasTooltip ? tooltipId : undefined}
        aria-label={`${stove.harness.label}: ${stoveDisplayIdentity(stove)}, ${sessionLabel}, ${stateLabel}`}
        onClick={() => onActivate?.(stove)}
        onPointerEnter={() => onTooltipVisibilityChange?.(true, stove)}
        onPointerLeave={() => onTooltipVisibilityChange?.(false, stove)}
        onFocus={() => onTooltipVisibilityChange?.(true, stove)}
        onBlur={() => onTooltipVisibilityChange?.(false, stove)}
      >
        <span className="stove-burner__ring"><ProgressRing stove={stove} /></span>
        {showHarnessMark ? <span className="stove-burner__identity"><HarnessMark harness={stove.harness} /></span> : null}
        <span
          className="stove-burner__session"
          data-testid="stove-session-identity"
          title={stoveDisplayIdentity(stove)}
        >
          <span>{projectLabel}</span><b>{sessionIdentity}</b>
        </span>
        <HostBadge stove={stove} />
      </button>
      {renderTooltip ? <StoveTooltip stove={stove} id={tooltipId} /> : null}
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
