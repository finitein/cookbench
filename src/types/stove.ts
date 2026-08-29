/** Sanitized state received from the desktop process. Never add session text. */
export type HarnessKind = "codex" | "claudeCode" | "pi" | (string & {});

export type StoveState =
  | "starting"
  | "planning"
  | "cooking"
  | "needsHuman"
  | "cooked"
  | "failed"
  | "disconnected";

export type ProgressProvenance = "structuredSession" | "hook";

export interface ProgressWire {
  completed: number;
  total: number;
  provenance: ProgressProvenance;
}

export interface HarnessWire {
  id: HarnessKind;
  label: string;
}

export interface HostWire {
  kind: "local" | "ssh";
  id: string;
}

export interface StoveWire {
  id: string;
  harness: HarnessWire;
  host: HostWire;
  projectRoot: string;
  projectLabel?: string;
  projectRootDisplay?: string;
  taskTitle?: string | null;
  currentAction?: string | null;
  nextAction?: string | null;
  elapsedMs?: number | null;
  state: StoveState;
  progress: ProgressWire | null;
  locatorCapability: "available" | "unavailable";
  retainedCompletion: boolean;
}

export interface StoveSnapshot {
  revision: number;
  stoves: StoveWire[];
}

export interface StoveChange {
  revision: number;
  stove: StoveWire | null;
  removedStoveId: string | null;
}

export function hasStructuredProgress(stove: StoveWire): stove is StoveWire & { progress: ProgressWire } {
  return stove.progress !== null && stove.progress.total > 0;
}
