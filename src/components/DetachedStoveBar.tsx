import type { StoveWire } from "../types/stove";
import { HarnessMark } from "./HarnessMark";
import { StoveBurner } from "./StoveBurner";
import { useI18n } from "../i18n/i18n";
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
  const { t } = useI18n();
  return (
    <section
      className="detached-stove-bar"
      aria-label={t("bar.detached", { name: stove.harness.label })}
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
        aria-label={t("stove.closeDetached")}
        title={t("common.close")}
        onClick={() => onClose?.(stove)}
      >
        <span aria-hidden="true" />
      </button>
      {onClear ? (
        <button
          className="detached-stove-bar__clear"
          type="button"
          aria-label={t("stove.clear", { name: stove.harness.label })}
          title={t("stove.clear", { name: stove.harness.label })}
          onClick={() => onClear(stove)}
        >
          <span aria-hidden="true">x</span>
        </button>
      ) : null}
    </section>
  );
}
