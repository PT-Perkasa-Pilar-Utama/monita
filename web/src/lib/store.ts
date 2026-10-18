// Per-browser conveniences in localStorage. Storage can be blocked or full; the page works
// without it, so a failure reads as nothing saved.

export function readLocal(key: string): string {
  try {
    return localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

export function writeLocal(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Not saved; the page still shows the value.
  }
}
