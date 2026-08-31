import { useEffect, useState } from "react";

import { getArchivedSessions, restoreArchivedSession } from "../../services/stoves";
import type { ArchivedSessionWire } from "../../types/stove";
import { useI18n, type TranslationKey } from "../../i18n/i18n";
import { stoveStateLabel } from "../../components/ProgressRing";

export function ArchiveSettingsPanel() {
  const { locale, t } = useI18n();
  const [sessions, setSessions] = useState<ArchivedSessionWire[]>([]);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [restoring, setRestoring] = useState<string | null>(null);
  const [message, setMessage] = useState<TranslationKey | null>(null);

  useEffect(() => {
    let active = true;
    void getArchivedSessions().then((next) => {
      if (!active) return;
      setSessions(next);
      setStatus("ready");
    }).catch(() => {
      if (!active) return;
      setStatus("error");
    });
    return () => { active = false; };
  }, []);

  const restore = async (session: ArchivedSessionWire) => {
    setRestoring(session.id);
    setMessage(null);
    try {
      await restoreArchivedSession(session.id);
      setSessions((current) => current.filter((candidate) => candidate.id !== session.id));
      setMessage("archive.restored");
    } catch {
      setMessage(session.sourceAvailable ? "archive.restoreFailed" : "archive.sourceMissing");
    } finally {
      setRestoring(null);
    }
  };

  return (
    <section className="archive-settings" aria-labelledby="archive-settings-title">
      <div className="notification-settings__section-heading">
        <h2 id="archive-settings-title">{t("settings.archive")}</h2>
      </div>
      {status === "loading" ? <p role="status">{t("archive.loading")}</p> : null}
      {status === "error" ? <p role="alert">{t("archive.unavailable")}</p> : null}
      {status === "ready" && sessions.length === 0 ? <p>{t("archive.empty")}</p> : null}
      {status === "ready" && sessions.length > 0 ? (
        <div className="archive-settings__list" role="list" aria-label={t("archive.list")}>
          {sessions.map((session) => (
            <article className="archive-settings__item" role="listitem" key={session.id}>
              <div>
                <strong>{session.projectLabel?.trim() || t("stove.session")}</strong>
                <span>{session.harness.label} · {session.sessionIdentity}</span>
                {session.projectRootDisplay ? <span title={session.projectRootDisplay}>{session.projectRootDisplay}</span> : null}
                <span>{session.reason === "expired" ? t("archive.expired") : t("archive.deleted")} · {stoveStateLabel(session.lastState, t)} · {new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(session.archivedAtMs))}</span>
              </div>
              <button
                type="button"
                disabled={restoring === session.id}
                onClick={() => void restore(session)}
              >
                {t("common.restore")}
              </button>
            </article>
          ))}
        </div>
      ) : null}
      <output role="status" aria-live="polite">{message ? t(message) : ""}</output>
    </section>
  );
}
