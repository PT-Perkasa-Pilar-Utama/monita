import type { ReactElement, ReactNode } from "react";
import { Mark } from "@/components/brand/Mark";
import { ThemeToggle } from "@/components/layout/ThemeToggle";

/** Sign-in and invite: the brand, one centred card and a footer. `scene` runs full width along
 * the bottom and gives up its height first, so the page fits the screen without scrolling. Only a
 * screen too short for the card itself scrolls. */
export function BareLayout({ children, scene }: { children: ReactNode; scene?: ReactNode }): ReactElement {
  return (
    <div className="flex h-dvh flex-col overflow-y-auto">
      <main className="relative flex flex-[1_0_auto] flex-col items-center justify-center gap-6 px-4 py-8 [@media(max-height:640px)]:gap-4 [@media(max-height:640px)]:py-4">
        <ThemeToggle className="absolute top-4 right-4" />
        <div className="flex flex-col items-center gap-3 text-center">
          <Mark className="size-12 text-primary" />
          <p className="text-3xl font-bold tracking-tight">Monita</p>
          <p className="text-muted-foreground">Site reliability for every project your team ships.</p>
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
      {scene && <div className="min-h-0 flex-[0_1_240px] [@media(max-height:680px)]:hidden">{scene}</div>}
    </div>
  );
}
