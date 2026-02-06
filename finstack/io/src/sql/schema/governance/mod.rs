//! Governance, authentication, and workflow schema definitions.
//!
//! This module contains all schema definitions related to:
//! - Authentication (users, roles, groups)
//! - Resource management (entities, shares, changes)
//! - Workflow (states, transitions, bindings, events, policies)

mod auth_groups;
mod auth_roles;
mod auth_user_groups;
mod auth_user_roles;
mod auth_users;
mod resource_changes;
mod resource_entities;
mod resource_shares;
mod workflow_bindings;
mod workflow_events;
mod workflow_policies;
mod workflow_states;
mod workflow_transitions;

// Re-export all table enums
pub use auth_groups::AuthGroups;
pub use auth_roles::AuthRoles;
pub use auth_user_groups::AuthUserGroups;
pub use auth_user_roles::AuthUserRoles;
pub use auth_users::AuthUsers;
pub use resource_changes::ResourceChanges;
pub use resource_entities::ResourceEntities;
pub use resource_shares::ResourceShares;
pub use workflow_bindings::WorkflowBindings;
pub use workflow_events::WorkflowEvents;
pub use workflow_policies::WorkflowPolicies;
pub use workflow_states::WorkflowStates;
pub use workflow_transitions::WorkflowTransitions;
