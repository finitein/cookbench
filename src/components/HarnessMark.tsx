import type { StoveWire } from "../types/stove";

type Harness = "codex" | "claude" | "claudeCode" | "pi" | "grok_cli" | "goose" | "amp";

const harnessDetails: Record<Harness, { label: string; token: string }> = {
  codex: { label: "Codex", token: "CX" },
  claude: { label: "Claude Code", token: "CL" },
  claudeCode: { label: "Claude Code", token: "CL" },
  pi: { label: "Pi", token: "PI" },
  grok_cli: { label: "Grok Build", token: "GB" },
  goose: { label: "Goose", token: "GO" },
  amp: { label: "Amp", token: "AM" },
};

export function harnessInfo(harness: string) {
  return harnessDetails[harness as Harness] ?? { label: harness, token: harness.slice(0, 2).toUpperCase() };
}

export function HarnessMark({ harness }: { harness: StoveWire["harness"] }) {
  const { label, token } = harnessInfo(harness.id);

  return (
    <span className={`harness-mark harness-mark--${harness.id}`} aria-label={harness.label || label} title={harness.label || label}>
      {token}
    </span>
  );
}
