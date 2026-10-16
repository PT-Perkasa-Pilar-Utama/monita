/** One place for the test server's address and reset token. */
export const PORT = 18080;
export const BASE_URL = `http://127.0.0.1:${PORT}`;
// Local-only value. The route that reads it does not exist in prod (SEC-17).
export const RESET_TOKEN = process.env["MONITA_RESET_TOKEN"] ?? "e2e-local-reset-token";
