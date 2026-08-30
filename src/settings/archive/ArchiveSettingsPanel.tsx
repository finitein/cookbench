import { useEffect, useState } from "react";

import { getArchivedSessions, restoreArchivedSession } from "../../services/stoves";
import type { ArchivedSessionWire } from "../../types/stove";

function archiveReasonLabel(reason: ArchivedSessionWire["reason"]): string {
  return reason === "expired" ? "Expired after 2 days" : "Deleted manually";
}

function archiveTime(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(new Date(timestamp));
}

export function ArchiveSettingsPanel() {
  const [sessions, setSessions] = useState<ArchivedSessionWire[]>([]);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [restoring, setRestoring] = useState<string | null>(null);
  const [message, setMessage] = useState("");

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
    setMessage("");
    try {
      await restoreArchivedSession(session.id);
      setSessions((current) => current.filter((candidate) => candidate.id !== session.id));
      setMessage("Session restored.");
    } catch {
      setMessage(session.sourceAvailable ? "Session could not be restored." : "The native session source is no longer available.");
    } finally {
      setRestoring(null);
    }
  };

  return (
    <section className="archive-settings" aria-labelledby="archive-settings-title">
      <div className="notification-settings__section-heading">
        <h2 id="archive-settings-title">Archive</h2>
      </div>
      {status === "loading" ? <p role="status">Loading archived sessions.</p> : null}
      {status === "error" ? <p role="alert">Archived sessions are unavailable.</p> : null}
      {status === "ready" && sessions.length === 0 ? <p>No archived sessions.</p> : null}
      {status === "ready" && sessions.length > 0 ? (
        <div className="archive-settings__list" role="list" aria-label="Archived sessions">
          {sessions.map((session) => (
            <article className="archive-settings__item" role="listitem" key={session.id}>
              <div>
                <strong>{session.projectLabel?.trim() || "Session"}</strong>
                <span>{session.harness.label} · {session.sessionIdentity}</span>
                {session.projectRootDisplay ? <span title={session.projectRootDisplay}>{session.projectRootDisplay}</span> : null}
                <span>{archiveReasonLabel(session.reason)} · {session.lastState} · {archiveTime(session.archivedAtMs)}</span>
              </div>
              <button
                type="button"
                disabled={restoring === session.id}
                onClick={() => void restore(session)}
              >
                Restore
              </button>
            </article>
          ))}
        </div>
      ) : null}
      <output role="status" aria-live="polite">{message}</output>
    </section>
  );
}
