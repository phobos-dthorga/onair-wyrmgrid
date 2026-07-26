export const SENSITIVE_CONTENT_PATTERNS = Object.freeze([
  /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/,
  /\bgh[pousr]_[A-Za-z0-9]{20,}\b/,
  /\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}\b/,
  /\bBearer\s+[A-Za-z0-9._~+/=-]{16,}\b/i,
  /\b(?:api[_-]?key|password|secret|token)["']?\s*[:=]\s*["']?[^\s"']{12,}/i,
]);

export function containsSensitiveContent(value) {
  return SENSITIVE_CONTENT_PATTERNS.some((pattern) => pattern.test(value));
}

export function containsUnsafeControlContent(value) {
  return /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f\u2028\u2029\u202a-\u202e\u2066-\u2069]/u.test(
    value,
  );
}
