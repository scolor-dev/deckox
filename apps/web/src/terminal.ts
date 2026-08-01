export type TerminalServerMessage =
  | { type: "ready" }
  | { type: "exit" }
  | { type: "error"; code: string };

export function terminalWebSocketUrl(location: Pick<Location, "protocol" | "host">): string {
  const protocol = location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${location.host}/api/v1/terminal/ws`;
}

export function parseTerminalServerMessage(value: string): TerminalServerMessage | null {
  try {
    const message = JSON.parse(value) as Partial<TerminalServerMessage>;
    if (message.type === "ready" || message.type === "exit") return { type: message.type };
    if (message.type === "error" && typeof message.code === "string") {
      return { type: "error", code: message.code };
    }
    return null;
  } catch {
    return null;
  }
}
