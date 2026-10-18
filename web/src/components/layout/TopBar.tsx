import type { ReactElement, ReactNode } from "react";
import { Link, useLocation } from "react-router-dom";
import { Mark } from "@/components/brand/Mark";
import { BookOpen, Folder, LogOut, Moon, Server, Sun, Users } from "@/components/icons";
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
import { useMe } from "@/lib/session";
import { cn } from "@/lib/utils";

const DOCS = "https://pt-perkasa-pilar-utama.github.io/monita/docs/";

/** An area link at the top bar's right end. The open area is a raised pill. */
function AreaLink({ to, label, icon, active }: { to: string; label: string; icon: ReactNode; active: boolean }): ReactElement {
  return (
    <Button
      asChild
      variant="ghost"
      size="sm"
      title={label}
      className={cn(active && "bg-background font-semibold text-foreground ring-1 ring-border hover:bg-background")}
    >
      <Link to={to} aria-current={active ? "page" : undefined}>
        {icon}
        <span className="hidden md:inline">{label}</span>
      </Link>
    </Button>
  );
}

/** One click between the light and the dark theme. */
function ThemeButton(): ReactElement {
  const { resolved, setTheme } = useTheme();
  const next = resolved === "dark" ? "light" : "dark";
  return (
    <Button variant="ghost" size="icon-sm" aria-label={`Switch to the ${next} theme`} title={`Switch to the ${next} theme`} onClick={() => setTheme(next)}>
      {resolved === "dark" ? <Sun /> : <Moon />}
    </Button>
  );
}

/** The account menu: who is signed in, and sign out. */
function AccountMenu(): ReactElement {
  const me = useMe();
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
          <span className="text-xs text-muted-foreground">{me.is_admin ? "Super admin" : "Tech lead"}</span>
        </DropdownMenuLabel>
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
 * The sticky band over every signed-in page: the brand, the place (a project or Admin), the area
 * links and the account menu, then the area's tab bar when it has one.
 */
export function TopBar({ crumb, tabs }: { crumb?: ReactNode; tabs?: ReactNode }): ReactElement {
  const me = useMe();
  const { pathname } = useLocation();
  const at = (prefix: string): boolean => pathname.startsWith(prefix);
  return (
    <header className="sticky top-0 z-40 border-b bg-card shadow-xs" data-has-tabs={tabs ? "" : undefined}>
      <div className="px-4 sm:px-8 lg:px-12">
        <div className="mx-auto flex h-14 max-w-app items-center gap-3">
          <Link to="/" className="flex shrink-0 items-center gap-2 text-xl font-semibold tracking-tight" aria-label="Monita, all projects">
            <Mark className="size-[26px] text-primary" />
            <span className="hidden sm:inline">Monita</span>
          </Link>
          {crumb && (
            <>
              <span aria-hidden="true" className="hidden text-xl font-light text-input sm:inline">
                /
              </span>
              {crumb}
            </>
          )}
          <nav aria-label="Account" className="ml-auto flex items-center gap-1">
            <AreaLink to="/" label="Projects" icon={<Folder />} active={!at("/admin")} />
            {me.is_admin && (
              <>
                <AreaLink to="/admin/users" label="Users" icon={<Users />} active={at("/admin/users")} />
                <AreaLink to="/admin/settings" label="System" icon={<Server />} active={at("/admin/settings")} />
              </>
            )}
            <Button asChild variant="ghost" size="sm" title="Docs">
              <a href={DOCS}>
                <BookOpen />
                <span className="hidden md:inline">Docs</span>
              </a>
            </Button>
            <ThemeButton />
            <AccountMenu />
          </nav>
        </div>
      </div>
      {tabs}
    </header>
  );
}
