import { api, type ServerStatus, type InstalledSoftware, type SystemCapabilities, type ServiceSummary, type SoftwarePackage, type StorageDisk, type StorageMount, type SystemInfo } from "../api/client";
import { createSource } from "./source";

const MINUTE = 60_000;

/** The Server's own state and its view of the Agent. */
export const serverStatus = createSource<ServerStatus>(() => api.serverStatus(), MINUTE / 2);

export const systemInfo = createSource<SystemInfo>(() => api.systemInfo(), 5 * MINUTE);

export const storageMounts = createSource<StorageMount[]>(() => api.storage(), MINUTE);

/** Disks are read with the mounts; a host without `lsblk` simply has none. */
export const storageDisks = createSource<StorageDisk[]>(() => api.storageDisks().catch(() => []), MINUTE);

export const servicesList = createSource<ServiceSummary[]>(() => api.services(), MINUTE / 2);

export const softwarePackages = createSource<SoftwarePackage[]>(() => api.software(), 10 * MINUTE);

/** Every installed package; scanning it is slow, so it is kept longest. */
export const installedSoftware = createSource<InstalledSoftware[]>(() => api.installedSoftware(), 10 * MINUTE);

/** What the host allows: restart and self-update, as `agent.toml` sets them. */
export const systemCapabilities = createSource<SystemCapabilities>(() => api.systemCapabilities(), 5 * MINUTE);

export function resetSources() {
  for (const source of [serverStatus, systemInfo, storageMounts, storageDisks, servicesList, softwarePackages, installedSoftware, systemCapabilities]) source.reset();
}
