import { readLocal, writeLocal } from "@/lib/store";

/** Per user, so a second account in the same browser still sees its own note. */
const key = (email: string): string => `monita.2fa-note-closed:${email}`;

/** The note shows while two-factor is off and this browser has not closed it for this user. */
export function showsNote(twoFactorOn: boolean, closed: string): boolean {
  return !twoFactorOn && closed !== "1";
}

export function noteClosed(email: string): string {
  return readLocal(key(email));
}

export function closeNote(email: string): void {
  writeLocal(key(email), "1");
}
