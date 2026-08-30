import type { HarnessKind, StoveState, StoveWire } from "../types/stove";

export const STOVE_SLOT_WIDTH = 86;

export type StoveBench = {
  id: HarnessKind | "all";
  label: string;
  stoves: readonly StoveWire[];
};

export type BenchLayout = {
  grouped: boolean;
  benches: readonly StoveBench[];
};

const HARNESS_ORDER = ["codex", "claudeCode", "pi"] as const;

/** The display must always reserve at least one complete Stove slot. */
export function stoveCapacityForWidth(width: number): number {
  return Math.max(1, Math.floor(width / STOVE_SLOT_WIDTH));
}

function stateRank(state: StoveState): number {
  switch (state) {
    case "needsHuman":
      return 0;
    case "starting":
    case "planning":
    case "cooking":
      return 1;
    case "cooked":
    case "failed":
    case "disconnected":
      return 2;
  }
}

/**
 * A stable state sort deliberately retains the runtime's source order within a
 * state. The wire format does not carry a separate last-activity timestamp.
 */
export function sortStovesForBench(stoves: readonly StoveWire[]): StoveWire[] {
  return stoves
    .map((stove, index) => ({ stove, index }))
    .sort((left, right) => stateRank(left.stove.state) - stateRank(right.stove.state) || left.index - right.index)
    .map(({ stove }) => stove);
}

function orderedHarnesses(stoves: readonly StoveWire[]): HarnessKind[] {
  const seen = new Set(stoves.map((stove) => stove.harness.id));
  const known = HARNESS_ORDER.filter((harness) => seen.has(harness));
  const unknown = [...seen].filter((harness) => !HARNESS_ORDER.includes(harness as typeof HARNESS_ORDER[number])).sort();
  return [...known, ...unknown];
}

export function arrangeBenches(stoves: readonly StoveWire[], rowCapacity: number): BenchLayout {
  const capacity = Math.max(1, rowCapacity);
  const byHarness = new Map<HarnessKind, StoveWire[]>();
  for (const stove of stoves) {
    const group = byHarness.get(stove.harness.id) ?? [];
    group.push(stove);
    byHarness.set(stove.harness.id, group);
  }

  const grouped = [...byHarness.values()].some((group) => group.length > capacity);
  if (!grouped) {
    return {
      grouped: false,
      benches: [{ id: "all", label: "All sessions", stoves: sortStovesForBench(stoves) }],
    };
  }

  return {
    grouped: true,
    benches: orderedHarnesses(stoves).map((harness) => {
      const harnessStoves = byHarness.get(harness) ?? [];
      return {
        id: harness,
        label: harnessStoves[0]?.harness.label ?? harness,
        stoves: sortStovesForBench(harnessStoves),
      };
    }),
  };
}
