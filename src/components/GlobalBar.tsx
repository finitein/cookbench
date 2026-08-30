import { useEffect, useRef } from "react";
import type { StoveState, StoveWire } from "../types/stove";
import mark from "../assets/cookbench-mark.svg";
import { StoveBurner } from "./StoveBurner";
import "./global-bar.css";

export type GlobalBarProps = {
  stoves: readonly StoveWire[];
  onActivateStove?: (stove: StoveWire) => void;
  onDetachStove?: (stove: StoveWire) => void;
  onClearStove?: (stove: StoveWire) => void;
  onOpenSettings?: () => void;
};

export function GlobalBar({ stoves, onActivateStove, onDetachStove, onClearStove, onOpenSettings }: GlobalBarProps) {
  const previousStates = useRef(new Map<string, StoveState>());
  const priorStates = previousStates.current;

  useEffect(() => {
    previousStates.current = new Map(stoves.map((stove) => [stove.id, stove.state]));
  }, [stoves]);

  return (
    <section
      className={`global-bar${stoves.length === 0 ? " global-bar--empty" : ""}`}
      aria-label={`Cookbench global bar with ${stoves.length} stoves`}
      style={{
        "--stove-grid-width": `${Math.min(stoves.length, 8) * 86}px`,
      } as React.CSSProperties}
    >
      <div className="global-bar__brand" aria-label="Cookbench">
        <img src={mark} alt="Cookbench" />
        {onOpenSettings ? (
          <button
            className="global-bar__settings"
            type="button"
            aria-label="Open Cookbench settings"
            title="Settings"
            onClick={onOpenSettings}
          >
            <span aria-hidden="true"><i /><i /><i /></span>
          </button>
        ) : null}
      </div>
      <div className="global-bar__stoves" role="list" aria-label="Stoves">
        {stoves.map((stove) => (
          <div className="global-bar__item" role="listitem" key={stove.id}>
            <StoveBurner
              stove={stove}
              onActivate={onActivateStove}
              onDetach={onDetachStove}
              onClear={onClearStove}
              previousState={priorStates.get(stove.id)}
              isInitialSnapshot={!priorStates.has(stove.id)}
            />
          </div>
        ))}
      </div>
    </section>
  );
}
