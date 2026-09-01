import { useEffect, useRef } from "react";

import { stoveDisplayIdentity, type StoveWire } from "../types/stove";
import { stoveStateLabel } from "./ProgressRing";
import { useI18n } from "../i18n/i18n";

export type StovePriorityMenuProps = {
  stoves: readonly StoveWire[];
  onActivate?: (stove: StoveWire) => void;
  onClose: () => void;
};

/** The full attention list remains reachable while the Bar is intentionally tiny. */
export function StovePriorityMenu({ stoves, onActivate, onClose }: StovePriorityMenuProps) {
  const { t } = useI18n();
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const firstItem = menuRef.current?.querySelector<HTMLButtonElement>("[role='menuitem']");
    firstItem?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    const onPointerDown = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("mousedown", onPointerDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("mousedown", onPointerDown);
    };
  }, [onClose]);

  return (
    <div className="stove-priority-menu" ref={menuRef} role="menu" aria-label={t("bar.priorityList")}>
      <p className="stove-priority-menu__title">{t("bar.priorityList")}</p>
      {stoves.map((stove, index) => (
        <button
          className="stove-priority-menu__item"
          type="button"
          role="menuitem"
          key={stove.id}
          onClick={() => {
            onActivate?.(stove);
            onClose();
          }}
        >
          <span className="stove-priority-menu__rank">{index + 1}</span>
          <span className="stove-priority-menu__identity">{stoveDisplayIdentity(stove, t("stove.session"))}</span>
          <span className="stove-priority-menu__state">{stoveStateLabel(stove.state, t)}</span>
        </button>
      ))}
    </div>
  );
}
