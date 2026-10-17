import type { ReactElement, ReactNode } from "react";
import { Link, useLocation } from "react-router-dom";
import { Mark } from "@/components/brand/Mark";
import { ChevronDown, Folder, House, LogOut, Moon, Server, Sun, Users } from "@/components/icons";
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
      className={cn("rounded-lg", active && "bg-card font-semibold text-foreground shadow-xs ring-1 ring-border hover:bg-card")}
    >
      <Link to={to} aria-current={active ? "true" : undefined}>
        {icon}
        <span className="hidden md:inline">{label}</span>
      </Link>
    </Button>
  );
}

function AdminMenu({ active }: { active: boolean }): ReactElement {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className={cn("rounded-lg", active && "bg-card font-semibold text-foreground shadow-xs ring-1 ring-border hover:bg-card")}
          aria-current={active ? "true" : undefined}
        >
          <Users />
          <span className="hidden md:inline">Admin</span>
          <ChevronDown />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48">
        <DropdownMenuGroup>
          <DropdownMenuItem asChild>
            <Link to="/admin/users">
              <Users />
              Users
            </Link>
          </DropdownMenuItem>
          <DropdownMenuItem asChild>
            <Link to="/admin/settings">
              <Server />
              System
            </Link>
          </DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** The account menu: who is signed in, the theme switch and sign out, two clicks away. */
function AccountMenu(): ReactElement {
  const me = useMe();
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
      <DropdownMenuContent align="end" className="w-72">
        <DropdownMenuLabel className="flex items-center gap-3 py-2 font-normal">
          <Avatar className="size-8">
            <AvatarFallback className="bg-primary/15 font-semibold text-link">{initial}</AvatarFallback>
          </Avatar>
          <span className="flex min-w-0 flex-col">
            <span className="break-all font-medium text-foreground">{me.email}</span>
            <span className="text-xs text-muted-foreground">{me.is_admin ? "Super admin" : "Tech lead"}</span>
          </span>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          <DropdownMenuItem asChild>
            <Link to="/">
              <House />
              All projects
            </Link>
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => setTheme(resolved === "dark" ? "light" : "dark")}>
            {resolved === "dark" ? <Sun /> : <Moon />}
            Switch to the {resolved === "dark" ? "light" : "dark"} theme
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
 * The sticky band over every signed-in page: the brand, the place (a project or Admin), the area
 * links and the account menu, then the area's tab bar when it has one.
 */
export function TopBar({ crumb, tabs }: { crumb?: ReactNode; tabs?: ReactNode }): ReactElement {
  const me = useMe();
  const { pathname } = useLocation();
  const admin = pathname.startsWith("/admin");
  return (
    <header className="sticky top-0 z-40 border-b bg-background" data-has-tabs={tabs ? "" : undefined}>
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
            <AreaLink to="/" label="Projects" icon={<Folder />} active={!admin} />
            {me.is_admin && <AdminMenu active={admin} />}
            <Button asChild variant="ghost" size="sm" className="hidden rounded-lg sm:inline-flex">
              <a href={DOCS}>Docs</a>
            </Button>
            <AccountMenu />
          </nav>
        </div>
      </div>
      {tabs}
    </header>
  );
}
