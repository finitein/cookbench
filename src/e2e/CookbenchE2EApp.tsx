import { useEffect, useRef, useState } from "react";
import { DetachedStoveBar } from "../components/DetachedStoveBar";
import { GlobalBar } from "../components/GlobalBar";
import { LOCAL_ALERT_TEST_STOVE_ID } from "../services/localAlerts";
import type { StoveWire } from "../types/stove";
import "./e2e.css";

type Placement = { x: number; y: number };
type NotificationRecord = { destination: string; event: string };
type GlobalBarMode = "full" | "minimal";
type DockPhase = "undocked" | "dockedExpanded" | "dockedCollapsed";

export type CookbenchE2EPresentationSnapshot = {
  stoves: StoveWire[];
  attentionOrder: string[];
  globalBarMode: GlobalBarMode;
  dockPhase: DockPhase;
  dockBestEffort: boolean;
  macStatusAvailable: boolean;
  macStatusStoveCount: number;
};

export type CookbenchE2EDriver = {
  replaceStoves(stoves: StoveWire[]): Promise<void>;
  replaceSnapshot(snapshot: Partial<CookbenchE2EPresentationSnapshot> & Pick<CookbenchE2EPresentationSnapshot, "stoves">): Promise<void>;
  setGlobalBarMode(mode: GlobalBarMode): Promise<void>;
  setDockState(phase: DockPhase, bestEffort?: boolean): Promise<void>;
  setMacStatusFixture(available: boolean, stoveCount: number): Promise<void>;
  acknowledgeCooked(stoveId: string, postAcknowledgementOrder: string[]): Promise<void>;
  restart(): Promise<void>;
  detach(stoveId: string): Promise<void>;
  moveDetached(stoveId: string, x: number, y: number): Promise<void>;
  restoreDetached(): Promise<void>;
  clear(stoveId: string): Promise<void>;
  notifications(): Promise<readonly NotificationRecord[]>;
  flash(stoveId?: string): Promise<void>;
};

declare global {
  interface Window {
    __COOKBENCH_E2E__?: CookbenchE2EDriver;
  }
}

const PRESENTATION_KEY = "cookbench-e2e-presentation";
const LAYOUT_KEY = "cookbench-e2e-layout";

function readStored<T>(key: string, fallback: T): T {
  try {
    return JSON.parse(localStorage.getItem(key) ?? "") as T;
  } catch {
    return fallback;
  }
}

function normalizeAttentionOrder(order: readonly string[], stoves: readonly StoveWire[]) {
  const byId = new Map(stoves.map((stove) => [stove.id, stove]));
  const seen = new Set<string>();
  const normalized: StoveWire[] = [];
  for (const id of order) {
    const stove = byId.get(id);
    if (stove && !seen.has(id)) {
      seen.add(id);
      normalized.push(stove);
    }
  }
  for (const stove of stoves) {
    if (!seen.has(stove.id)) normalized.push(stove);
  }
  return normalized;
}

function defaultPresentation(): CookbenchE2EPresentationSnapshot {
  return {
    stoves: [],
    attentionOrder: [],
    globalBarMode: "full",
    dockPhase: "undocked",
    dockBestEffort: false,
    macStatusAvailable: false,
    macStatusStoveCount: 3,
  };
}

function readPresentation(): CookbenchE2EPresentationSnapshot {
  const stored = readStored<Partial<CookbenchE2EPresentationSnapshot>>(PRESENTATION_KEY, {});
  const next = { ...defaultPresentation(), ...stored };
  next.stoves = normalizeAttentionOrder(next.attentionOrder, next.stoves ?? []);
  next.attentionOrder = next.stoves.map((stove) => stove.id);
  next.macStatusStoveCount = Math.max(0, Math.min(8, Math.trunc(next.macStatusStoveCount)));
  return next;
}

