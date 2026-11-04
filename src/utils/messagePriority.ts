/**
 * Whether a message is important enough to be listed in the digest and to pause an advance: High
 * or Urgent. Copy of the backend's `MessagePriority::needs_attention` (domain, #1273), which the advance
 * pause reads; it goes when that rule is generated for TypeScript (#591). A message carries its priority as a plain string.
 */
export function needsAttention(priority: string): boolean {
  return priority === "High" || priority === "Urgent";
}
