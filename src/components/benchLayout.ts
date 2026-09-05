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

const HARNESS_ORDER = ["codex", "claudeCode", "pi", "grok_cli", "goose", "amp"] as const;

/** The display must always reserve at least one complete Stove slot. */
export function stoveCapacityForWidth(width: number): number {
  return Math.max(1, Math.floor(width / STOVE_SLOT_WIDTH));
}

export function sortStovesForBench(stoves: readonly StoveWire[]): StoveWire[] {
  // The desktop process owns attention ordering. Keep this small UI helper as
  // an identity copy so grouped benches cannot silently compete with it.
  return [...stoves];
}

function harnessRank(harness: HarnessKind): number {
  const index = (HARNESS_ORDER as readonly string[]).indexOf(harness);
  return index >= 0 ? index : HARNESS_ORDER.length;
}

/** Preserve desktop attention order: first harness in `stoves` wins the top bench. */
function orderedHarnesses(stoves: readonly StoveWire[]): HarnessKind[] {
  const firstIndex = new Map<HarnessKind, number>();
  stoves.forEach((stove, index) => {
    if (!firstIndex.has(stove.harness.id)) {
      firstIndex.set(stove.harness.id, index);
    }
  });
  return [...firstIndex.keys()].sort((left, right) => {
    const byAttention = (firstIndex.get(left) ?? 0) - (firstIndex.get(right) ?? 0);
    if (byAttention !== 0) return byAttention;
    const byKnown = harnessRank(left) - harnessRank(right);
    if (byKnown !== 0) return byKnown;
    return left.localeCompare(right);
  });
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
