import type { StoveWire } from "../types/stove";
import { StoveBurner } from "./StoveBurner";
import "./detached-stove-bar.css";

export type DetachedStoveBarProps = {
  stove: StoveWire;
  onActivate?: (stove: StoveWire) => void;
  onClear?: (stove: StoveWire) => void;
};

/** One movable view of one Stove; it intentionally shares the global burner. */
export function DetachedStoveBar({ stove, onActivate, onClear }: DetachedStoveBarProps) {
  return (
    <section
      className="detached-stove-bar"
      aria-label={`Detached Stove bar for ${stove.harness.label}`}
      data-tauri-drag-region
    >
      <div className="detached-stove-bar__identity" aria-hidden="true">
        {stove.harness.label}
      </div>
      <StoveBurner stove={stove} onActivate={onActivate} />
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
