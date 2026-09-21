import { WEB_MODULES } from "./registry";

interface MessageTarget {
  global: { mergeLocaleMessage: (locale: "ja" | "en", messages: Record<string, unknown>) => void };
}

/** Adds each module's own texts to the app's messages. */
export function mergeModuleMessages(i18n: MessageTarget) {
  for (const module of WEB_MODULES) {
    if (!module.messages) continue;
    i18n.global.mergeLocaleMessage("ja", module.messages.ja);
    i18n.global.mergeLocaleMessage("en", module.messages.en);
  }
}