export default function CookbenchE2EApp() {
  const [presentation, setPresentation] = useState<CookbenchE2EPresentationSnapshot>(readPresentation);
  const [placements, setPlacements] = useState<Record<string, Placement>>({});
  const [notifications, setNotifications] = useState<NotificationRecord[]>([]);
  const [activeAlertStoveId, setActiveAlertStoveId] = useState<string | null>(null);
  const stoves = presentation.stoves;
  const presentationRef = useRef(presentation);
  const placementsRef = useRef(placements);
  const notificationsRef = useRef(notifications);

  useEffect(() => { presentationRef.current = presentation; }, [presentation]);
  useEffect(() => { placementsRef.current = placements; }, [placements]);
  useEffect(() => { notificationsRef.current = notifications; }, [notifications]);

  const updatePresentation = (patch: Partial<CookbenchE2EPresentationSnapshot>) => {
    setPresentation((current) => {
      const next = { ...current, ...patch };
      next.stoves = normalizeAttentionOrder(next.attentionOrder, next.stoves);
      next.attentionOrder = next.stoves.map((stove) => stove.id);
      localStorage.setItem(PRESENTATION_KEY, JSON.stringify(next));
      return next;
    });
  };

  useEffect(() => {
    const persistPlacements = (next: Record<string, Placement>) => {
      localStorage.setItem(LAYOUT_KEY, JSON.stringify(next));
      setPlacements(next);
    };
    window.__COOKBENCH_E2E__ = {
      async replaceStoves(next) {
        updatePresentation({ stoves: next, attentionOrder: next.map((stove) => stove.id) });
        setNotifications(next.map((stove) => ({
          destination: "e2e-enabled",
          event: stove.state,
        })));
      },
      async replaceSnapshot(next) {
        updatePresentation(next);
      },
      async setGlobalBarMode(globalBarMode) {
        updatePresentation({ globalBarMode });
      },
      async setDockState(dockPhase, dockBestEffort = false) {
        updatePresentation({ dockPhase, dockBestEffort });
      },
      async setMacStatusFixture(macStatusAvailable, macStatusStoveCount) {
        updatePresentation({ macStatusAvailable, macStatusStoveCount });
      },
      async acknowledgeCooked(stoveId, postAcknowledgementOrder) {
        const current = presentationRef.current;
        updatePresentation({
          attentionOrder: postAcknowledgementOrder,
          stoves: current.stoves.map((stove) => stove.id === stoveId && stove.state === "cooked"
            ? { ...stove, retainedCompletion: true }
            : stove),
        });
      },
      async restart() {
        setPresentation(readPresentation());
        setPlacements(readStored<Record<string, Placement>>(LAYOUT_KEY, {}));
      },
      async detach(stoveId) {
        persistPlacements({ ...placementsRef.current, [stoveId]: { x: 24, y: 24 } });
      },
      async moveDetached(stoveId, x, y) {
        persistPlacements({ ...placementsRef.current, [stoveId]: { x, y } });
      },
      async restoreDetached() {
        setPlacements(readStored<Record<string, Placement>>(LAYOUT_KEY, {}));
      },
      async clear(stoveId) {
        const current = presentationRef.current;
        const remaining = current.stoves.filter((stove) => stove.id !== stoveId);
        const nextPlacements = { ...placementsRef.current };
        delete nextPlacements[stoveId];
        updatePresentation({ stoves: remaining, attentionOrder: current.attentionOrder.filter((id) => id !== stoveId) });
        persistPlacements(nextPlacements);
      },
      async notifications() {
        return notificationsRef.current;
      },
      async flash(stoveId = LOCAL_ALERT_TEST_STOVE_ID) {
        setActiveAlertStoveId(stoveId);
      },
    };
    return () => {
      delete window.__COOKBENCH_E2E__;
    };
  // Browser tests deliberately use a stable driver. State lives in refs so a
  // React render cannot exchange a command between an action and its effect.
  // This is fixture-only and never reaches the production entry point.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const revealDock = () => {
    if (presentation.dockPhase === "dockedCollapsed") updatePresentation({ dockPhase: "dockedExpanded" });
  };
  const scheduleCollapse = () => {
    if (presentation.dockPhase !== "dockedExpanded" || presentation.dockBestEffort) return;
    window.setTimeout(() => {
      setPresentation((current) => {
        if (current.dockPhase !== "dockedExpanded" || current.dockBestEffort) return current;
        const next = { ...current, dockPhase: "dockedCollapsed" as const };
        localStorage.setItem(PRESENTATION_KEY, JSON.stringify(next));
        return next;
      });
    }, 600);
  };

  return (
    <main
      className={`shell cookbench-e2e-presentation cookbench-e2e-presentation--${presentation.dockPhase}`}
      aria-label="Cookbench E2E presentation"
      data-dock-phase={presentation.dockPhase}
      data-dock-best-effort={presentation.dockBestEffort ? "true" : "false"}
      onPointerLeave={presentation.dockPhase === "dockedExpanded" ? scheduleCollapse : undefined}
    >
      {presentation.dockPhase === "dockedCollapsed" ? <div
        className="cookbench-e2e-dock-trigger"
        data-testid="e2e-dock-trigger"
        aria-label="Reveal docked Cookbench bar"
        onPointerEnter={revealDock}
      /> : null}
      <GlobalBar
        stoves={stoves}
        onActivateStove={() => {
          setActiveAlertStoveId(null);
        }}
        onDetachStove={() => undefined}
        onClearStove={() => undefined}
        onOpenSettings={() => undefined}
        hoverDetailsEnabled
        activeAlertStoveId={activeAlertStoveId}
        mode={presentation.globalBarMode}
        onModeChange={(globalBarMode) => updatePresentation({ globalBarMode })}
      />
      <button className="cookbench-e2e-undock" type="button" onClick={() => updatePresentation({ dockPhase: "undocked", dockBestEffort: false })}>
        Undock fixture
      </button>
      <output
        className="cookbench-e2e-mac-status-fixture"
        data-testid="e2e-mac-status-fixture"
        aria-hidden="true"
        data-available={presentation.macStatusAvailable ? "true" : "false"}
        data-count={presentation.macStatusStoveCount}
        data-stove-ids={presentation.macStatusAvailable
          ? stoves.slice(0, presentation.macStatusStoveCount).map((stove) => stove.id).join(",")
          : ""}
      />
      {Object.entries(placements).map(([stoveId, placement]) => {
        const stove = stoves.find((candidate) => candidate.id === stoveId);
        if (!stove) return null;
        return (
          <div
            data-testid={`detached-window-${stoveId}`}
            key={stoveId}
            style={{ position: "fixed", zIndex: 20, left: placement.x, top: placement.y, width: 164, height: 104 }}
          >
            <DetachedStoveBar stove={stove} />
          </div>
        );
      })}
    </main>
  );
}
