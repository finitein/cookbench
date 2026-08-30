import type { StoveWire } from "../types/stove";
import { HarnessMark } from "./HarnessMark";
import { StoveBurner } from "./StoveBurner";
import "./detached-stove-bar.css";

export type DetachedStoveBarProps = {
  stove: StoveWire;
  onActivate?: (stove: StoveWire) => void;
  onClose?: (stove: StoveWire) => void;
  onClear?: (stove: StoveWire) => void;
  onPin?: (stove: StoveWire) => void;
  onArchive?: (stove: StoveWire) => void;
  onStartDrag?: () => void;
  activeAlertStoveId?: string | null;
};

/** One movable view of one Stove; it intentionally shares the global burner. */
export function DetachedStoveBar({ stove, onActivate, onClose, onClear, onPin, onArchive, onStartDrag, activeAlertStoveId = null }: DetachedStoveBarProps) {
  return (
    <section
      className="detached-stove-bar"
      aria-label={`Detached Stove bar for ${stove.harness.label}`}
      data-tauri-drag-region
      onPointerDown={(event) => {
        if (event.button > 0 || (event.target as Element).closest("button")) return;
        onStartDrag?.();
      }}
    >
      <div className="detached-stove-bar__harness" data-tauri-drag-region>
        <HarnessMark harness={stove.harness} />
      </div>
      <StoveBurner
        stove={stove}
        onActivate={onActivate}
        onPin={onPin}
        onArchive={onArchive}
        renderTooltip={false}
        showHarnessMark={false}
        flashing={activeAlertStoveId === stove.id}
      />
      <button
        className="detached-stove-bar__close"
        type="button"
        aria-label="Close detached Stove"
        title="Close"
        onClick={() => onClose?.(stove)}
      >
        <span aria-hidden="true" />
      </button>
      {onClear ? (
        <button
          className="detached-stove-bar__clear"
          type="button"
          aria-label={`Clear ${stove.harness.label} Stove`}
          title={`Clear ${stove.harness.label} Stove`}
          onClick={() => onClear(stove)}
        >
          <span aria-hidden="true">x</span>
        </button>
      ) : null}
    </section>
  );
}
