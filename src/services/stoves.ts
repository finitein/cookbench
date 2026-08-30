import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { StoveChange, StoveSnapshot, StoveWire } from "../types/stove";

export const STOVE_CHANGED_EVENT = "cookbench://stove-changed";

export interface StoveTransport {
  snapshot(): Promise<StoveSnapshot>;
  listen(handler: (change: StoveChange) => void): Promise<UnlistenFn>;
}

export const tauriStoveTransport: StoveTransport = {
  snapshot: () => invoke<StoveSnapshot>("get_stoves_snapshot"),
  listen: (handler) => listen<StoveChange>(STOVE_CHANGED_EVENT, (event) => handler(event.payload)),
};

export function clearCookedStove(stoveId: string): Promise<void> {
  return invoke<void>("clear_cooked_stove", { stoveId });
}

/** Maintains a local revisioned view. A revision gap never gets guessed. */
export class StoveSync {
  private revision = 0;
  private stoves = new Map<string, StoveWire>();

  replace(snapshot: StoveSnapshot): StoveSnapshot {
    this.revision = snapshot.revision;
    this.stoves = new Map(snapshot.stoves.map((stove) => [stove.id, stove]));
    return this.current();
  }

  apply(change: StoveChange): "applied" | "stale" | "gap" {
    if (change.revision <= this.revision) return "stale";
    if (change.revision !== this.revision + 1) return "gap";
    this.revision = change.revision;
    if (change.stove) this.stoves.set(change.stove.id, change.stove);
    if (change.removedStoveId) this.stoves.delete(change.removedStoveId);
    return "applied";
  }

  current(): StoveSnapshot {
    return { revision: this.revision, stoves: [...this.stoves.values()] };
  }
}

export async function subscribeToStoves(
  onSnapshot: (snapshot: StoveSnapshot) => void,
  transport: StoveTransport = tauriStoveTransport,
): Promise<UnlistenFn> {
  const sync = new StoveSync();
  let ready = false;
  const queued: StoveChange[] = [];
  const handle = (change: StoveChange) => {
    if (!ready) {
      queued.push(change);
      return;
    }
    const result = sync.apply(change);
    if (result === "applied") {
      onSnapshot(sync.current());
    } else if (result === "gap") {
      void transport.snapshot().then((snapshot) => onSnapshot(sync.replace(snapshot)));
    }
  };
  const unlisten = await transport.listen(handle).catch((): UnlistenFn => () => {});
  onSnapshot(sync.replace(await transport.snapshot()));
  ready = true;
  for (const change of queued.sort((left, right) => left.revision - right.revision)) handle(change);
  return unlisten;
}
