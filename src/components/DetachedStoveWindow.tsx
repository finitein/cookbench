import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect } from "react";
import type { StoveWire } from "../types/stove";
import { createPositionPersistence, detachedStoveTransport, startDetachedWindowDrag } from "../services/detachedStoves";
import { DetachedStoveBar } from "./DetachedStoveBar";
import { archiveStove, setStovePinned } from "../services/stoves";

export type DetachedStoveWindowProps = {
  stove: StoveWire;
  onActivate?: (stove: StoveWire) => void;
};

export function DetachedStoveWindow({ stove, onActivate }: DetachedStoveWindowProps) {
  useEffect(() => {
    const persistence = createPositionPersistence(({ x, y }) => {
      void detachedStoveTransport.recordPosition(stove.id, x, y);
    });
    let disposed = false;
    let stop: (() => void) | undefined;
    try {
      void getCurrentWebviewWindow().onMoved(({ payload }) => {
        persistence.schedule(payload);
      }).then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      });
    } catch {
      // Browser fixtures do not expose a native window; production Tauri does.
    }
    return () => {
      disposed = true;
      persistence.flush();
      stop?.();
    };
  }, [stove.id]);

  return (
    <main className="shell shell--detached" aria-label="Cookbench detached Stove">
      <DetachedStoveBar
        stove={stove}
        onActivate={onActivate}
        onClose={(current) => { void detachedStoveTransport.close(current.id); }}
        onStartDrag={() => { void startDetachedWindowDrag(); }}
        onClear={stove.retainedCompletion ? (current) => { void detachedStoveTransport.clear(current.id); } : undefined}
        onPin={(current) => { void setStovePinned(current.id, !current.pinned); }}
        onArchive={stove.retainedCompletion ? undefined : (current) => { void archiveStove(current.id); }}
      />
    </main>
  );
}
