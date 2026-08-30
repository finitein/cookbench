import { stoveSessionIdentity, type StoveWire } from "../types/stove";
import { stoveStateLabel } from "./ProgressRing";

function duration(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  const remaining = seconds % 60;
  return minutes > 0 ? `${minutes}m ${remaining}s` : `${remaining}s`;
}

export function StoveTooltip({
  stove,
  id,
  className,
}: {
  stove: StoveWire;
  id: string;
  className?: string;
}) {
  const structured = Boolean(stove.progress?.total);
  const elapsed = stove.elapsedMs == null ? "Not reported" : duration(Math.floor(stove.elapsedMs / 1000));

  return (
    <aside className={`stove-tooltip${className ? ` ${className}` : ""}`} id={id} role="tooltip">
      <strong>{stove.projectLabel}</strong>
      <span>{stove.taskTitle ?? "Current session"}</span>
      <dl>
        <div><dt>Harness</dt><dd>{stove.harness.label}</dd></div>
        <div><dt>Host</dt><dd>{stove.host.kind === "ssh" ? `Remote: ${stove.host.id}` : `Local: ${stove.host.id}`}</dd></div>
        <div><dt>Project</dt><dd>{stove.projectLabel ?? "Not reported"}</dd></div>
        <div><dt>Session</dt><dd>{stoveSessionIdentity(stove)}</dd></div>
        <div><dt>State</dt><dd>{stoveStateLabel(stove.state)}</dd></div>
        <div><dt>Activity</dt><dd>{stove.currentAction ?? "Waiting for source activity"}</dd></div>
        <div><dt>Progress</dt><dd>{structured ? `${stove.progress?.completed ?? 0}/${stove.progress?.total} (${stove.progress?.provenance})` : "No structured progress"}</dd></div>
        <div><dt>Elapsed</dt><dd>{elapsed}</dd></div>
        <div><dt>Next</dt><dd>{stove.nextAction ?? "No next action reported"}</dd></div>
      </dl>
    </aside>
  );
}
