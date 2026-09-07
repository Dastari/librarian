//! GraphQL authentication helpers.

use async_graphql::{Context, ErrorExtensions, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Role {
    Admin,
    Member,
}

impl Role {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "admin" => Some(Self::Admin),
            "member" | "user" => Some(Self::Member),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user_id: String,
    pub email: Option<String>,
    pub role: Option<String>,
}

impl AuthUser {
    pub fn system_admin() -> Self {
        Self {
            user_id: "system".to_string(),
            email: None,
            role: Some(Role::Admin.as_str().to_string()),
        }
    }

    pub fn system_admin_for(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            email: None,
            role: Some(Role::Admin.as_str().to_string()),
        }
    }

    pub fn parsed_role(&self) -> Option<Role> {
        self.role.as_deref().and_then(Role::parse)
    }

    pub fn is_admin(&self) -> bool {
        self.parsed_role() == Some(Role::Admin)
    }

    pub fn has_role(&self, required: Role) -> bool {
        match required {
            Role::Admin => self.is_admin(),
            Role::Member => matches!(self.parsed_role(), Some(Role::Admin | Role::Member)),
        }
    }
}

impl From<agql_auth::AuthUser> for AuthUser {
    fn from(user: agql_auth::AuthUser) -> Self {
        let role = user.roles.first().cloned();
        Self {
            user_id: user.user_id,
            email: None,
            role,
        }
    }
}

pub fn verify_token(_token: &str, _jwt_secret: &str) -> Result<AuthUser> {
    Err(
        async_graphql::Error::new("direct JWT verification has been replaced by agql-auth")
            .extend_with(|_, e| e.set("code", "UNAUTHORIZED")),
    )
}

pub trait AuthExt {
    fn librarian_auth_user(&self) -> Result<&AuthUser>;
    fn try_auth_user(&self) -> Option<&AuthUser>;
    fn require_role(&self, role: Role) -> Result<&AuthUser>;
    fn require_admin(&self) -> Result<&AuthUser>;
    fn require_member(&self) -> Result<&AuthUser>;
}

impl<'a> AuthExt for Context<'a> {
    fn librarian_auth_user(&self) -> Result<&AuthUser> {
        self.data_opt::<AuthUser>().ok_or_else(|| {
            async_graphql::Error::new("Authentication required")
                .extend_with(|_, e| e.set("code", "UNAUTHORIZED"))
        })
    }

    fn try_auth_user(&self) -> Option<&AuthUser> {
        self.data_opt::<AuthUser>()
    }

    fn require_role(&self, role: Role) -> Result<&AuthUser> {
        let user = self.librarian_auth_user()?;
        if user.has_role(role) {
            Ok(user)
        } else {
            Err(
                async_graphql::Error::new(format!("Role '{}' required", role.as_str()))
                    .extend_with(|_, e| e.set("code", "FORBIDDEN")),
            )
        }
    }

    fn require_admin(&self) -> Result<&AuthUser> {
        self.require_role(Role::Admin)
    }

    fn require_member(&self) -> Result<&AuthUser> {
        self.require_role(Role::Member)
    }
}

pub struct AuthGuard;

impl async_graphql::Guard for AuthGuard {
    fn check(&self, ctx: &Context<'_>) -> impl std::future::Future<Output = Result<()>> + Send {
        let result = ctx.librarian_auth_user().map(|_| ());
        async move { result }
    }
}

pub struct RoleGuard {
    pub role: String,
}

impl RoleGuard {
    pub fn new(role: impl Into<String>) -> Self {
        Self { role: role.into() }
    }
}

impl async_graphql::Guard for RoleGuard {
    fn check(&self, ctx: &Context<'_>) -> impl std::future::Future<Output = Result<()>> + Send {
        let required = Role::parse(&self.role).unwrap_or(Role::Admin);
        let result = ctx.require_role(required).map(|_| ());
        async move { result }
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthUser, Role};

    #[test]
    fn roles_parse_case_insensitively() {
        assert_eq!(Role::parse("ADMIN"), Some(Role::Admin));
        assert_eq!(Role::parse(" member "), Some(Role::Member));
        assert_eq!(Role::parse("user"), Some(Role::Member));
        assert_eq!(Role::parse("owner"), None);
    }

    #[test]
    fn admin_satisfies_member_but_member_does_not_satisfy_admin() {
        let admin = AuthUser {
            user_id: "admin".to_string(),
            email: None,
            role: Some("Admin".to_string()),
        };
        let member = AuthUser {
            user_id: "member".to_string(),
            email: None,
            role: Some("member".to_string()),
        };
        let anonymous = AuthUser {
            user_id: "anonymous".to_string(),
            email: None,
            role: None,
        };

        assert!(admin.has_role(Role::Admin));
        assert!(admin.has_role(Role::Member));
        assert!(!member.has_role(Role::Admin));
        assert!(member.has_role(Role::Member));
        assert!(!anonymous.has_role(Role::Member));
    }

    #[test]
    fn system_contexts_are_admins() {
        assert!(AuthUser::system_admin().is_admin());
        assert!(AuthUser::system_admin_for("worker").has_role(Role::Admin));
    }
}
