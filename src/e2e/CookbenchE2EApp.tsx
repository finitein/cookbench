import { useEffect, useState } from "react";
import { DetachedStoveBar } from "../components/DetachedStoveBar";
import { GlobalBar } from "../components/GlobalBar";
import type { StoveWire } from "../types/stove";

type Placement = { x: number; y: number };
type NotificationRecord = { destination: string; event: string };

export type CookbenchE2EDriver = {
  replaceStoves(stoves: StoveWire[]): Promise<void>;
  restart(): Promise<void>;
  detach(stoveId: string): Promise<void>;
  moveDetached(stoveId: string, x: number, y: number): Promise<void>;
  restoreDetached(): Promise<void>;
  clear(stoveId: string): Promise<void>;
  notifications(): Promise<readonly NotificationRecord[]>;
};

declare global {
  interface Window {
    __COOKBENCH_E2E__?: CookbenchE2EDriver;
  }
}

const STOVES_KEY = "cookbench-e2e-stoves";
const LAYOUT_KEY = "cookbench-e2e-layout";

function readStored<T>(key: string, fallback: T): T {
  try {
    return JSON.parse(localStorage.getItem(key) ?? "") as T;
  } catch {
    return fallback;
  }
}

export default function CookbenchE2EApp() {
  const [stoves, setStoves] = useState<StoveWire[]>([]);
  const [placements, setPlacements] = useState<Record<string, Placement>>({});
  const [notifications, setNotifications] = useState<NotificationRecord[]>([]);

  useEffect(() => {
    const persistPlacements = (next: Record<string, Placement>) => {
      localStorage.setItem(LAYOUT_KEY, JSON.stringify(next));
      setPlacements(next);
    };
    window.__COOKBENCH_E2E__ = {
      async replaceStoves(next) {
        localStorage.setItem(STOVES_KEY, JSON.stringify(next));
        setStoves(next);
        setNotifications(next.map((stove) => ({
          destination: "e2e-enabled",
          event: stove.state,
        })));
      },
      async restart() {
        setStoves(readStored<StoveWire[]>(STOVES_KEY, []));
        setPlacements(readStored<Record<string, Placement>>(LAYOUT_KEY, {}));
      },
      async detach(stoveId) {
        persistPlacements({ ...placements, [stoveId]: { x: 24, y: 24 } });
      },
      async moveDetached(stoveId, x, y) {
        persistPlacements({ ...placements, [stoveId]: { x, y } });
      },
      async restoreDetached() {
        setPlacements(readStored<Record<string, Placement>>(LAYOUT_KEY, {}));
      },
      async clear(stoveId) {
        const remaining = stoves.filter((stove) => stove.id !== stoveId);
        const nextPlacements = { ...placements };
        delete nextPlacements[stoveId];
        localStorage.setItem(STOVES_KEY, JSON.stringify(remaining));
        persistPlacements(nextPlacements);
        setStoves(remaining);
      },
      async notifications() {
        return notifications;
      },
    };
    return () => {
      delete window.__COOKBENCH_E2E__;
    };
  }, [notifications, placements, stoves]);

  return (
    <main className="shell" aria-label="Cookbench E2E presentation">
      <GlobalBar
        stoves={stoves}
        onActivateStove={() => undefined}
        onDetachStove={() => undefined}
        onClearStove={() => undefined}
        onOpenSettings={() => undefined}
      />
      {Object.entries(placements).map(([stoveId, placement]) => {
        const stove = stoves.find((candidate) => candidate.id === stoveId);
        if (!stove) return null;
        return (
          <div
            data-testid={`detached-window-${stoveId}`}
            key={stoveId}
            style={{ position: "fixed", left: placement.x, top: placement.y }}
          >
            <DetachedStoveBar stove={stove} />
          </div>
        );
      })}
    </main>
  );
}
