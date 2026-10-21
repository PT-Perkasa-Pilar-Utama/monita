import type { ReactElement } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { avatarUrl } from "@/lib/avatar";
import { cn } from "@/lib/utils";

/** A user's picture, or the first letter of their email while they have none. */
export function UserAvatar({ id, email, avatarAt, className }: { id: number; email: string; avatarAt: number | null; className?: string }): ReactElement {
  const src = avatarUrl(id, avatarAt);
  return (
    <Avatar className={cn("size-8", className)}>
      {src && <AvatarImage src={src} alt="" />}
      <AvatarFallback className="bg-primary/15 font-semibold text-link">{email.charAt(0).toUpperCase()}</AvatarFallback>
    </Avatar>
  );
}
