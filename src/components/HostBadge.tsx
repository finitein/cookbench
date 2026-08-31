import type { StoveWire } from "../types/stove";
import { useI18n } from "../i18n/i18n";

export function HostBadge({ stove }: { stove: StoveWire }) {
  const { t } = useI18n();
  const remote = stove.host.kind === "ssh";
  const host = stove.host.id;
  const label = remote ? t("host.remoteLabel", { host }) : t("host.localLabel", { host });

  return (
    <span className={`host-badge host-badge--${remote ? "remote" : "local"}`} aria-label={label} title={label}>
      {remote ? t("host.remote") : t("host.local")}
    </span>
  );
}
