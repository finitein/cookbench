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

/**
 * A compact, UI-only discriminator derived from a Stove key. The full key is
 * deliberately never rendered: session keys can be long and do not belong in
 * an always-visible desktop surface.
 */
export function stoveSessionIdentity(stove: Pick<StoveWire, "id">): string {
  const nativeSessionId = stove.id.split(":").at(-1) ?? "";
  if (/^[A-Za-z0-9_-]{4,}$/.test(nativeSessionId)) {
    return `#${nativeSessionId.slice(-8)}`;
  }

  // Keep malformed or future adapter ids recognizable without echoing them.
  let hash = 2_166_136_261;
  for (const character of stove.id) {
    hash ^= character.charCodeAt(0);
    hash = Math.imul(hash, 16_777_619);
  }
  return `#${(hash >>> 0).toString(36).padStart(6, "0").slice(-6)}`;
}

export function stoveDisplayIdentity(stove: Pick<StoveWire, "id" | "projectLabel">): string {
  return `${stove.projectLabel?.trim() || "Session"} ${stoveSessionIdentity(stove)}`;
}

export function hasStructuredProgress(stove: StoveWire): stove is StoveWire & { progress: ProgressWire } {
  return stove.progress !== null && stove.progress.total > 0;
}
