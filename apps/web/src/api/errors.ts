import { ApiError } from "./client";

const ERROR_KEYS: Readonly<Record<string, string>> = {
  agent_unavailable: "errors.agentUnavailable",
  bad_request: "errors.badRequest",
  conflict: "errors.conflict",
  forbidden: "errors.forbidden",
  internal_error: "errors.internal",
  invalid_credentials: "errors.invalidCredentials",
  invalid_current_password: "errors.invalidCurrentPassword",
  invalid_new_password: "errors.invalidNewPassword",
  not_found: "errors.notFound",
  rate_limited: "errors.rateLimited",
  unavailable: "errors.unavailable",
};

export function apiErrorKey(error: unknown, fallbackKey: string): string {
  if (!(error instanceof ApiError)) return fallbackKey;
  if (error.code && ERROR_KEYS[error.code]) return ERROR_KEYS[error.code];
  if (error.status === 401) return "errors.unauthorized";
  return fallbackKey;
}
