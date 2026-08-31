import { stoveSessionIdentity, type StoveWire } from "../types/stove";
import { stoveStateLabel } from "./ProgressRing";
import { useI18n } from "../i18n/i18n";

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
  const { t } = useI18n();
  const structured = Boolean(stove.progress?.total);
  const elapsed = stove.elapsedMs == null ? t("common.notReported") : duration(Math.floor(stove.elapsedMs / 1000));

  return (
    <aside className={`stove-tooltip${className ? ` ${className}` : ""}`} id={id} role="tooltip">
      <strong>{stove.projectLabel}</strong>
      <span>{stove.taskTitle ?? t("common.currentSession")}</span>
      <dl>
        <div><dt>{t("tooltip.harness")}</dt><dd>{stove.harness.label}</dd></div>
        <div><dt>{t("tooltip.host")}</dt><dd>{stove.host.kind === "ssh" ? t("tooltip.remote", { host: stove.host.id }) : t("tooltip.local", { host: stove.host.id })}</dd></div>
        <div><dt>{t("tooltip.project")}</dt><dd>{stove.projectLabel ?? t("common.notReported")}</dd></div>
        <div><dt>{t("tooltip.session")}</dt><dd>{stoveSessionIdentity(stove)}</dd></div>
        <div><dt>{t("tooltip.state")}</dt><dd>{stoveStateLabel(stove.state, t)}</dd></div>
        <div><dt>{t("tooltip.activity")}</dt><dd>{stove.currentAction ?? t("tooltip.waiting")}</dd></div>
        <div><dt>{t("tooltip.progress")}</dt><dd>{structured ? `${stove.progress?.completed ?? 0}/${stove.progress?.total} (${stove.progress?.provenance})` : t("tooltip.noProgress")}</dd></div>
        <div><dt>{t("tooltip.elapsed")}</dt><dd>{elapsed}</dd></div>
        <div><dt>{t("tooltip.next")}</dt><dd>{stove.nextAction ?? t("tooltip.noNext")}</dd></div>
      </dl>
    </aside>
  );
}
