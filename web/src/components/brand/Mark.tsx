import type { ReactElement } from "react";

/** Monita's mark: a pulse in a ring. Colour comes from the text colour. */
export function Mark({ className }: { className?: string }): ReactElement {
  return (
    <svg className={className} viewBox="0 0 20 20" fill="none" aria-hidden="true">
      <circle cx="10" cy="10" r="8.25" stroke="currentColor" strokeWidth="1.5" />
      <path
        d="M4.5 10.5h2.5l1.5-3.5 3 6 1.5-2.5h2.5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
