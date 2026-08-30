import { useEffect, useMemo, useRef, useState } from "react";
import type { StoveState, StoveWire } from "../types/stove";
import mark from "../assets/cookbench-mark.svg";
import { arrangeBenches, stoveCapacityForWidth } from "./benchLayout";
import { StoveBurner } from "./StoveBurner";
import { StoveTooltip } from "./StoveTooltip";
import { LOCAL_ALERT_TEST_STOVE_ID } from "../services/localAlerts";
import "./global-bar.css";

export type GlobalBarProps = {
  stoves: readonly StoveWire[];
  onActivateStove?: (stove: StoveWire) => void;
  onDetachStove?: (stove: StoveWire) => void;
  onClearStove?: (stove: StoveWire) => void;
  onPinStove?: (stove: StoveWire) => void;
  onArchiveStove?: (stove: StoveWire) => void;
  onOpenSettings?: () => void;
  hoverDetailsEnabled?: boolean;
  activeAlertStoveId?: string | null;
};

function usableBarWidth(): number {
  if (typeof document === "undefined") {
    return 900;
  }

  // The logo rail and the Bar padding do not participate in Stove rows. JSDOM
  // reports a zero-sized document, so retain the desktop's normal initial size.
  const viewportWidth = document.documentElement.clientWidth || window.innerWidth || 900;
  return Math.max(86, viewportWidth - 82);
}

function useBenchCapacity(): number {
  const [capacity, setCapacity] = useState(() => stoveCapacityForWidth(usableBarWidth()));

  useEffect(() => {
    const refresh = () => setCapacity(stoveCapacityForWidth(usableBarWidth()));
    refresh();
    window.addEventListener("resize", refresh);
    const observer = typeof ResizeObserver === "undefined"
      ? null
      : new ResizeObserver(refresh);
    observer?.observe(document.documentElement);
    return () => {
      window.removeEventListener("resize", refresh);
      observer?.disconnect();
    };
  }, []);

  return capacity;
}

export function GlobalBar({
  stoves,
  onActivateStove,
  onDetachStove,
  onClearStove,
  onPinStove,
  onArchiveStove,
  onOpenSettings,
  hoverDetailsEnabled = false,
  activeAlertStoveId = null,
}: GlobalBarProps) {
  const previousStates = useRef(new Map<string, StoveState>());
  const priorStates = previousStates.current;
  const [tooltipStoveId, setTooltipStoveId] = useState<string | null>(null);
  const tooltipStove = hoverDetailsEnabled
    ? stoves.find((stove) => stove.id === tooltipStoveId) ?? null
    : null;
  const benchCapacity = useBenchCapacity();
  const layout = useMemo(() => arrangeBenches(stoves, benchCapacity), [benchCapacity, stoves]);

  useEffect(() => {
    previousStates.current = new Map(stoves.map((stove) => [stove.id, stove.state]));
  }, [stoves]);

  useEffect(() => {
    if (!hoverDetailsEnabled) setTooltipStoveId(null);
  }, [hoverDetailsEnabled]);

  return (
    <section
      className={`global-bar${stoves.length === 0 ? " global-bar--empty" : ""}${tooltipStove ? " global-bar--tooltip-open" : ""}${activeAlertStoveId === LOCAL_ALERT_TEST_STOVE_ID ? " global-bar--alert" : ""}`}
      aria-label={`Cookbench global bar with ${stoves.length} stoves`}
      data-layout={layout.grouped ? "grouped" : "mixed"}
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
      <div className="global-bar__benches" data-layout={layout.grouped ? "grouped" : "mixed"}>
        {layout.benches.map((bench) => (
          <section className="global-bar__bench" data-harness={bench.id} key={bench.id} aria-label={bench.label}>
            {layout.grouped ? <h2 className="global-bar__bench-heading">{bench.label}</h2> : null}
            <div
              className="global-bar__stoves global-bar__bench-stoves"
              role="list"
              aria-label={layout.grouped ? `${bench.label} stoves` : "Stoves"}
            >
              {bench.stoves.map((stove) => (
                <div className="global-bar__item" role="listitem" key={stove.id}>
                  <StoveBurner
                    stove={stove}
                    onActivate={onActivateStove}
                    onDetach={onDetachStove}
                    onClear={onClearStove}
                    onPin={onPinStove}
                    onArchive={onArchiveStove}
                    previousState={priorStates.get(stove.id)}
                    isInitialSnapshot={!priorStates.has(stove.id)}
                    tooltipId={hoverDetailsEnabled ? "global-bar-tooltip" : undefined}
                    renderTooltip={false}
                    flashing={activeAlertStoveId === stove.id}
                    onTooltipVisibilityChange={hoverDetailsEnabled
                      ? (visible, value) => setTooltipStoveId((current) => visible ? value.id : current === value.id ? null : current)
                      : undefined}
                  />
                </div>
              ))}
            </div>
          </section>
        ))}
      </div>
      {tooltipStove ? <StoveTooltip stove={tooltipStove} id="global-bar-tooltip" className="global-bar__tooltip" /> : null}
    </section>
  );
}
