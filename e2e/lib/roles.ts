/** Seeded accounts (seeds/qa.sql). The password works only where reset-state exists. */
export const SEED_PASSWORD = "monita-dev-password";
export const AUTH_DIR = ".auth";
export const ROLES = { admin: "admin@monita.test", tl: "tl@monita.test", supervisor: "supervisor@monita.test" } as const;
export const authFile = (role: keyof typeof ROLES) => `${AUTH_DIR}/${role}.json`;
