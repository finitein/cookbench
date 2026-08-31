import { useEffect, useState } from "react";

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

  return (
    <section className="source-status" aria-labelledby="source-status-title">
      <header>
        <h2 id="source-status-title">{t("sources.title")}</h2>
        <p>{t("sources.description")}</p>
      </header>
      <ul className="source-status__list" aria-label={t("sources.list")}>
        {sources.map((source) => (
          <li key={source.harness} className="source-status__source">
            <div className="source-status__identity">
              <strong>{source.label}</strong>
              <span className={`source-status__tier source-status__tier--${source.tier}`}>{t(TIER_KEYS[source.tier])}</span>
              <span title={source.rootDisplay}>{source.rootDisplay}</span>
            </div>
            <div className="source-status__details">
              <span className={`source-status__health source-status__health--${source.health}`}>
                {t(HEALTH_KEYS[source.health])}
              </span>
              <span>{t(OBSERVATION_KEYS[source.observation])}</span>
              <span>{t(source.discoveredSessions === 1 ? "sources.sessionCount" : "sources.sessionCountPlural", { count: source.discoveredSessions })}</span>
              {source.parserErrors > 0 ? <span>{t(source.parserErrors === 1 ? "sources.issueCount" : "sources.issueCountPlural", { count: source.parserErrors })}</span> : null}
            </div>
          </li>
        ))}
      </ul>
      <output role="status" aria-live="polite">{status ? t(status) : ""}</output>
    </section>
  );
}
