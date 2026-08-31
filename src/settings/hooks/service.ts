import { invoke } from "@tauri-apps/api/core";

export type HookHarness = "codex" | "claudeCode" | "pi" | "kimiCode" | "zcode" | (string & {});
export type HookHealth = "detected" | "notInstalled" | "healthy" | "outdated" | "conflicted" | "unwritable" | "noRecentEvents";
export type HookAction = "previewInstall" | "install" | "repair" | "uninstall";
export type HookSupportTier = "full" | "standard" | "experimental";
export type HookIntegration = "automatic" | "manual" | "presenceOnly";

export type HookStatus = {
  harness: HookHarness;
  label: string;
  tier: HookSupportTier;
  integration: HookIntegration;
  health: HookHealth;
  configDisplay: string;
  detail: string;
  canInstall: boolean;
  canRepair: boolean;
  canUninstall: boolean;
};

export type HookActionResult = {
  status: HookStatus;
  changed: boolean;
  preview: string | null;
  backupDisplay: string | null;
};

export function getHookStatus(): Promise<HookStatus[]> {
  return invoke<HookStatus[]>("get_hook_status");
}

export function manageHook(harness: HookHarness, action: HookAction): Promise<HookActionResult> {
  return invoke<HookActionResult>("manage_hook", { request: { harness, action } });
}
