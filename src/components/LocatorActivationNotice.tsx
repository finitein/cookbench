import { useEffect, useState } from "react";

import type { LocatorActivationResult } from "../services/locator";
import { useI18n } from "../i18n/i18n";

export function LocatorActivationNotice({ result }: { result: LocatorActivationResult | null }) {
  const { t } = useI18n();
  const [expiredResult, setExpiredResult] = useState<LocatorActivationResult | null>(null);

  useEffect(() => {
    if (!result || result.status === "focused") return;
    const timeout = window.setTimeout(() => setExpiredResult(result), 20_000);
    return () => window.clearTimeout(timeout);
  }, [result]);

  if (!result || result === expiredResult) return null;

  if (result.status === "focused") {
    return null;
  }

  if (result.target === "exactThread" && result.status === "visibleFallback") {
    return <output role="status" aria-live="polite">{t("locator.opening")}</output>;
  }

  if (result.status === "visibleFallback" && result.resumeSessionId) {
    return (
      <output role="status" aria-live="polite">
        {t("locator.fallbackWithId")} <code>{result.resumeSessionId}</code>
      </output>
    );
  }

  return <output role="status" aria-live="polite">{t("locator.fallback")}</output>;
}
