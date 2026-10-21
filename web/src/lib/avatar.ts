/** The user's own picture, versioned by when it changed, or null to show the initial. */
export function avatarUrl(avatarAt: number | null): string | null {
  return avatarAt === null ? null : `/me/avatar?v=${avatarAt}`;
}
