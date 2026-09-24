import { ref } from "vue";

/** Counts password changes, so the shell can send the admin back to sign in. */
export const passwordChanges = ref(0);

export function announcePasswordChange() {
  passwordChanges.value += 1;
}
