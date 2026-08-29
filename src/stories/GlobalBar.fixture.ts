import type { StoveState, StoveWire } from "../types/stove";

const harnesses = [
  { id: "codex", label: "Codex" },
  { id: "claudeCode", label: "Claude Code" },
  { id: "pi", label: "Pi" },
] as const;

const states: StoveState[] = ["cooking", "needsHuman", "cooked", "failed", "disconnected", "planning"];

export function makeStove(index: number, overrides: Partial<StoveWire> = {}): StoveWire {
  const harness = harnesses[index % harnesses.length];
  const state = states[index % states.length];

  const stove = {
    id: `fixture:${harness.id}:${index}`,
    harness,
    host: index % 4 === 0 ? { kind: "ssh", id: "build-host" } : { kind: "local", id: "this-device" },
    projectRootDisplay: `/workspace/project-${index + 1}`,
    projectLabel: `Project ${index + 1}`,
    taskTitle: `Task ${index + 1}`,
    currentAction: state === "cooking" ? "Running a safe check" : "Waiting for the source update",
    nextAction: "Return to the original session",
    elapsedMs: 75_000 + index * 1_000,
    state,
    progress: state === "cooking" ? { completed: 2, total: 5, provenance: "structuredSession" } : null,
    locatorCapability: "available",
    retainedCompletion: state === "cooked",
    ...overrides,
  };

  return stove as StoveWire;
}

export function globalBarFixture(count: number): StoveWire[] {
  return Array.from({ length: count }, (_, index) => makeStove(index));
}
