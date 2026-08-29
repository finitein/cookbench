import { invoke } from "@tauri-apps/api/core";

export type RemoteSourceWire = {
  id: string;
  alias: string;
  sessionRoots: string[];
  enabled: boolean;
  bridgeEnabled: boolean;
  bridgeBinaryPath: string | null;
};

export type RemoteSourceInput = {
  id: string | null;
  alias: string;
  sessionRoots: string[];
  enabled: boolean;
  bridgeEnabled: boolean;
  bridgeBinaryPath: string | null;
};

export function getRemoteSources(): Promise<RemoteSourceWire[]> {
  return invoke<RemoteSourceWire[]>("get_remote_sources");
}

export function configureRemoteSource(input: RemoteSourceInput): Promise<RemoteSourceWire[]> {
  return invoke<RemoteSourceWire[]>("configure_remote_source", { input });
}

export function removeRemoteSource(id: string): Promise<RemoteSourceWire[]> {
  return invoke<RemoteSourceWire[]>("remove_remote_source", { id });
}
