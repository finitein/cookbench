import type { StoveWire } from "../types/stove";

export function HostBadge({ stove }: { stove: StoveWire }) {
  const remote = stove.host.kind === "ssh";
  const host = stove.host.id;
  const label = remote ? `Remote host: ${host}` : `Local host: ${host}`;

  return (
    <span className={`host-badge host-badge--${remote ? "remote" : "local"}`} aria-label={label} title={label}>
      {remote ? "Remote" : "Local"}
    </span>
  );
}
