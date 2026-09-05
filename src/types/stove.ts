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
  pinned: boolean;
}

/** Safe archive metadata only; native session content is never sent to the UI. */
export interface ArchivedSessionWire {
  id: string;
  harness: HarnessWire;
  host: HostWire;
  projectLabel?: string;
  projectRootDisplay?: string;
  sessionIdentity: string;
  lastState: StoveState;
  reason: "expired" | "manual";
  archivedAtMs: number;
  sourceAvailable: boolean;
  pinned: boolean;
}

export interface StoveSnapshot {
  revision: number;
  stoves: StoveWire[];
  /** Canonical attention order; optional only for pre-attention browser fixtures. */
  attentionOrder?: string[];
}

export interface StoveChange {
  revision: number;
  stove: StoveWire | null;
  removedStoveId: string | null;
  attentionOrder?: string[];
}

/**
 * A compact, UI-only discriminator derived from a Stove key. The full key is
 * deliberately never rendered: session keys can be long and do not belong in
 * an always-visible desktop surface.
 */
export function stoveSessionIdentity(stove: Pick<StoveWire, "id">): string {
  const nativeSessionId = stove.id.split(":").at(-1) ?? "";
  if (/^[A-Za-z0-9_-]{4,}$/.test(nativeSessionId)) {
    return `#${compactSessionIdentity(nativeSessionId)}`;
  }

  // Keep malformed or future adapter ids recognizable without echoing them.
  let hash = 2_166_136_261;
  for (const character of stove.id) {
    hash ^= character.charCodeAt(0);
    hash = Math.imul(hash, 16_777_619);
  }
  return `#${(hash >>> 0).toString(36).padStart(6, "0").slice(-6)}`;
}

/**
 * Prefer a readable prefix for short or UUID-like session ids. Suffix-only
 * truncation turned `session-001` into `#sion-001` and UUID tails into
 * `#eeeeffff`, which are hard to recognize on the Bar. Long multi-segment ids
 * without a 4–8 trailing token (e.g. `claude-session-abc`) must not collapse
 * into mid-word chops like `#claude-s`.
 */
export function compactSessionIdentity(nativeSessionId: string): string {
  if (nativeSessionId.length <= 8) {
    return nativeSessionId;
  }

  const uuidLike =
    /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;
  if (uuidLike.test(nativeSessionId)) {
    return nativeSessionId.replace(/-/g, "").slice(0, 8);
  }

  const parts = nativeSessionId.split(/([-_])/).filter((part) => part.length > 0);
  const segments = parts.filter((part) => part !== "-" && part !== "_");
  const trailing = segments.at(-1) ?? "";
  if (/^[A-Za-z0-9]{4,8}$/.test(trailing) && trailing.length < nativeSessionId.length) {
    return trailing;
  }

  // Join the last two segments when that yields a compact 4–8 discriminator.
  if (segments.length >= 2) {
    const lastSepIndex = (() => {
      for (let index = parts.length - 2; index >= 0; index -= 1) {
        if (parts[index] === "-" || parts[index] === "_") {
          return index;
        }
      }
      return -1;
    })();
    if (lastSepIndex > 0) {
      const joined = `${parts[lastSepIndex - 1]}${parts[lastSepIndex]}${parts[lastSepIndex + 1]}`;
      if (
        joined.length >= 4 &&
        joined.length <= 8 &&
        /^[A-Za-z0-9]+([-_][A-Za-z0-9]+)?$/.test(joined)
      ) {
        return joined;
      }
    }
  }

  // Prefix fallback: end on a separator boundary instead of mid-segment.
  const prefix = nativeSessionId.slice(0, 8);
  const nextChar = nativeSessionId[8];
  const cutMidSegment =
    nextChar !== undefined && nextChar !== "-" && nextChar !== "_";
  if (cutMidSegment) {
    const boundary = Math.max(prefix.lastIndexOf("-"), prefix.lastIndexOf("_"));
    if (boundary >= 3) {
      return prefix.slice(0, boundary + 1);
    }
    const first = segments[0] ?? "";
    if (first.length >= 4 && first.length <= 8) {
      return first;
    }
  }

  return prefix;
}

export function stoveDisplayIdentity(
  stove: Pick<StoveWire, "id" | "projectLabel">,
  sessionFallback = "Session",
): string {
  return `${stove.projectLabel?.trim() || sessionFallback} ${stoveSessionIdentity(stove)}`;
}

export function hasStructuredProgress(stove: StoveWire): stove is StoveWire & { progress: ProgressWire } {
  return stove.progress !== null && stove.progress.total > 0;
}
