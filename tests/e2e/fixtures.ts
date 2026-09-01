import type { Page } from "@playwright/test";
import type { CookbenchE2EDriver, CookbenchE2EPresentationSnapshot } from "../../src/e2e/CookbenchE2EApp";
import type { StoveState, StoveWire } from "../../src/types/stove";

export const E2E_HARNESSES = [
  { id: "codex", label: "Codex" },
  { id: "claudeCode", label: "Claude Code" },
  { id: "pi", label: "Pi" },
] as const;

export function stoveFixture(
  index: number,
  state: StoveState = "cooking",
  overrides: Partial<StoveWire> = {},
): StoveWire {
  const harness = E2E_HARNESSES[index % E2E_HARNESSES.length];
  const isCooking = state === "cooking";
  return {
    id: `fixture-${harness.id}-${index}`,
    harness,
    host: { kind: "local", id: "test-host" },
    projectRoot: `/fixture/project-${index}`,
    projectLabel: `Project ${index}`,
    projectRootDisplay: `~/fixture/project-${index}`,
    taskTitle: `Synthetic task ${index}`,
    currentAction: isCooking ? "Running fixture action" : "Fixture state transition",
    nextAction: "Return to original source",
    elapsedMs: 60_000,
    state,
    progress: isCooking ? { completed: 2, total: 5, provenance: "structuredSession" } : null,
    locatorCapability: "available",
    retainedCompletion: state === "cooked",
    ...overrides,
  };
}

export const allStateFixtures = (): StoveWire[] => [
  stoveFixture(0, "starting"),
  stoveFixture(1, "planning"),
  stoveFixture(2, "cooking"),
  stoveFixture(3, "needsHuman"),
  stoveFixture(4, "cooked"),
  stoveFixture(5, "failed"),
  stoveFixture(6, "disconnected", { host: { kind: "ssh", id: "fixture-ssh" } }),
];

export async function e2eDriver(page: Page) {
  await page.waitForFunction(() => Boolean(window.__COOKBENCH_E2E__), null, { timeout: 5_000 });
  return {
    replaceStoves: (stoves: StoveWire[]) =>
      page.evaluate((fixtures) => window.__COOKBENCH_E2E__!.replaceStoves(fixtures), stoves),
    replaceSnapshot: (snapshot: Partial<CookbenchE2EPresentationSnapshot> & Pick<CookbenchE2EPresentationSnapshot, "stoves">) =>
      page.evaluate((value) => window.__COOKBENCH_E2E__!.replaceSnapshot(value), snapshot),
    setGlobalBarMode: (mode: "full" | "minimal") =>
      page.evaluate((value) => window.__COOKBENCH_E2E__!.setGlobalBarMode(value), mode),
    setDockState: (phase: "undocked" | "dockedExpanded" | "dockedCollapsed", bestEffort?: boolean) =>
      page.evaluate(([value, fallback]) => window.__COOKBENCH_E2E__!.setDockState(value, fallback), [phase, bestEffort] as const),
    setMacStatusFixture: (available: boolean, stoveCount: number) =>
      page.evaluate(([value, count]) => window.__COOKBENCH_E2E__!.setMacStatusFixture(value, count), [available, stoveCount] as const),
    acknowledgeCooked: (stoveId: string, postAcknowledgementOrder: string[]) =>
      page.evaluate(([id, order]) => window.__COOKBENCH_E2E__!.acknowledgeCooked(id, order), [stoveId, postAcknowledgementOrder] as const),
    restart: () => page.evaluate(() => window.__COOKBENCH_E2E__!.restart()),
    detach: (stoveId: string) =>
      page.evaluate((id) => window.__COOKBENCH_E2E__!.detach(id), stoveId),
    moveDetached: (stoveId: string, x: number, y: number) =>
      page.evaluate(
        ([id, left, top]) => window.__COOKBENCH_E2E__!.moveDetached(id, left, top),
        [stoveId, x, y] as const,
      ),
    restoreDetached: () => page.evaluate(() => window.__COOKBENCH_E2E__!.restoreDetached()),
    clear: (stoveId: string) => page.evaluate((id) => window.__COOKBENCH_E2E__!.clear(id), stoveId),
    notifications: () => page.evaluate(() => window.__COOKBENCH_E2E__!.notifications()),
    flash: (stoveId?: string) => page.evaluate((id) => window.__COOKBENCH_E2E__!.flash(id), stoveId),
  };
}
