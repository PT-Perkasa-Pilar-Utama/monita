/** The path the app is served under: empty when Monita serves it, `/monita/demo` in the online
 * demo. web/build.ts sets it at build time from MONITA_BASE. */
export const BASE: string = process.env["MONITA_BASE"] ?? "";
