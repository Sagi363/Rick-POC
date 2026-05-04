const KNOWN_LIMITS: Record<string, number> = {
  'claude-opus-4-7[1m]': 1_000_000,
  'claude-opus-4-7': 200_000,
  'claude-sonnet-4-6[1m]': 1_000_000,
  'claude-sonnet-4-6': 200_000,
  'claude-haiku-4-5': 200_000
}

export function modelLimit(modelId: string): { limit: number; known: boolean } {
  const direct = KNOWN_LIMITS[modelId]
  if (direct) return { limit: direct, known: true }
  // Strip date suffix (e.g. claude-haiku-4-5-20251001).
  const withoutDate = modelId.replace(/-\d{8}$/, '')
  const fallback = KNOWN_LIMITS[withoutDate]
  if (fallback) return { limit: fallback, known: true }
  return { limit: 200_000, known: false }
}
