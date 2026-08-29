/**
 * Optional Pi extension bridge. It reports only bounded lifecycle state to a
 * local Cookbench hook; it never registers tools, consumes input, or changes
 * Pi's model context.
 */
export type PiLifecycleEvent =
  | { type: "prompt_submitted" }
  | { type: "tool_started" }
  | { type: "tool_completed"; succeeded: boolean }
  | { type: "question_asked" }
  | { type: "permission_requested" }
  | { type: "turn_completed" }
  | { type: "session_failed" }
  | { type: "todo_progress"; completed: number; total: number };

export interface PiLifecycleEnvelope {
  version: 1;
  sessionId: string;
  event: PiLifecycleEvent;
}

export type LifecycleSink = (envelope: PiLifecycleEnvelope) => void | Promise<void>;

const MAX_SESSION_ID_BYTES = 512;

export function createPiLifecycleEmitter(sessionId: string, sink: LifecycleSink) {
  const sessionIdBytes = new TextEncoder().encode(sessionId).byteLength;
  const validSessionId = sessionIdBytes > 0 && sessionIdBytes <= MAX_SESSION_ID_BYTES;

  return async (event: PiLifecycleEvent): Promise<void> => {
    if (!validSessionId || !isValidEvent(event)) {
      return;
    }

    try {
      await sink({ version: 1, sessionId, event });
    } catch {
      // Extension delivery is best effort and must never break Pi's lifecycle.
    }
  };
}

function isValidEvent(event: PiLifecycleEvent): boolean {
  if (event.type !== "todo_progress") {
    return true;
  }
  return Number.isInteger(event.completed) && Number.isInteger(event.total) && event.total > 0 && event.completed >= 0 && event.completed <= event.total;
}
