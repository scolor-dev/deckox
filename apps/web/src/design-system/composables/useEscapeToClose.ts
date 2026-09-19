import { onBeforeUnmount, watch } from "vue";

/**
 * Closes an open overlay (dialog, modal, popover) on Escape. Shared by
 * ConfirmDialog/AppModal/AppPopover, which otherwise each hand-rolled the
 * same watch-open/add-listener/remove-listener/cleanup-on-unmount sequence.
 *
 * `open` and `enabled` are getters (not raw values) so the caller's own
 * reactivity — usually a prop — stays the source of truth; this composable
 * holds no state of its own.
 */
export function useEscapeToClose(
  open: () => boolean,
  onClose: () => void,
  enabled: () => boolean = () => true,
) {
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && enabled()) onClose();
  }

  watch(open, (isOpen) => {
    if (isOpen) window.addEventListener("keydown", handleKeydown);
    else window.removeEventListener("keydown", handleKeydown);
  });

  onBeforeUnmount(() => { window.removeEventListener("keydown", handleKeydown); });
}
