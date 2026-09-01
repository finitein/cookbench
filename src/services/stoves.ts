import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ArchivedSessionWire, StoveChange, StoveSnapshot, StoveWire } from "../types/stove";

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

export function acknowledgeCookedStove(stoveId: string): Promise<void> {
  return invoke<void>("acknowledge_cooked_stove", { stoveId });
}

export function setStovePinned(stoveId: string, pinned: boolean): Promise<void> {
  return invoke<void>("set_stove_pinned", { stoveId, pinned });
}

export function archiveStove(stoveId: string): Promise<void> {
  return invoke<void>("archive_stove", { stoveId });
}

export function getArchivedSessions(): Promise<ArchivedSessionWire[]> {
  return invoke<ArchivedSessionWire[]>("get_archived_sessions");
}

export function restoreArchivedSession(stoveId: string): Promise<void> {
  return invoke<void>("restore_archived_session", { stoveId });
}

/** Maintains a local revisioned view. A revision gap never gets guessed. */
export class StoveSync {
  private revision = 0;
  private stoves = new Map<string, StoveWire>();
  private attentionOrder: string[] = [];

  replace(snapshot: StoveSnapshot): StoveSnapshot {
    this.revision = snapshot.revision;
    this.stoves = new Map(snapshot.stoves.map((stove) => [stove.id, stove]));
    this.attentionOrder = normalizeAttentionOrder(snapshot.attentionOrder, this.stoves);
    return this.current();
  }

  apply(change: StoveChange): "applied" | "stale" | "gap" {
    if (change.revision <= this.revision) return "stale";
    if (change.revision !== this.revision + 1) return "gap";
    this.revision = change.revision;
    if (change.stove) this.stoves.set(change.stove.id, change.stove);
    if (change.removedStoveId) this.stoves.delete(change.removedStoveId);
    this.attentionOrder = normalizeAttentionOrder(change.attentionOrder, this.stoves);
    return "applied";
  }

  current(): StoveSnapshot {
    const attentionOrder = normalizeAttentionOrder(this.attentionOrder, this.stoves);
    return {
      revision: this.revision,
      attentionOrder,
      stoves: attentionOrder.map((id) => this.stoves.get(id)!).filter(Boolean),
    };
  }
}

function normalizeAttentionOrder(order: string[] | undefined, stoves: Map<string, StoveWire>): string[] {
  const normalized: string[] = [];
  const seen = new Set<string>();
  for (const id of order ?? []) {
    if (stoves.has(id) && !seen.has(id)) {
      seen.add(id);
      normalized.push(id);
    }
  }
  for (const id of stoves.keys()) {
    if (!seen.has(id)) normalized.push(id);
  }
  return normalized;
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
