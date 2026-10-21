/** A user's picture, versioned by when it changed, or null to show the initial. */
export function avatarUrl(userId: number, avatarAt: number | null): string | null {
  return avatarAt === null ? null : `/users/${userId}/avatar?v=${avatarAt}`;
}
