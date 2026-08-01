import { describe, expect, it } from "vitest";
import { parseTerminalServerMessage, terminalWebSocketUrl } from "./terminal";

describe("terminalWebSocketUrl", () => {
  it("uses a secure websocket for HTTPS", () => {
    expect(terminalWebSocketUrl({ protocol: "https:", host: "deckox.local" }))
      .toBe("wss://deckox.local/api/v1/terminal/ws");
  });
});

describe("parseTerminalServerMessage", () => {
  it("rejects unknown control messages", () => {
    expect(parseTerminalServerMessage('{"type":"command"}')).toBeNull();
  });

  it("parses an error code", () => {
    expect(parseTerminalServerMessage('{"type":"error","code":"terminal_idle_timeout"}'))
      .toEqual({ type: "error", code: "terminal_idle_timeout" });
  });
});
