import type { ReactElement, ReactNode } from "react";
import { Link, useLocation } from "react-router-dom";
import { Mark } from "@/components/brand/Mark";
import { ProjectBadge } from "@/components/app/ProjectBadge";
import { AlertTriangle, BookOpen, Folder, LogOut, Moon, Settings, ShieldCheck, Sun, Ticket, Undo2 } from "@/components/icons";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useTheme } from "@/components/layout/ThemeProvider";
import { api } from "@/lib/api";
import { lastAdminPage } from "@/lib/adminPage";
import { lastProject } from "@/lib/lastProject";
import { ROLE_LABEL, useMe } from "@/lib/session";
import { cn } from "@/lib/utils";

export const DOCS = "https://pt-perkasa-pilar-utama.github.io/monita/docs/";

/** An area link in the header. The open area is a raised pill.
 * Below 1024 px only the icon shows. */
function AreaLink({ to, label, icon, active, external, className }: { to: string; label: string; icon: ReactNode; active: boolean; external?: boolean; className?: string }): ReactElement {
  const body = (
    <>
      {icon}
      <span className="hidden lg:inline">{label}</span>
    </>
  );
  return (
    <Button asChild variant="ghost" size="sm" title={label} className={cn(active && "bg-background font-semibold text-foreground ring-1 ring-border hover:bg-background", className)}>
      {external ? (
        <a href={to} target="_blank" rel="noreferrer" aria-label={`${label}, opens in a new tab`}>
          {body}
        </a>
      ) : (
        <Link to={to} aria-current={active ? "page" : undefined} aria-label={label}>
          {body}
        </Link>
      )}
    </Button>
  );
}

/** The header's areas: Projects, Incidents and Tickets across your projects, Admin for a super
 * admin (it opens the admin page opened last), and the documentation. */
function AreaLinks(): ReactElement {
  const me = useMe();
  const { pathname } = useLocation();
  const at = (p: string): boolean => pathname === p || pathname.startsWith(`${p}/`);
  return (
    <>
      <AreaLink to="/" label="Projects" icon={<Folder />} active={pathname === "/" || at("/projects")} />
      <AreaLink to="/incidents" label="Incidents" icon={<AlertTriangle />} active={at("/incidents")} />
      <AreaLink to="/tickets" label="Tickets" icon={<Ticket />} active={at("/tickets")} />
      {/* On a phone these two move into the account menu, so the project name keeps its room. */}
      {me.sees_all && <AreaLink to={at("/admin") ? pathname : lastAdminPage()} label="Admin" icon={<Settings />} active={at("/admin")} className="max-sm:hidden" />}
      <AreaLink to={DOCS} label="Documentation" icon={<BookOpen />} active={false} external className="max-sm:hidden" />
    </>
  );
}

/** The account menu: who is signed in, the theme and sign out. */
function AccountMenu(): ReactElement {
  const me = useMe();
  const { pathname } = useLocation();
  const { resolved, setTheme } = useTheme();
  const initial = me.email.charAt(0).toUpperCase();
  const signOut = async (): Promise<void> => {
    await api("/logout", "POST");
    window.location.assign("/login");
  };
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label={`Account: ${me.email}`}
          title={me.email}
          className="rounded-full outline-none focus-visible:ring-[3px] focus-visible:ring-ring"
        >
          <Avatar className="size-8">
            <AvatarFallback className="bg-primary/15 font-semibold text-link">{initial}</AvatarFallback>
          </Avatar>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <DropdownMenuLabel className="flex flex-col gap-0.5 py-2 font-normal">
          <span className="break-all font-medium text-foreground">{me.email}</span>
          <span className="text-xs text-muted-foreground">{ROLE_LABEL[me.role]}</span>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          {me.sees_all && (
            <DropdownMenuItem asChild className="sm:hidden">
              <Link to={pathname.startsWith("/admin/") ? pathname : lastAdminPage()}>
                <Settings />
                Admin
              </Link>
            </DropdownMenuItem>
          )}
          <DropdownMenuItem asChild className="sm:hidden">
            <a href={DOCS} target="_blank" rel="noreferrer">
              <BookOpen />
              Documentation
            </a>
          </DropdownMenuItem>
          <DropdownMenuItem asChild>
            <Link to="/account">
              <ShieldCheck />
              Two-factor sign-in
            </Link>
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => setTheme(resolved === "dark" ? "light" : "dark")}>
            {resolved === "dark" ? <Sun /> : <Moon />}
            Toggle theme
          </DropdownMenuItem>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          <DropdownMenuItem variant="destructive" onSelect={() => void signOut()}>
            <LogOut />
            Sign out
          </DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/**
 * The sticky band over every signed-in page: the brand, the place (the open project, or the way
 * back to the last one), the area links and the account menu, then the area's tab bar.
 */
export function TopBar({ crumb, tabs }: { crumb?: ReactNode; tabs?: ReactNode }): ReactElement {
  const me = useMe();
  // Outside a project, the way back to the one opened last in this browser.
  const last = crumb ? null : lastProject(me.email);
  const place =
    crumb ??
    (last && (
      <Link
        to={`/projects/${last.slug}`}
        title={`Back to ${last.name}`}
        aria-label={`Back to ${last.name}`}
        className="flex min-w-0 items-center gap-2 rounded-lg px-2 py-1 font-semibold text-muted-foreground hover:bg-accent hover:text-foreground"
      >
        <Undo2 aria-hidden="true" className="size-4 shrink-0" />
        <ProjectBadge name={last.name} iconUrl={last.icon_url} />
        <span className="truncate">{last.name}</span>
      </Link>
    ));
  return (
    <header className="sticky top-0 z-40 border-b bg-card shadow-xs" data-has-tabs={tabs ? "" : undefined}>
      <div className="px-4 sm:px-8 lg:px-12">
        <div className="mx-auto flex h-14 max-w-app items-center gap-3">
          <Link to="/" className="flex shrink-0 items-center gap-2 text-xl font-semibold tracking-tight" aria-label="Monita, all projects">
            <Mark className="size-[26px] text-primary" />
            <span className="hidden sm:inline">Monita</span>
          </Link>
          {place && (
            <>
              <span aria-hidden="true" className="hidden text-xl font-light text-input sm:inline">
                /
              </span>
              {place}
            </>
          )}
          <nav aria-label="Areas" className="ml-auto flex items-center gap-1">
            <AreaLinks />
          </nav>
          <AccountMenu />
        </div>
      </div>
      {tabs}
    </header>
  );
}
