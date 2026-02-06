//! GovernanceStore implementation for SqliteStore.

use crate::governance::{
    ActorKind, ChangeKind, ResourceChange, ResourceChangeInsert, ResourceEntity, ResourceShare,
    SharePermission, ShareType, UserRole, VisibilityScope, WorkflowBinding, WorkflowState,
    WorkflowTransition,
};
use crate::store::GovernanceStore;
use crate::{Error, Result};
use async_trait::async_trait;
use rusqlite::params;

use super::store::{meta_json, optional_row, SqliteStore};

fn parse_share_row(row: &rusqlite::Row) -> rusqlite::Result<ResourceShare> {
    let share_type: String = row.get(2)?;
    let permission: String = row.get(5)?;
    Ok(ResourceShare {
        resource_type: row.get(0)?,
        resource_id: row.get(1)?,
        share_type: parse_share_type(share_type).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
        })?,
        share_id: row.get(3)?,
        share_scope_id: row.get(4)?,
        permission: parse_share_permission(permission).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
        })?,
    })
}

fn parse_visibility(value: String) -> Result<VisibilityScope> {
    VisibilityScope::parse(&value)
}

fn parse_share_type(value: String) -> Result<ShareType> {
    ShareType::parse(&value)
}

fn parse_share_permission(value: String) -> Result<SharePermission> {
    SharePermission::parse(&value)
}

fn parse_change_kind(value: String) -> Result<ChangeKind> {
    ChangeKind::parse(&value)
}

fn parse_actor_kind(value: String) -> Result<ActorKind> {
    ActorKind::parse(&value)
}

