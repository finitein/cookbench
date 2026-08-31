import { useEffect, useState } from "react";

import {
  closeDetachedBar,
  configureDisplaySettings,
  getDisplaySettings,
  getLaunchAtLogin,
  setLaunchAtLogin,
  type DisplaySettingsWire,
  type GlobalBarPlacement,
  type AppLocale,
} from "./service";
import { useI18n, type TranslationKey } from "../../i18n/i18n";
import "./display-settings.css";

const PLACEMENTS: GlobalBarPlacement[] = ["topLeft", "topCenter", "topRight", "bottomLeft", "bottomCenter", "bottomRight"];
const LOCALES: AppLocale[] = ["system", "en", "zh-CN", "ja", "ko"];
const PLACEMENT_KEYS: Record<GlobalBarPlacement, TranslationKey> = {
  topLeft: "display.topLeft", topCenter: "display.topCenter", topRight: "display.topRight",
  bottomLeft: "display.bottomLeft", bottomCenter: "display.bottomCenter", bottomRight: "display.bottomRight",
};
const LOCALE_KEYS: Record<AppLocale, TranslationKey> = {
  system: "language.system", en: "language.en", "zh-CN": "language.zh-CN", ja: "language.ja", ko: "language.ko",
};

export function DisplaySettingsPanel() {
  const { t } = useI18n();
  const [settings, setSettings] = useState<DisplaySettingsWire | null>(null);
  const [status, setStatus] = useState<TranslationKey | null>(null);
  const [closing, setClosing] = useState<string | null>(null);
  const [launchAtLogin, setLaunchAtLoginState] = useState<boolean | null>(null);

  useEffect(() => {
    void getDisplaySettings().then(setSettings).catch(() => {
      setStatus("display.unavailable");
    });
    void getLaunchAtLogin().then((preference) => {
      setLaunchAtLoginState(preference.enabled);
    }).catch(() => {
      setStatus("display.loginUnavailable");
    });
  }, []);

  const save = (change: Partial<Pick<
    DisplaySettingsWire,
    "globalBarVisible" | "globalBarPlacement" | "hoverDetailsEnabled" | "locale"
  >>) => {
    if (!settings) return;
    const next = { ...settings, ...change };
    setSettings(next);
    setStatus(null);
    void configureDisplaySettings({
      globalBarVisible: next.globalBarVisible,
      globalBarPlacement: next.globalBarPlacement,
      hoverDetailsEnabled: next.hoverDetailsEnabled,
      locale: next.locale,
    }).then(setSettings).catch(() => {
      setSettings(settings);
      setStatus("display.saveFailed");
    });
  };

  const close = (stoveId: string) => {
    setClosing(stoveId);
    setStatus(null);
    void closeDetachedBar(stoveId).then((closed) => {
      if (closed) {
        setSettings((current) => current && {
          ...current,
          detachedBars: current.detachedBars.filter((bar) => bar.stoveId !== stoveId),
        });
      }
    }).catch(() => {
      setStatus("display.closeFailed");
    }).finally(() => setClosing(null));
  };

  return (
    <section className="display-settings" aria-labelledby="display-settings-title">
      <header>
        <h2 id="display-settings-title">{t("display.title")}</h2>
      </header>
      <section className="display-settings__section" aria-labelledby="global-bar-title">
        <div>
          <h3 id="global-bar-title">{t("display.global")}</h3>
          <p>{t("display.globalDescription")}</p>
        </div>
        <label className="display-settings__toggle">
          <input
            type="checkbox"
            checked={settings?.globalBarVisible ?? false}
            disabled={!settings}
            onChange={(event) => save({ globalBarVisible: event.target.checked })}
          />
          <span>{t("display.showGlobal")}</span>
        </label>
        <label className="display-settings__toggle">
          <input
            type="checkbox"
            checked={settings?.hoverDetailsEnabled ?? false}
            disabled={!settings || !settings.globalBarVisible}
            onChange={(event) => save({ hoverDetailsEnabled: event.target.checked })}
          />
          <span>{t("display.hover")}</span>
        </label>
        <label className="display-settings__placement">
          <span>{t("display.placement")}</span>
          <select
            value={settings?.globalBarPlacement ?? "topCenter"}
            disabled={!settings || !settings.globalBarVisible}
            onChange={(event) => save({ globalBarPlacement: event.target.value as GlobalBarPlacement })}
          >
            {PLACEMENTS.map((placement) => <option key={placement} value={placement}>{t(PLACEMENT_KEYS[placement])}</option>)}
          </select>
        </label>
      </section>
      <section className="display-settings__section" aria-labelledby="language-title">
        <div>
          <h3 id="language-title">{t("language.title")}</h3>
          <p>{t("language.description")}</p>
        </div>
        <label className="display-settings__placement">
          <span>{t("language.title")}</span>
          <select
            value={settings?.locale ?? "system"}
            disabled={!settings}
            onChange={(event) => save({ locale: event.target.value as AppLocale })}
          >
            {LOCALES.map((locale) => <option key={locale} value={locale}>{t(LOCALE_KEYS[locale])}</option>)}
          </select>
        </label>
      </section>
      <section className="display-settings__section" aria-labelledby="desktop-behavior-title">
        <div>
          <h3 id="desktop-behavior-title">{t("display.desktop")}</h3>
          <p>{t("display.desktopDescription")}</p>
        </div>
        <label className="display-settings__toggle">
          <input
            type="checkbox"
            checked={launchAtLogin ?? false}
            disabled={launchAtLogin === null}
            onChange={(event) => {
              const previous = launchAtLogin;
              const enabled = event.target.checked;
              setLaunchAtLoginState(enabled);
              setStatus(null);
              void setLaunchAtLogin(enabled)
                .then((preference) => setLaunchAtLoginState(preference.enabled))
                .catch(() => {
                  setLaunchAtLoginState(previous);
                  setStatus("display.loginFailed");
                });
            }}
          />
          <span>{t("display.login")}</span>
        </label>
      </section>
      <section className="display-settings__section" aria-labelledby="independent-bars-title">
        <div>
          <h3 id="independent-bars-title">{t("display.independent")}</h3>
          <p>{t("display.independentDescription")}</p>
        </div>
        {settings?.detachedBars.length ? (
          <ul className="display-settings__bars">
            {settings.detachedBars.map((bar) => (
              <li key={bar.stoveId}>
                <span>{t("display.independent")} {bar.stoveId}</span>
                <button
                  type="button"
                  disabled={closing === bar.stoveId}
                  aria-label={t("display.closeIndependent", { id: bar.stoveId })}
                  onClick={() => close(bar.stoveId)}
                >
                  {t("common.close")}
                </button>
              </li>
            ))}
          </ul>
        ) : <p className="display-settings__empty">{t("display.empty")}</p>}
      </section>
      <output role="status" aria-live="polite">{status ? t(status) : ""}</output>
    </section>
  );
}
