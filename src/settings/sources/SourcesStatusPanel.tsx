import { useEffect, useMemo, useState } from "react";

import { getLocalSourceStatus, type LocalSourceHealth, type LocalSourceObservation, type LocalSourceStatus, type LocalSourceSupportTier } from "./service";
import { useI18n, type TranslationKey } from "../../i18n/i18n";
import "./source-status.css";

const TIER_KEYS: Record<LocalSourceSupportTier, TranslationKey> = {
  full: "sources.full", standard: "sources.standard", experimental: "sources.experimental",
};
const HEALTH_KEYS: Record<LocalSourceHealth, TranslationKey> = {
  healthy: "sources.healthy", degraded: "sources.degraded", unavailable: "sources.unavailableStatus",
};
const OBSERVATION_KEYS: Record<LocalSourceObservation, TranslationKey> = {
  nativeSessions: "sources.nativeSessions", structuredHook: "sources.structuredHook", presenceOnly: "sources.presenceOnly",
};

export function SourcesStatusPanel() {
  const { t } = useI18n();
  const [sources, setSources] = useState<LocalSourceStatus[]>([]);
  const [status, setStatus] = useState<TranslationKey | null>(null);

  useEffect(() => {
    void getLocalSourceStatus().then((response) => {
      setSources(response.sources);
    }).catch(() => {
      setStatus("sources.unavailable");
    });
  }, []);

  const { monitored, unavailable } = useMemo(() => {
    const monitoredList: LocalSourceStatus[] = [];
    const unavailableList: LocalSourceStatus[] = [];
    for (const source of sources) {
      if (source.health === "unavailable") unavailableList.push(source);
      else monitoredList.push(source);
    }
    return { monitored: monitoredList, unavailable: unavailableList };
  }, [sources]);

  const renderSource = (source: LocalSourceStatus) => (
    <li key={source.harness} className="source-status__source">
      <div className="source-status__identity">
        <strong>{source.label}</strong>
        <span className={`source-status__tier source-status__tier--${source.tier}`}>{t(TIER_KEYS[source.tier])}</span>
        <span title={source.rootDisplay}>{source.rootDisplay}</span>
      </div>
      <div className="source-status__meta">
        <div className="source-status__details">
          <span className={`source-status__health source-status__health--${source.health}`}>
            {t(HEALTH_KEYS[source.health])}
          </span>
          <span>{t(OBSERVATION_KEYS[source.observation])}</span>
          <span>{t(source.discoveredSessions === 1 ? "sources.sessionCount" : "sources.sessionCountPlural", { count: source.discoveredSessions })}</span>
          {source.parserErrors > 0 ? <span>{t(source.parserErrors === 1 ? "sources.issueCount" : "sources.issueCountPlural", { count: source.parserErrors })}</span> : null}
        </div>
        {source.health === "unavailable" ? (
          <p className="source-status__hint" title={source.rootDisplay}>{t("sources.unavailableHint")}</p>
        ) : (
          <p className="source-status__hint">{t("sources.monitoringHint")}</p>
        )}
      </div>
    </li>
  );

  return (
    <section className="source-status" aria-labelledby="source-status-title">
      <header>
        <h2 id="source-status-title">{t("sources.title")}</h2>
        <p>{t("sources.description")}</p>
      </header>
      <p className="source-status__next-step">{t("sources.nextStep")}</p>
      <ul className="source-status__list" aria-label={t("sources.list")}>
        {monitored.length ? (
          <>
            {unavailable.length ? <li className="source-status__group" role="presentation"><h3 className="source-status__group-heading">{t("sources.groupMonitored")}</h3></li> : null}
            {monitored.map(renderSource)}
          </>
        ) : null}
        {unavailable.length ? (
          <>
            {monitored.length ? <li className="source-status__group" role="presentation"><h3 className="source-status__group-heading">{t("sources.groupUnavailable")}</h3></li> : null}
            {unavailable.map(renderSource)}
          </>
        ) : null}
      </ul>
      <output role="status" aria-live="polite">{status ? t(status) : ""}</output>
    </section>
  );
}
