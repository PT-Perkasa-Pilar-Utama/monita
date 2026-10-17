// Response shapes of /api/ui. Each mirrors a `Serialize` struct in src/<module>/api.rs.

export type Project = {
  name: string;
  slug: string;
  owner_email: string;
  description: string | null;
  icon_url: string | null;
  ip_allowlist: string | null;
  created_at: number;
};

export type Owner = { email: string; role: string };

export type ProjectList = { projects: Project[]; owners: Owner[] };
