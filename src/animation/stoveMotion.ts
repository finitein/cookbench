import type { StoveState } from "../types/stove";

export type StoveMotionState = StoveState;

export type CompletionPhase = "none" | "finishing" | "settled";

export interface StoveMotionPreferences {
  reducedMotion: boolean;
  soundEnabled: boolean;
}

export interface StoveMotionInput {
  previousState?: StoveMotionState;
  nextState: StoveMotionState;
  /** Snapshots establish state; they must not replay historic completion effects. */
  isInitialSnapshot?: boolean;
}

export interface StoveMotionResult {
  completion: CompletionPhase;
  notify: boolean;
  playSound: boolean;
  ring: "progress" | "complete";
  rotates: boolean;
}

const STATIC_COMPLETE_STATES = new Set<StoveMotionState>([
  "needsHuman",
  "cooked",
  "failed",
  "disconnected",
]);

/**
 * Decides presentation effects from a state transition rather than a snapshot.
 * It intentionally has no timers or DOM writes so rendering stays stable.
 */
export function motionForStoveTransition(
  input: StoveMotionInput,
  preferences: StoveMotionPreferences,
): StoveMotionResult {
  const newlyCooked =
    !input.isInitialSnapshot &&
    input.previousState !== undefined &&
    input.previousState !== "cooked" &&
    input.nextState === "cooked";

  if (newlyCooked) {
    return {
      completion: preferences.reducedMotion ? "settled" : "finishing",
      notify: true,
      playSound: preferences.soundEnabled,
      ring: "complete",
      rotates: false,
    };
  }

  return {
    completion: input.nextState === "cooked" ? "settled" : "none",
    notify: false,
    playSound: false,
    ring: STATIC_COMPLETE_STATES.has(input.nextState) ? "complete" : "progress",
    rotates: input.nextState === "cooking" && !preferences.reducedMotion,
  };
}

/** Returns the post-animation state without changing ring geometry or layout. */
export function settleCompletionEffect(result: StoveMotionResult): StoveMotionResult {
  if (result.completion !== "finishing") {
    return result;
  }

  return {
    ...result,
    completion: "settled",
    notify: false,
    playSound: false,
    ring: "complete",
    rotates: false,
  };
}
