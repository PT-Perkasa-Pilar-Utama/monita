import { useId, useMemo, useState } from "react";
import type { KeyboardEvent, ReactElement } from "react";
import { Check } from "@/components/icons";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { matches } from "@/lib/filter";
import { cn } from "@/lib/utils";

export type PickableUser = { email: string; role: string };

const roleLabel = (role: string): string => (role === "super_admin" ? "Super admin" : "Tech lead");

/**
 * Search people by email and pick one, the way Beaverflow adds a member: the field filters the
 * rows under it, and a click or Enter picks one. Typed text that matches no one stays as typed;
 * the server checks it again.
 */
export function UserPicker({
  label,
  users,
  value,
  onChange,
  error,
  required,
}: {
  label: string;
  users: PickableUser[];
  value: string;
  onChange: (email: string) => void;
  error?: string | undefined;
  required?: boolean;
}): ReactElement {
  const id = useId();
  const listId = `${id}-list`;
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState<string | null>(null);
  const [active, setActive] = useState(0);
  const shown = useMemo(() => users.filter((u) => matches(query ?? "", u.email)), [users, query]);

  const pick = (u: PickableUser): void => {
    onChange(u.email);
    setQuery(null);
    setOpen(false);
  };

  const onKey = (e: KeyboardEvent<HTMLInputElement>): void => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      setOpen(true);
      const step = e.key === "ArrowDown" ? 1 : -1;
      setActive((a) => (shown.length ? (a + step + shown.length) % shown.length : 0));
    } else if (e.key === "Enter" && open && shown[active]) {
      e.preventDefault();
      pick(shown[active]);
    } else if (e.key === "Escape" && open) {
      // Close the list, not the dialog around it.
      e.preventDefault();
      e.stopPropagation();
      setOpen(false);
    }
  };

  return (
    <Field data-invalid={error ? true : undefined}>
      <FieldLabel htmlFor={id} required={required}>
        {label}
      </FieldLabel>
      <Input
        id={id}
        role="combobox"
        aria-expanded={open}
        aria-controls={listId}
        aria-autocomplete="list"
        aria-required={required || undefined}
        aria-invalid={error ? true : undefined}
        aria-activedescendant={open && shown[active] ? `${id}-${active}` : undefined}
        autoComplete="off"
        spellCheck={false}
        placeholder="Search by email"
        value={query ?? value}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onChange={(e) => {
          setQuery(e.target.value);
          onChange(e.target.value);
          setActive(0);
          setOpen(true);
        }}
        onKeyDown={onKey}
      />
      {open && (
        <ul id={listId} role="listbox" aria-label={label} className="max-h-56 overflow-y-auto rounded-lg border bg-card py-1">
          {shown.length === 0 && <li className="px-3 py-2 text-sm text-muted-foreground">No one matches this email.</li>}
          {shown.map((u, i) => (
            <li
              key={u.email}
              id={`${id}-${i}`}
              role="option"
              aria-selected={u.email === value}
              onMouseDown={(e) => e.preventDefault()}
              onMouseEnter={() => setActive(i)}
              onClick={() => pick(u)}
              className={cn("flex cursor-pointer items-center gap-3 px-3 py-2", i === active && "bg-accent")}
            >
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="truncate text-sm font-medium">{u.email}</span>
                <span className="text-xs text-muted-foreground">{roleLabel(u.role)}</span>
              </span>
              {u.email === value && <Check className="size-4 text-primary" />}
            </li>
          ))}
        </ul>
      )}
      {error && <FieldError>{error}</FieldError>}
    </Field>
  );
}
