import { stoveDisplayIdentity, stoveSessionIdentity, type StoveWire } from "../types/stove";
import { motionForStoveTransition, type StoveMotionPreferences, type StoveMotionState } from "../animation/stoveMotion";
import { HarnessMark } from "./HarnessMark";
import { HostBadge } from "./HostBadge";
import { ProgressRing, stoveStateLabel } from "./ProgressRing";
import { StoveTooltip } from "./StoveTooltip";
import { useI18n } from "../i18n/i18n";

export type StoveBurnerProps = {
  stove: StoveWire;
  onActivate?: (stove: StoveWire) => void;
  onDetach?: (stove: StoveWire) => void;
  onClear?: (stove: StoveWire) => void;
  onPin?: (stove: StoveWire) => void;
  onArchive?: (stove: StoveWire) => void;
  previousState?: StoveMotionState;
  isInitialSnapshot?: boolean;
  motionPreferences?: StoveMotionPreferences;
  onTooltipVisibilityChange?: (visible: boolean, stove: StoveWire) => void;
  tooltipId?: string;
  renderTooltip?: boolean;
  showHarnessMark?: boolean;
  flashing?: boolean;
};

export function StoveBurner({
  stove,
  onActivate,
  onDetach,
  onClear,
  onPin,
  onArchive,
  previousState,
  isInitialSnapshot,
  motionPreferences,
  onTooltipVisibilityChange,
  tooltipId: suppliedTooltipId,
  renderTooltip = true,
  showHarnessMark = true,
  flashing = false,
}: StoveBurnerProps) {
  const { t } = useI18n();
  const tooltipId = suppliedTooltipId ?? `stove-tooltip-${stove.id}`;
  const hasTooltip = renderTooltip || Boolean(onTooltipVisibilityChange);
  const stateLabel = stoveStateLabel(stove.state, t);
  const sessionLabel = stove.taskTitle ?? t("common.currentSession");
  const projectLabel = stove.projectLabel ?? t("stove.session");
  const sessionIdentity = stoveSessionIdentity(stove);
  const motion = motionForStoveTransition(
    { previousState, nextState: stove.state, isInitialSnapshot },
    motionPreferences ?? { reducedMotion: false, soundEnabled: false },
  );

  return (
    <div className={`stove-burner-wrap${flashing ? " stove-burner-wrap--alert" : ""}`} data-stove-id={stove.id}>
      <button
        className="stove-burner"
        data-testid="stove"
        data-state={stove.state}
        data-completion={motion.completion}
        data-motion={motion.completion === "none" ? "system" : motion.completion}
        type="button"
        aria-describedby={hasTooltip ? tooltipId : undefined}
        aria-label={`${stove.harness.label}: ${stoveDisplayIdentity(stove, t("stove.session"))}, ${sessionLabel}, ${stateLabel}`}
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
          title={stoveDisplayIdentity(stove, t("stove.session"))}
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
          aria-label={t("stove.detach", { name: stove.harness.label })}
          title={t("stove.detach", { name: stove.harness.label })}
          onClick={() => onDetach(stove)}
        >
          <span aria-hidden="true" />
        </button>
      ) : null}
      {onPin ? (
        <button
          className="stove-burner__control stove-burner__control--pin"
          type="button"
          aria-pressed={stove.pinned}
          aria-label={t(stove.pinned ? "stove.unpin" : "stove.pin", { name: stove.harness.label })}
          title={t(stove.pinned ? "common.unpin" : "common.pin")}
          onClick={() => onPin(stove)}
        >
          <span aria-hidden="true" />
        </button>
      ) : null}
      {onClear && stove.retainedCompletion ? (
        <button
          className="stove-burner__control stove-burner__control--clear"
          type="button"
          aria-label={t("stove.clear", { name: stove.harness.label })}
          title={t("stove.clear", { name: stove.harness.label })}
          onClick={() => onClear(stove)}
        >
          <span aria-hidden="true" />
        </button>
      ) : null}
      {onArchive && !stove.retainedCompletion ? (
        <button
          className="stove-burner__control stove-burner__control--archive"
          type="button"
          aria-label={t("stove.delete", { name: stove.harness.label })}
          title={t("common.delete")}
          onClick={() => onArchive(stove)}
        >
          <span aria-hidden="true" />
        </button>
      ) : null}
    </div>
  );
}
