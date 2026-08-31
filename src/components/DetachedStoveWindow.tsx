import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect } from "react";
import type { StoveWire } from "../types/stove";
import { createPositionPersistence, detachedStoveTransport, startDetachedWindowDrag } from "../services/detachedStoves";
import { DetachedStoveBar } from "./DetachedStoveBar";
import { archiveStove, setStovePinned } from "../services/stoves";
import { useI18n } from "../i18n/i18n";

export type DetachedStoveWindowProps = {
  stove: StoveWire;
  onActivate?: (stove: StoveWire) => void;
  activeAlertStoveId?: string | null;
};

export function DetachedStoveWindow({ stove, onActivate, activeAlertStoveId }: DetachedStoveWindowProps) {
  const { t } = useI18n();
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
    <main className="shell shell--detached" aria-label={t("bar.detached", { name: "Cookbench" })}>
      <DetachedStoveBar
        stove={stove}
        onActivate={onActivate}
        onClose={(current) => { void detachedStoveTransport.close(current.id); }}
        onStartDrag={() => { void startDetachedWindowDrag(); }}
        onClear={stove.retainedCompletion ? (current) => { void detachedStoveTransport.clear(current.id); } : undefined}
        onPin={(current) => { void setStovePinned(current.id, !current.pinned); }}
        onArchive={stove.retainedCompletion ? undefined : (current) => { void archiveStove(current.id); }}
        activeAlertStoveId={activeAlertStoveId}
      />
    </main>
  );
}
