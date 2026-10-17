import type { ReactElement, ReactNode } from "react";
import { Mark } from "@/components/brand/Mark";
import { ThemeToggle } from "@/components/layout/ThemeToggle";

/** Sign-in and invite: the brand, one centred card and a footer. */
export function BareLayout({ children }: { children: ReactNode }): ReactElement {
  return (
    <main className="relative flex min-h-dvh flex-col items-center justify-center gap-8 px-4 py-12">
      <ThemeToggle className="absolute top-4 right-4" />
      <div className="flex flex-col items-center gap-3 text-center">
        <Mark className="size-12 text-primary" />
        <p className="text-3xl font-bold tracking-tight">Monita</p>
        <p className="text-muted-foreground">Uptime, incidents and status pages for your team.</p>
      </div>
      <div className="w-full max-w-[420px]">{children}</div>
      <footer className="flex flex-col items-center gap-2 text-sm text-muted-foreground">
        <span>MIT licensed. Made by PT. Perkasa Pilar Utama.</span>
        <span className="flex gap-4">
          <a className="underline-offset-4 hover:text-foreground hover:underline" href="https://github.com/PT-Perkasa-Pilar-Utama/monita">
            GitHub
          </a>
          <a className="underline-offset-4 hover:text-foreground hover:underline" href="https://pt-perkasa-pilar-utama.github.io/monita/docs/">
            Documentation
          </a>
        </span>
      </footer>
    </main>
  );
}
