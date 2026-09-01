import { useEffect, useRef, useState } from "react";

import { stoveDisplayIdentity, type StoveWire } from "../types/stove";
import { stoveStateLabel } from "./ProgressRing";
import { useI18n } from "../i18n/i18n";

export type StovePriorityMenuProps = {
  stoves: readonly StoveWire[];
  onActivate?: (stove: StoveWire) => void;
  onClose: (restoreFocus: boolean) => void;
};

/** The full attention list remains reachable while the Bar is intentionally tiny. */
export function StovePriorityMenu({ stoves, onActivate, onClose }: StovePriorityMenuProps) {
  const { t } = useI18n();
  const menuRef = useRef<HTMLDivElement>(null);
  const [activeIndex, setActiveIndex] = useState(0);

  const moveFocus = (index: number) => {
    const items = [...(menuRef.current?.querySelectorAll<HTMLButtonElement>("[role='menuitem']") ?? [])];
    const next = (index + items.length) % items.length;
    setActiveIndex(next);
    items.at(next)?.focus();
  };

  useEffect(() => {
    const firstItem = menuRef.current?.querySelector<HTMLButtonElement>("[role='menuitem']");
    firstItem?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      const items = [...(menuRef.current?.querySelectorAll<HTMLButtonElement>("[role='menuitem']") ?? [])];
      const index = items.indexOf(document.activeElement as HTMLButtonElement);
      if (event.key === "Escape") {
        event.preventDefault();
        onClose(true);
      } else if (event.key === "ArrowDown") {
        event.preventDefault();
        moveFocus(index < 0 ? 0 : index + 1);
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        moveFocus(index < 0 ? items.length - 1 : index - 1);
      } else if (event.key === "Home") {
        event.preventDefault();
        moveFocus(0);
      } else if (event.key === "End") {
        event.preventDefault();
        moveFocus(items.length - 1);
      } else if (event.key === "Tab") {
        onClose(false);
      }
    };
    const onPointerDown = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) onClose(false);
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
      <p className="stove-priority-menu__title" aria-hidden="true">{t("bar.priorityList")}</p>
      {stoves.map((stove, index) => (
        <button
          className="stove-priority-menu__item"
          type="button"
          role="menuitem"
          key={stove.id}
          tabIndex={index === activeIndex ? 0 : -1}
          onClick={() => {
            onActivate?.(stove);
            onClose(true);
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
