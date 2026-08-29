import { useEffect, useRef } from "react";
import type { StoveState, StoveWire } from "../types/stove";
import mark from "../assets/cookbench-mark.svg";
import { StoveBurner } from "./StoveBurner";
import "./global-bar.css";

export type GlobalBarProps = {
  stoves: readonly StoveWire[];
  onActivateStove?: (stove: StoveWire) => void;
};

export function GlobalBar({ stoves, onActivateStove }: GlobalBarProps) {
  const previousStates = useRef(new Map<string, StoveState>());
  const priorStates = previousStates.current;

  useEffect(() => {
    previousStates.current = new Map(stoves.map((stove) => [stove.id, stove.state]));
  }, [stoves]);

  return (
    <section className="global-bar" aria-label={`Cookbench global bar with ${stoves.length} stoves`}>
      <div className="global-bar__brand" aria-label="Cookbench">
        <img src={mark} alt="" />
      </div>
      <div className="global-bar__stoves" role="list" aria-label="Stoves">
        {stoves.map((stove) => (
          <div className="global-bar__item" role="listitem" key={stove.id}>
            <StoveBurner
              stove={stove}
              onActivate={onActivateStove}
              previousState={priorStates.get(stove.id)}
              isInitialSnapshot={!priorStates.has(stove.id)}
            />
          </div>
        ))}
      </div>
    </section>
  );
}
