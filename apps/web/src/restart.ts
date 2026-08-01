export function hasServerRestarted(
  previousInstance: string | null,
  currentInstance: string,
  observedOffline: boolean,
): boolean {
  if (previousInstance) return previousInstance !== currentInstance;
  return observedOffline;
}