#[async_trait]
impl GovernanceStore for SqliteStore {
    async fn get_resource_entity(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<ResourceEntity>> {
        let resource_type = resource_type.to_string();
        let resource_id = resource_id.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<ResourceEntity>> {
                let table = naming.resolve("resource_entities");
                let sql = format!(
                    "SELECT resource_type, resource_id, owner_user_id, visibility_scope, visibility_id \
                     FROM {table} WHERE resource_type = ?1 AND resource_id = ?2"
                );
                Ok(optional_row(conn.query_row(
                    sql.as_str(),
                    params![resource_type, resource_id],
                    |row| {
                        let scope: String = row.get(3)?;
                        Ok(ResourceEntity {
                            resource_type: row.get(0)?,
                            resource_id: row.get(1)?,
                            owner_user_id: row.get(2)?,
                            visibility_scope: parse_visibility(scope)
                                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e)))?,
                            visibility_id: row.get(4)?,
                        })
                    },
                ))?)
            })
            .await
            .map_err(Into::into)
    }

    async fn list_resource_entities(&self, resource_type: &str) -> Result<Vec<ResourceEntity>> {
        let resource_type = resource_type.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Vec<ResourceEntity>> {
                let table = naming.resolve("resource_entities");
                let sql = format!(
                    "SELECT resource_type, resource_id, owner_user_id, visibility_scope, visibility_id \
                     FROM {table} WHERE resource_type = ?1"
                );
                let mut stmt = conn.prepare(sql.as_str())?;
                let rows = stmt.query_map(params![resource_type], |row| {
                    let scope: String = row.get(3)?;
                    Ok(ResourceEntity {
                        resource_type: row.get(0)?,
                        resource_id: row.get(1)?,
                        owner_user_id: row.get(2)?,
                        visibility_scope: parse_visibility(scope)
                            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e)))?,
                        visibility_id: row.get(4)?,
                    })
                })?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await
            .map_err(Into::into)
    }

    async fn upsert_resource_entity(&self, entity: &ResourceEntity) -> Result<()> {
        let entity = entity.clone();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let table = naming.resolve("resource_entities");
                let sql = format!(
                    "INSERT INTO {table} (resource_type, resource_id, owner_user_id, visibility_scope, visibility_id) \
                     VALUES (?1, ?2, ?3, ?4, ?5) \
                     ON CONFLICT(resource_type, resource_id) DO UPDATE SET \
                        owner_user_id = excluded.owner_user_id, \
                        visibility_scope = excluded.visibility_scope, \
                        visibility_id = excluded.visibility_id, \
                        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
                );
                conn.execute(
                    sql.as_str(),
                    params![
                        entity.resource_type,
                        entity.resource_id,
                        entity.owner_user_id,
                        entity.visibility_scope.as_str(),
                        entity.visibility_id
                    ],
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn list_resource_shares(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Vec<ResourceShare>> {
        let resource_type = resource_type.to_string();
        let resource_id = resource_id.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Vec<ResourceShare>> {
                let table = naming.resolve("resource_shares");
                let sql = format!(
                    "SELECT resource_type, resource_id, share_type, share_id, share_scope_id, permission \
                     FROM {table} WHERE resource_type = ?1 AND resource_id = ?2"
                );
                let mut stmt = conn.prepare(sql.as_str())?;
                let rows = stmt.query_map(params![resource_type, resource_id], |row| {
                    parse_share_row(row)
                })?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await
            .map_err(Into::into)
    }

    async fn list_all_resource_shares(
        &self,
        resource_type: &str,
    ) -> Result<std::collections::HashMap<String, Vec<ResourceShare>>> {
        let resource_type = resource_type.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(
                move |conn| -> tokio_rusqlite::Result<
                    std::collections::HashMap<String, Vec<ResourceShare>>,
                > {
                    let table = naming.resolve("resource_shares");
                    let sql = format!(
                        "SELECT resource_type, resource_id, share_type, share_id, share_scope_id, permission \
                         FROM {table} WHERE resource_type = ?1"
                    );
                    let mut stmt = conn.prepare(sql.as_str())?;
                    let rows = stmt.query_map(params![resource_type], parse_share_row)?;
                    let mut map: std::collections::HashMap<String, Vec<ResourceShare>> =
                        std::collections::HashMap::new();
                    for row in rows {
                        let share = row?;
                        map.entry(share.resource_id.clone())
                            .or_default()
                            .push(share);
                    }
                    Ok(map)
                },
            )
            .await
            .map_err(Into::into)
    }

    async fn list_user_roles(&self, user_id: &str) -> Result<Vec<UserRole>> {
        let user_id = user_id.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Vec<UserRole>> {
                let table = naming.resolve("auth_user_roles");
                let sql = format!("SELECT role_id, group_id FROM {table} WHERE user_id = ?1");
                let mut stmt = conn.prepare(sql.as_str())?;
                let rows = stmt.query_map(params![user_id], |row| {
                    let group_id: String = row.get(1)?;
                    Ok(UserRole {
                        role_id: row.get(0)?,
                        group_id: if group_id.is_empty() {
                            None
                        } else {
                            Some(group_id)
                        },
                    })
                })?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await
            .map_err(Into::into)
    }

    async fn list_user_groups(&self, user_id: &str) -> Result<Vec<String>> {
        let user_id = user_id.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Vec<String>> {
                let mut out = Vec::new();
                let mut seen = std::collections::HashSet::new();

                let groups_table = naming.resolve("auth_user_groups");
                let sql_groups =
                    format!("SELECT group_id FROM {groups_table} WHERE user_id = ?1");
                let mut stmt = conn.prepare(sql_groups.as_str())?;
                let rows = stmt.query_map(params![user_id.clone()], |row| row.get::<_, String>(0))?;
                for row in rows {
                    let value = row?;
                    if seen.insert(value.clone()) {
                        out.push(value);
                    }
                }

                let roles_table = naming.resolve("auth_user_roles");
                let sql_roles = format!(
                    "SELECT DISTINCT group_id FROM {roles_table} WHERE user_id = ?1 AND group_id != ''"
                );
                let mut stmt = conn.prepare(sql_roles.as_str())?;
                let rows = stmt.query_map(params![user_id], |row| row.get::<_, String>(0))?;
                for row in rows {
                    let value = row?;
                    if seen.insert(value.clone()) {
                        out.push(value);
                    }
                }
                Ok(out)
            })
            .await
            .map_err(Into::into)
    }

    async fn list_workflow_bindings(&self, resource_type: &str) -> Result<Vec<WorkflowBinding>> {
        let resource_type = resource_type.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Vec<WorkflowBinding>> {
                let table = naming.resolve("workflow_bindings");
                let sql = format!(
                    "SELECT id, resource_type, visibility_scope, visibility_id, change_kind, base_verified_source, policy_id, priority \
                     FROM {table} WHERE resource_type = ?1"
                );
                let mut stmt = conn.prepare(sql.as_str())?;
                let rows = stmt.query_map(params![resource_type], |row| {
                    Ok(WorkflowBinding {
                        id: row.get(0)?,
                        resource_type: row.get(1)?,
                        visibility_scope: row.get(2)?,
                        visibility_id: row.get(3)?,
                        change_kind: row.get(4)?,
                        base_verified_source: row.get(5)?,
                        policy_id: row.get(6)?,
                        priority: row.get(7)?,
                    })
                })?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await
            .map_err(Into::into)
    }

    async fn get_workflow_transition(
        &self,
        policy_id: &str,
        from_state: &str,
        to_state: &str,
    ) -> Result<Option<WorkflowTransition>> {
        let policy_id = policy_id.to_string();
        let from_state = from_state.to_string();
        let to_state = to_state.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<WorkflowTransition>> {
                let table = naming.resolve("workflow_transitions");
                let sql = format!(
                    "SELECT id, policy_id, from_state, to_state, required_role_id, required_group_id, \
                        allow_owner, allow_system_actor, require_verifier_not_owner, require_verifier_not_submitter, \
                        require_distinct_from_last_actor \
                     FROM {table} WHERE policy_id = ?1 AND from_state = ?2 AND to_state = ?3"
                );
                Ok(optional_row(conn.query_row(
                    sql.as_str(),
                    params![policy_id, from_state, to_state],
                    |row| {
                        Ok(WorkflowTransition {
                            id: row.get(0)?,
                            policy_id: row.get(1)?,
                            from_state: row.get(2)?,
                            to_state: row.get(3)?,
                            required_role_id: row.get(4)?,
                            required_group_id: row.get(5)?,
                            allow_owner: row.get(6)?,
                            allow_system_actor: row.get(7)?,
                            require_verifier_not_owner: row.get(8)?,
                            require_verifier_not_submitter: row.get(9)?,
                            require_distinct_from_last_actor: row.get(10)?,
                        })
                    },
                ))?)
            })
            .await
            .map_err(Into::into)
    }

    async fn get_workflow_state(
        &self,
        policy_id: &str,
        state_key: &str,
    ) -> Result<Option<WorkflowState>> {
        let policy_id = policy_id.to_string();
        let state_key = state_key.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(
                move |conn| -> tokio_rusqlite::Result<Option<WorkflowState>> {
                    let table = naming.resolve("workflow_states");
                    let sql = format!(
                    "SELECT policy_id, state_key, is_final, verified_source, system_only, category \
                     FROM {table} WHERE policy_id = ?1 AND state_key = ?2"
                );
                    Ok(optional_row(conn.query_row(
                        sql.as_str(),
                        params![policy_id, state_key],
                        |row| {
                            Ok(WorkflowState {
                                policy_id: row.get(0)?,
                                state_key: row.get(1)?,
                                is_final: row.get(2)?,
                                verified_source: row.get(3)?,
                                system_only: row.get(4)?,
                                category: row.get(5)?,
                            })
                        },
                    ))?)
                },
            )
            .await
            .map_err(Into::into)
    }

    async fn insert_workflow_event(
        &self,
        event_id: &str,
        change_id: &str,
        resource_type: &str,
        resource_id: &str,
        resource_key2: &str,
        from_state: &str,
        to_state: &str,
        actor_kind: ActorKind,
        actor_id: &str,
        note: Option<&str>,
    ) -> Result<()> {
        let event_id = event_id.to_string();
        let change_id = change_id.to_string();
        let resource_type = resource_type.to_string();
        let resource_id = resource_id.to_string();
        let resource_key2 = resource_key2.to_string();
        let from_state = from_state.to_string();
        let to_state = to_state.to_string();
        let actor_kind = actor_kind.as_str().to_string();
        let actor_id = actor_id.to_string();
        let note = note.map(|value| value.to_string());
        let naming = self.naming().clone();

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let table = naming.resolve("workflow_events");
                let sql = format!(
                    "INSERT INTO {table} (id, change_id, resource_type, resource_id, resource_key2, from_state, to_state, actor_kind, actor_id, note) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
                );
                conn.execute(
                    sql.as_str(),
                    params![
                        event_id,
                        change_id,
                        resource_type,
                        resource_id,
                        resource_key2,
                        from_state,
                        to_state,
                        actor_kind,
                        actor_id,
                        note
                    ],
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn last_workflow_event_actor(&self, change_id: &str) -> Result<Option<String>> {
        let change_id = change_id.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<String>> {
                let table = naming.resolve("workflow_events");
                let sql = format!(
                    "SELECT actor_id FROM {table} WHERE change_id = ?1 ORDER BY at_ts DESC LIMIT 1"
                );
                Ok(optional_row(conn.query_row(
                    sql.as_str(),
                    params![change_id],
                    |row| row.get(0),
                ))?)
            })
            .await
            .map_err(Into::into)
    }

    async fn insert_resource_change(&self, change: &ResourceChangeInsert) -> Result<()> {
        let change_id = change.change_id.clone();
        let resource_type = change.resource_type.clone();
        let resource_id = change.resource_id.clone();
        let resource_key2 = change.resource_key2.clone();
        let change_kind = change.change_kind.as_str().to_string();
        let workflow_policy_id = change.workflow_policy_id.clone();
        let workflow_state = change.workflow_state.clone();
        let owner_user_id = change.owner_user_id.clone();
        let created_by_kind = change.created_by_kind.as_str().to_string();
        let created_by_id = change.created_by_id.clone();
        let ingestion_source = change.ingestion_source.clone();
        let ingestion_run_id = change.ingestion_run_id.clone();
        let payload = serde_json::to_vec(&change.payload).map_err(Error::from)?;
        let meta = meta_json(Some(&change.meta))?;
        let naming = self.naming().clone();

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let table = naming.resolve("resource_changes");
                let sql = format!(
                    "INSERT INTO {table} (change_id, resource_type, resource_id, resource_key2, change_kind, workflow_policy_id, workflow_state, owner_user_id, created_by_kind, created_by_id, ingestion_source, ingestion_run_id, payload, meta) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
                );
                conn.execute(
                    sql.as_str(),
                    params![
                        change_id,
                        resource_type,
                        resource_id,
                        resource_key2,
                        change_kind,
                        workflow_policy_id,
                        workflow_state,
                        owner_user_id,
                        created_by_kind,
                        created_by_id,
                        ingestion_source,
                        ingestion_run_id,
                        payload,
                        meta
                    ],
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn update_resource_change_state(
        &self,
        change_id: &str,
        workflow_state: &str,
        workflow_policy_id: Option<&str>,
        submitted_at: Option<&str>,
        applied_at: Option<&str>,
    ) -> Result<()> {
        let change_id = change_id.to_string();
        let workflow_state = workflow_state.to_string();
        let workflow_policy_id = workflow_policy_id.map(|value| value.to_string());
        let submitted_at = submitted_at.map(|value| value.to_string());
        let applied_at = applied_at.map(|value| value.to_string());
        let naming = self.naming().clone();

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let table = naming.resolve("resource_changes");
                let sql = format!(
                    "UPDATE {table} SET workflow_state = ?2, \
                        workflow_policy_id = COALESCE(?3, workflow_policy_id), \
                        submitted_at = COALESCE(?4, submitted_at), \
                        applied_at = COALESCE(?5, applied_at), \
                        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') \
                     WHERE change_id = ?1"
                );
                conn.execute(
                    sql.as_str(),
                    params![
                        change_id,
                        workflow_state,
                        workflow_policy_id,
                        submitted_at,
                        applied_at
                    ],
                )?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn get_resource_change(&self, change_id: &str) -> Result<Option<ResourceChange>> {
        let change_id = change_id.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<ResourceChange>> {
                let table = naming.resolve("resource_changes");
                let sql = format!(
                    "SELECT change_id, resource_type, resource_id, resource_key2, change_kind, workflow_policy_id, workflow_state, owner_user_id, created_by_kind, created_by_id, submitted_at, applied_at, base_etag, ingestion_source, ingestion_run_id, payload, meta \
                     FROM {table} WHERE change_id = ?1"
                );
                Ok(optional_row(conn.query_row(sql.as_str(), params![change_id], |row| {
                    let change_kind: String = row.get(4)?;
                    let created_by_kind: String = row.get(8)?;
                    let payload: Vec<u8> = row.get(15)?;
                    let meta: String = row.get(16)?;
                    let payload_json = serde_json::from_slice(&payload)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(15, rusqlite::types::Type::Blob, Box::new(e)))?;
                    let meta_json = serde_json::from_str(&meta)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(16, rusqlite::types::Type::Text, Box::new(e)))?;
                    Ok(ResourceChange {
                        change_id: row.get(0)?,
                        resource_type: row.get(1)?,
                        resource_id: row.get(2)?,
                        resource_key2: row.get(3)?,
                        change_kind: parse_change_kind(change_kind)
                            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?,
                        workflow_policy_id: row.get(5)?,
                        workflow_state: row.get(6)?,
                        owner_user_id: row.get(7)?,
                        created_by_kind: parse_actor_kind(created_by_kind)
                            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(e)))?,
                        created_by_id: row.get(9)?,
                        submitted_at: row.get(10)?,
                        applied_at: row.get(11)?,
                        base_etag: row.get(12)?,
                        ingestion_source: row.get(13)?,
                        ingestion_run_id: row.get(14)?,
                        payload: payload_json,
                        meta: meta_json,
                    })
                }))?)
            })
            .await
            .map_err(Into::into)
    }

    async fn list_resource_changes_for_owner(
        &self,
        owner_user_id: &str,
    ) -> Result<Vec<ResourceChange>> {
        let owner_user_id = owner_user_id.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Vec<ResourceChange>> {
                let table = naming.resolve("resource_changes");
                let sql = format!(
                    "SELECT change_id, resource_type, resource_id, resource_key2, change_kind, workflow_policy_id, workflow_state, owner_user_id, created_by_kind, created_by_id, submitted_at, applied_at, base_etag, ingestion_source, ingestion_run_id, payload, meta \
                     FROM {table} WHERE owner_user_id = ?1"
                );
                let mut stmt = conn.prepare(sql.as_str())?;
                let rows = stmt.query_map(params![owner_user_id], |row| {
                    let change_kind: String = row.get(4)?;
                    let created_by_kind: String = row.get(8)?;
                    let payload: Vec<u8> = row.get(15)?;
                    let meta: String = row.get(16)?;
                    let payload_json = serde_json::from_slice(&payload)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(15, rusqlite::types::Type::Blob, Box::new(e)))?;
                    let meta_json = serde_json::from_str(&meta)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(16, rusqlite::types::Type::Text, Box::new(e)))?;
                    Ok(ResourceChange {
                        change_id: row.get(0)?,
                        resource_type: row.get(1)?,
                        resource_id: row.get(2)?,
                        resource_key2: row.get(3)?,
                        change_kind: parse_change_kind(change_kind)
                            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?,
                        workflow_policy_id: row.get(5)?,
                        workflow_state: row.get(6)?,
                        owner_user_id: row.get(7)?,
                        created_by_kind: parse_actor_kind(created_by_kind)
                            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(e)))?,
                        created_by_id: row.get(9)?,
                        submitted_at: row.get(10)?,
                        applied_at: row.get(11)?,
                        base_etag: row.get(12)?,
                        ingestion_source: row.get(13)?,
                        ingestion_run_id: row.get(14)?,
                        payload: payload_json,
                        meta: meta_json,
                    })
                })?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await
            .map_err(Into::into)
    }

    async fn latest_verified_state(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<String>> {
        let resource_type = resource_type.to_string();
        let resource_id = resource_id.to_string();
        let naming = self.naming().clone();
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<String>> {
                let table = naming.resolve("resource_changes");
                let sql = format!(
                    "SELECT workflow_state FROM {table} \
                     WHERE resource_type = ?1 AND resource_id = ?2 \
                       AND workflow_state IN ('VERIFIED','SYSTEM_VERIFIED') \
                     ORDER BY applied_at DESC, updated_at DESC LIMIT 1"
                );
                Ok(optional_row(conn.query_row(
                    sql.as_str(),
                    params![resource_type, resource_id],
                    |row| row.get(0),
                ))?)
            })
            .await
            .map_err(Into::into)
    }

    async fn apply_change_to_verified(&self, change: &ResourceChange) -> Result<()> {
        use crate::governance::{parse_series_resource_id, resource_types};
        use crate::sql::{statements, Backend};

        let naming = self.naming().clone();
        let change = change.clone();
        let payload = serde_json::to_vec(&change.payload)?;
        let meta = meta_json(Some(&change.meta))?;
        let series_parts = if change.resource_type == resource_types::SERIES_META {
            Some(parse_series_resource_id(&change.resource_id)?)
        } else {
            None
        };
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                match change.resource_type.as_str() {
                    resource_types::INSTRUMENT => {
                        let sql =
                            statements::upsert_instrument_sql_with_naming(Backend::Sqlite, &naming);
                        conn.execute(sql.as_ref(), params![change.resource_id, payload, meta])?;
                    }
                    resource_types::MARKET_CONTEXT => {
                        if change.resource_key2.is_empty() {
                            return Err(tokio_rusqlite::Error::Other(Box::new(Error::Invariant(
                                "Missing as_of for market_context".to_string(),
                            ))));
                        }
                        let sql = statements::upsert_market_context_sql_with_naming(
                            Backend::Sqlite,
                            &naming,
                        );
                        conn.execute(
                            sql.as_ref(),
                            params![change.resource_id, change.resource_key2, payload, meta],
                        )?;
                    }
                    resource_types::PORTFOLIO => {
                        if change.resource_key2.is_empty() {
                            return Err(tokio_rusqlite::Error::Other(Box::new(Error::Invariant(
                                "Missing as_of for portfolio".to_string(),
                            ))));
                        }
                        let sql =
                            statements::upsert_portfolio_sql_with_naming(Backend::Sqlite, &naming);
                        conn.execute(
                            sql.as_ref(),
                            params![change.resource_id, change.resource_key2, payload, meta],
                        )?;
                    }
                    resource_types::SCENARIO => {
                        let sql =
                            statements::upsert_scenario_sql_with_naming(Backend::Sqlite, &naming);
                        conn.execute(sql.as_ref(), params![change.resource_id, payload, meta])?;
                    }
                    resource_types::STATEMENT_MODEL => {
                        let sql = statements::upsert_statement_model_sql_with_naming(
                            Backend::Sqlite,
                            &naming,
                        );
                        conn.execute(sql.as_ref(), params![change.resource_id, payload, meta])?;
                    }
                    resource_types::METRIC_REGISTRY => {
                        let sql = statements::upsert_metric_registry_sql_with_naming(
                            Backend::Sqlite,
                            &naming,
                        );
                        conn.execute(sql.as_ref(), params![change.resource_id, payload, meta])?;
                    }
                    resource_types::SERIES_META => {
                        let (namespace, kind, series_id) =
                            series_parts.clone().ok_or_else(|| {
                                tokio_rusqlite::Error::from(rusqlite::Error::InvalidQuery)
                            })?;
                        let sql = statements::upsert_series_meta_sql_with_naming(
                            Backend::Sqlite,
                            &naming,
                        );
                        conn.execute(
                            sql.as_ref(),
                            params![namespace, kind, series_id, change.meta.to_string()],
                        )?;
                    }
                    _ => {
                        return Err(tokio_rusqlite::Error::Other(Box::new(Error::Invariant(
                            format!("Unsupported resource_type: {}", change.resource_type),
                        ))));
                    }
                }

                Ok(())
            })
            .await?;
        Ok(())
    }
}
