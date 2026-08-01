import { ref } from "vue";

export type NotificationKind = "success" | "warning" | "error";

export interface NotificationItem {
  id: number;
  kind: NotificationKind;
  message: string;
}

export const notificationItems = ref<NotificationItem[]>([]);
let nextId = 1;

export function notify(kind: NotificationKind, message: string) {
  const item = { id: nextId, kind, message };
  nextId += 1;
  notificationItems.value = [...notificationItems.value.slice(-2), item];
  window.setTimeout(() => {
    dismissNotification(item.id);
  }, 5_000);
}

export function dismissNotification(id: number) {
  notificationItems.value = notificationItems.value.filter((item) => item.id !== id);
}
