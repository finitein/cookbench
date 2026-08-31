import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { I18nProvider, resolveLocale, useI18n } from "./i18n";

function Probe() {
  const { t } = useI18n();
  return <span>{[t("display.title"), t("notifications.title"), t("tooltip.activity"), t("remote.title")].join("|")}</span>;
}

describe("Cookbench interface locales", () => {
  it("resolves supported system languages and falls back to English", () => {
    expect(resolveLocale("system", "zh-Hans-CN")).toBe("zh-CN");
    expect(resolveLocale("system", "ja-JP")).toBe("ja");
    expect(resolveLocale("system", "ko-KR")).toBe("ko");
    expect(resolveLocale("system", "fr-FR")).toBe("en");
  });

  it("changes chrome and the document language without translating user data", () => {
    render(<I18nProvider preference="zh-CN"><Probe /></I18nProvider>);
    expect(screen.getByText(/显示\|通知\|活动\|SSH 来源/)).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("zh-CN");
  });

  it.each([
    ["ja" as const, /表示\|通知\|アクティビティ\|SSH ソース/],
    ["ko" as const, /표시\|알림\|활동\|SSH 소스/],
  ])("translates technical Settings and Stove tooltip chrome in %s", (preference, expected) => {
    render(<I18nProvider preference={preference}><Probe /></I18nProvider>);
    expect(screen.getByText(expected)).toBeInTheDocument();
  });
});
