import type { HarnessKind, StoveWire } from "../types/stove";

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

export function sortStovesForBench(stoves: readonly StoveWire[]): StoveWire[] {
  // The desktop process owns attention ordering. Keep this small UI helper as
  // an identity copy so grouped benches cannot silently compete with it.
  return [...stoves];
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
