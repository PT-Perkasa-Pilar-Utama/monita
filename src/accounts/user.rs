//! Who makes a request: a user with a role, or a project API key (v1.1 Q44).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    SuperAdmin,
    Tl,
    /// Sees everything a super admin sees and changes nothing.
    Supervisor,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::SuperAdmin => "super_admin",
            Role::Tl => "tl",
            Role::Supervisor => "supervisor",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "super_admin" => Some(Role::SuperAdmin),
            "tl" => Some(Role::Tl),
            "supervisor" => Some(Role::Supervisor),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub role: Role,
    /// Set when a project API key made the request (v1.1 Q44). The key reaches its own project
    /// only, so the methods below never let it see another project or an admin page.
    pub key: Option<KeyGrant>,
}

/// What a project API key may do under `/api/v1/projects/{slug}/`.
#[derive(Clone, Debug)]
pub struct KeyGrant {
    pub project_id: i64,
    /// The key's name, which a status update it writes shows as its author.
    pub name: String,
    pub write: bool,
}

impl User {
    /// The principal of a project API key: no user behind it, so `id` is 0 and matches no row.
    pub fn for_key(grant: KeyGrant) -> Self {
        User {
            id: 0,
            email: String::new(),
            role: Role::Tl,
            key: Some(grant),
        }
    }

    /// May change anything, in every project and in the admin pages.
    pub fn is_admin(&self) -> bool {
        self.key.is_none() && self.role == Role::SuperAdmin
    }

    /// Sees every project and the admin pages: a super admin or a supervisor.
    pub fn sees_all(&self) -> bool {
        self.key.is_none() && matches!(self.role, Role::SuperAdmin | Role::Supervisor)
    }

    /// Reads only: a supervisor or a read key. `CurrentUser` refuses every request from them that
    /// would change something.
    pub fn is_read_only(&self) -> bool {
        match &self.key {
            Some(k) => !k.write,
            None => self.role == Role::Supervisor,
        }
    }
}
