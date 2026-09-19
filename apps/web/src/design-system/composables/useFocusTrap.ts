import { nextTick, onBeforeUnmount, watch, type Ref } from "vue";

const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type='hidden'])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

const activeTraps: symbol[] = [];

export function focusableWithin(container: HTMLElement): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => !el.hidden && el.getAttribute("aria-hidden") !== "true");
}

/**
 * Keeps keyboard focus inside `container` while `open()` is true: focus moves
 * into the container on open (to `[data-autofocus]` when present, otherwise
 * the container itself — give it tabindex="-1"), Tab / Shift+Tab wrap at the
 * ends, and focus returns to the previously focused element on close. With
 * several traps open at once only the most recently opened one reacts.
 */
export function useFocusTrap(container: Ref<HTMLElement | null>, open: () => boolean) {
  const id = Symbol("focus-trap");
  let previouslyFocused: HTMLElement | null = null;

  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== "Tab" || activeTraps.at(-1) !== id || !container.value) return;
    const items = focusableWithin(container.value);
    if (items.length === 0) {
      event.preventDefault();
      container.value.focus();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const current = document.activeElement;
    if (event.shiftKey && (current === first || current === container.value)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && current === last) {
      event.preventDefault();
      first.focus();
    } else if (current instanceof Node && !container.value.contains(current)) {
      event.preventDefault();
      first.focus();
    }
  }

  function release() {
    const index = activeTraps.indexOf(id);
    if (index === -1) return;
    activeTraps.splice(index, 1);
    window.removeEventListener("keydown", handleKeydown);
    previouslyFocused?.focus();
    previouslyFocused = null;
  }

  watch(open, async (isOpen) => {
    if (!isOpen) {
      release();
      return;
    }
    previouslyFocused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    activeTraps.push(id);
    window.addEventListener("keydown", handleKeydown);
    await nextTick();
    const target = container.value?.querySelector<HTMLElement>("[data-autofocus]") ?? container.value;
    target?.focus();
  }, { flush: "post" });

  onBeforeUnmount(release);
}
