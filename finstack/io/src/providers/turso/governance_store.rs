//! GovernanceStore implementation for TursoStore.

use crate::governance::{
    ActorKind, ChangeKind, ResourceChange, ResourceChangeInsert, ResourceEntity, ResourceShare,
    SharePermission, ShareType, UserRole, VisibilityScope, WorkflowBinding, WorkflowState,
    WorkflowTransition,
};
use crate::store::GovernanceStore;
use crate::{Error, Result};
use async_trait::async_trait;
use libsql::params;

use super::store::{get_blob, get_optional_string, get_string, meta_json, TursoStore};

fn parse_share_row_turso(row: &libsql::Row) -> Result<ResourceShare> {
    let share_type = get_string(row, 2)?;
    let permission = get_string(row, 5)?;
    Ok(ResourceShare {
        resource_type: get_string(row, 0)?,
        resource_id: get_string(row, 1)?,
        share_type: parse_share_type(share_type)?,
        share_id: get_string(row, 3)?,
        share_scope_id: get_optional_string(row, 4)?,
        permission: parse_share_permission(permission)?,
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

fn table_name(store: &TursoStore, base: &str) -> String {
    store.naming().resolve(base)
}

#[async_trait]
impl GovernanceStore for TursoStore {
    async fn get_resource_entity(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<ResourceEntity>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "resource_entities");
        let sql = format!(
            "SELECT resource_type, resource_id, owner_user_id, visibility_scope, visibility_id \
             FROM {table} WHERE resource_type = ?1 AND resource_id = ?2"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![resource_type, resource_id]).await?;
        match rows.next().await.map_err(Error::from)? {
            Some(row) => {
                let scope = get_string(&row, 3)?;
                Ok(Some(ResourceEntity {
                    resource_type: get_string(&row, 0)?,
                    resource_id: get_string(&row, 1)?,
                    owner_user_id: get_string(&row, 2)?,
                    visibility_scope: parse_visibility(scope)?,
                    visibility_id: get_optional_string(&row, 4)?,
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_resource_entities(&self, resource_type: &str) -> Result<Vec<ResourceEntity>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "resource_entities");
        let sql = format!(
            "SELECT resource_type, resource_id, owner_user_id, visibility_scope, visibility_id \
             FROM {table} WHERE resource_type = ?1"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![resource_type]).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::from)? {
            let scope = get_string(&row, 3)?;
            out.push(ResourceEntity {
                resource_type: get_string(&row, 0)?,
                resource_id: get_string(&row, 1)?,
                owner_user_id: get_string(&row, 2)?,
                visibility_scope: parse_visibility(scope)?,
                visibility_id: get_optional_string(&row, 4)?,
            });
        }
        Ok(out)
    }

    async fn upsert_resource_entity(&self, entity: &ResourceEntity) -> Result<()> {
        let conn = self.get_conn()?;
        let table = table_name(self, "resource_entities");
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
                entity.resource_type.as_str(),
                entity.resource_id.as_str(),
                entity.owner_user_id.as_str(),
                entity.visibility_scope.as_str(),
                entity.visibility_id.as_deref()
            ],
        )
        .await?;
        Ok(())
    }

    async fn list_resource_shares(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Vec<ResourceShare>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "resource_shares");
        let sql = format!(
            "SELECT resource_type, resource_id, share_type, share_id, share_scope_id, permission \
             FROM {table} WHERE resource_type = ?1 AND resource_id = ?2"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![resource_type, resource_id]).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::from)? {
            out.push(parse_share_row_turso(&row)?);
        }
        Ok(out)
    }

    async fn list_all_resource_shares(
        &self,
        resource_type: &str,
    ) -> Result<std::collections::HashMap<String, Vec<ResourceShare>>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "resource_shares");
        let sql = format!(
            "SELECT resource_type, resource_id, share_type, share_id, share_scope_id, permission \
             FROM {table} WHERE resource_type = ?1"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![resource_type]).await?;
        let mut map: std::collections::HashMap<String, Vec<ResourceShare>> =
            std::collections::HashMap::new();
        while let Some(row) = rows.next().await.map_err(Error::from)? {
            let share = parse_share_row_turso(&row)?;
            map.entry(share.resource_id.clone())
                .or_default()
                .push(share);
        }
        Ok(map)
    }

    async fn list_user_roles(&self, user_id: &str) -> Result<Vec<UserRole>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "auth_user_roles");
        let sql = format!("SELECT role_id, group_id FROM {table} WHERE user_id = ?1");
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![user_id]).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::from)? {
            let group_id = get_string(&row, 1)?;
            out.push(UserRole {
                role_id: get_string(&row, 0)?,
                group_id: if group_id.is_empty() {
                    None
                } else {
                    Some(group_id)
                },
            });
        }
        Ok(out)
    }

    async fn list_user_groups(&self, user_id: &str) -> Result<Vec<String>> {
        let conn = self.get_conn()?;
        let groups_table = table_name(self, "auth_user_groups");
        let roles_table = table_name(self, "auth_user_roles");
        let sql_groups = format!("SELECT group_id FROM {groups_table} WHERE user_id = ?1");
        let sql_roles = format!(
            "SELECT DISTINCT group_id FROM {roles_table} WHERE user_id = ?1 AND group_id != ''"
        );
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();

        let mut stmt = conn.prepare(sql_groups.as_str()).await?;
        let mut rows = stmt.query(params![user_id]).await?;
        while let Some(row) = rows.next().await.map_err(Error::from)? {
            let value = get_string(&row, 0)?;
            if seen.insert(value.clone()) {
                out.push(value);
            }
        }

        let mut stmt = conn.prepare(sql_roles.as_str()).await?;
        let mut rows = stmt.query(params![user_id]).await?;
        while let Some(row) = rows.next().await.map_err(Error::from)? {
            let value = get_string(&row, 0)?;
            if seen.insert(value.clone()) {
                out.push(value);
            }
        }

        Ok(out)
    }

    async fn list_workflow_bindings(&self, resource_type: &str) -> Result<Vec<WorkflowBinding>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "workflow_bindings");
        let sql = format!(
            "SELECT id, resource_type, visibility_scope, visibility_id, change_kind, base_verified_source, policy_id, priority \
             FROM {table} WHERE resource_type = ?1"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![resource_type]).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::from)? {
            out.push(WorkflowBinding {
                id: get_string(&row, 0)?,
                resource_type: get_string(&row, 1)?,
                visibility_scope: get_optional_string(&row, 2)?,
                visibility_id: get_optional_string(&row, 3)?,
                change_kind: get_optional_string(&row, 4)?,
                base_verified_source: get_optional_string(&row, 5)?,
                policy_id: get_string(&row, 6)?,
                priority: row.get::<i64>(7).map_err(Error::from)? as i32,
            });
        }
        Ok(out)
    }

    async fn get_workflow_transition(
        &self,
        policy_id: &str,
        from_state: &str,
        to_state: &str,
    ) -> Result<Option<WorkflowTransition>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "workflow_transitions");
        let sql = format!(
            "SELECT id, policy_id, from_state, to_state, required_role_id, required_group_id, \
                allow_owner, allow_system_actor, require_verifier_not_owner, require_verifier_not_submitter, \
                require_distinct_from_last_actor \
             FROM {table} WHERE policy_id = ?1 AND from_state = ?2 AND to_state = ?3"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![policy_id, from_state, to_state]).await?;
        match rows.next().await.map_err(Error::from)? {
            Some(row) => Ok(Some(WorkflowTransition {
                id: get_string(&row, 0)?,
                policy_id: get_string(&row, 1)?,
                from_state: get_string(&row, 2)?,
                to_state: get_string(&row, 3)?,
                required_role_id: get_optional_string(&row, 4)?,
                required_group_id: get_optional_string(&row, 5)?,
                allow_owner: row.get::<bool>(6).map_err(Error::from)?,
                allow_system_actor: row.get::<bool>(7).map_err(Error::from)?,
                require_verifier_not_owner: row.get::<bool>(8).map_err(Error::from)?,
                require_verifier_not_submitter: row.get::<bool>(9).map_err(Error::from)?,
                require_distinct_from_last_actor: row.get::<bool>(10).map_err(Error::from)?,
            })),
            None => Ok(None),
        }
    }

    async fn get_workflow_state(
        &self,
        policy_id: &str,
        state_key: &str,
    ) -> Result<Option<WorkflowState>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "workflow_states");
        let sql = format!(
            "SELECT policy_id, state_key, is_final, verified_source, system_only, category \
             FROM {table} WHERE policy_id = ?1 AND state_key = ?2"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![policy_id, state_key]).await?;
        match rows.next().await.map_err(Error::from)? {
            Some(row) => Ok(Some(WorkflowState {
                policy_id: get_string(&row, 0)?,
                state_key: get_string(&row, 1)?,
                is_final: row.get::<bool>(2).map_err(Error::from)?,
                verified_source: get_optional_string(&row, 3)?,
                system_only: row.get::<bool>(4).map_err(Error::from)?,
                category: get_string(&row, 5)?,
            })),
            None => Ok(None),
        }
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
        let conn = self.get_conn()?;
        let table = table_name(self, "workflow_events");
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
                actor_kind.as_str(),
                actor_id,
                note
            ],
        )
        .await?;
        Ok(())
    }

    async fn last_workflow_event_actor(&self, change_id: &str) -> Result<Option<String>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "workflow_events");
        let sql = format!(
            "SELECT actor_id FROM {table} WHERE change_id = ?1 ORDER BY at_ts DESC LIMIT 1"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![change_id]).await?;
        match rows.next().await.map_err(Error::from)? {
            Some(row) => Ok(Some(get_string(&row, 0)?)),
            None => Ok(None),
        }
    }

    async fn insert_resource_change(&self, change: &ResourceChangeInsert) -> Result<()> {
        let conn = self.get_conn()?;
        let table = table_name(self, "resource_changes");
        let payload = serde_json::to_vec(&change.payload)?;
        let meta = meta_json(Some(&change.meta))?;
        let sql = format!(
            "INSERT INTO {table} (change_id, resource_type, resource_id, resource_key2, change_kind, workflow_policy_id, workflow_state, owner_user_id, created_by_kind, created_by_id, ingestion_source, ingestion_run_id, payload, meta) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
        );
        conn.execute(
            sql.as_str(),
            params![
                change.change_id.clone(),
                change.resource_type.clone(),
                change.resource_id.clone(),
                change.resource_key2.clone(),
                change.change_kind.as_str(),
                change.workflow_policy_id.clone(),
                change.workflow_state.clone(),
                change.owner_user_id.clone(),
                change.created_by_kind.as_str(),
                change.created_by_id.clone(),
                change.ingestion_source.clone(),
                change.ingestion_run_id.clone(),
                payload,
                meta
            ],
        )
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
        let conn = self.get_conn()?;
        let table = table_name(self, "resource_changes");
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
        )
        .await?;
        Ok(())
    }

    async fn get_resource_change(&self, change_id: &str) -> Result<Option<ResourceChange>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "resource_changes");
        let sql = format!(
            "SELECT change_id, resource_type, resource_id, resource_key2, change_kind, workflow_policy_id, workflow_state, owner_user_id, created_by_kind, created_by_id, submitted_at, applied_at, base_etag, ingestion_source, ingestion_run_id, payload, meta \
             FROM {table} WHERE change_id = ?1"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![change_id]).await?;
        match rows.next().await.map_err(Error::from)? {
            Some(row) => {
                let change_kind = get_string(&row, 4)?;
                let created_by_kind = get_string(&row, 8)?;
                let payload = get_blob(&row, 15)?;
                let meta = get_string(&row, 16)?;
                let payload_json = serde_json::from_slice(&payload)?;
                let meta_json = serde_json::from_str(&meta)?;
                Ok(Some(ResourceChange {
                    change_id: get_string(&row, 0)?,
                    resource_type: get_string(&row, 1)?,
                    resource_id: get_string(&row, 2)?,
                    resource_key2: get_string(&row, 3)?,
                    change_kind: parse_change_kind(change_kind)?,
                    workflow_policy_id: get_optional_string(&row, 5)?,
                    workflow_state: get_string(&row, 6)?,
                    owner_user_id: get_string(&row, 7)?,
                    created_by_kind: parse_actor_kind(created_by_kind)?,
                    created_by_id: get_string(&row, 9)?,
                    submitted_at: get_optional_string(&row, 10)?,
                    applied_at: get_optional_string(&row, 11)?,
                    base_etag: get_optional_string(&row, 12)?,
                    ingestion_source: get_optional_string(&row, 13)?,
                    ingestion_run_id: get_optional_string(&row, 14)?,
                    payload: payload_json,
                    meta: meta_json,
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_resource_changes_for_owner(
        &self,
        owner_user_id: &str,
    ) -> Result<Vec<ResourceChange>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "resource_changes");
        let sql = format!(
            "SELECT change_id, resource_type, resource_id, resource_key2, change_kind, workflow_policy_id, workflow_state, owner_user_id, created_by_kind, created_by_id, submitted_at, applied_at, base_etag, ingestion_source, ingestion_run_id, payload, meta \
             FROM {table} WHERE owner_user_id = ?1"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![owner_user_id]).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::from)? {
            let change_kind = get_string(&row, 4)?;
            let created_by_kind = get_string(&row, 8)?;
            let payload = get_blob(&row, 15)?;
            let meta = get_string(&row, 16)?;
            let payload_json = serde_json::from_slice(&payload)?;
            let meta_json = serde_json::from_str(&meta)?;
            out.push(ResourceChange {
                change_id: get_string(&row, 0)?,
                resource_type: get_string(&row, 1)?,
                resource_id: get_string(&row, 2)?,
                resource_key2: get_string(&row, 3)?,
                change_kind: parse_change_kind(change_kind)?,
                workflow_policy_id: get_optional_string(&row, 5)?,
                workflow_state: get_string(&row, 6)?,
                owner_user_id: get_string(&row, 7)?,
                created_by_kind: parse_actor_kind(created_by_kind)?,
                created_by_id: get_string(&row, 9)?,
                submitted_at: get_optional_string(&row, 10)?,
                applied_at: get_optional_string(&row, 11)?,
                base_etag: get_optional_string(&row, 12)?,
                ingestion_source: get_optional_string(&row, 13)?,
                ingestion_run_id: get_optional_string(&row, 14)?,
                payload: payload_json,
                meta: meta_json,
            });
        }
        Ok(out)
    }

    async fn latest_verified_state(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<String>> {
        let conn = self.get_conn()?;
        let table = table_name(self, "resource_changes");
        let sql = format!(
            "SELECT workflow_state FROM {table} \
             WHERE resource_type = ?1 AND resource_id = ?2 \
               AND workflow_state IN ('VERIFIED','SYSTEM_VERIFIED') \
             ORDER BY applied_at DESC, updated_at DESC LIMIT 1"
        );
        let mut stmt = conn.prepare(sql.as_str()).await?;
        let mut rows = stmt.query(params![resource_type, resource_id]).await?;
        match rows.next().await.map_err(Error::from)? {
            Some(row) => Ok(Some(get_string(&row, 0)?)),
            None => Ok(None),
        }
    }

    async fn apply_change_to_verified(&self, change: &ResourceChange) -> Result<()> {
        use crate::governance::{parse_series_resource_id, resource_types};
        use crate::sql::{statements, Backend};

        let conn = self.get_conn()?;
        let naming = self.naming().clone();
        let payload = serde_json::to_vec(&change.payload)?;
        let meta = meta_json(Some(&change.meta))?;
        match change.resource_type.as_str() {
            resource_types::INSTRUMENT => {
                let sql = statements::upsert_instrument_sql_with_naming(Backend::Sqlite, &naming);
                conn.execute(
                    sql.as_ref(),
                    params![change.resource_id.clone(), payload, meta],
                )
                .await?;
            }
            resource_types::MARKET_CONTEXT => {
                if change.resource_key2.is_empty() {
                    return Err(Error::Invariant(
                        "Missing as_of for market_context".to_string(),
                    ));
                }
                let sql =
                    statements::upsert_market_context_sql_with_naming(Backend::Sqlite, &naming);
                conn.execute(
                    sql.as_ref(),
                    params![
                        change.resource_id.clone(),
                        change.resource_key2.clone(),
                        payload,
                        meta
                    ],
                )
                .await?;
            }
            resource_types::PORTFOLIO => {
                if change.resource_key2.is_empty() {
                    return Err(Error::Invariant("Missing as_of for portfolio".to_string()));
                }
                let sql = statements::upsert_portfolio_sql_with_naming(Backend::Sqlite, &naming);
                conn.execute(
                    sql.as_ref(),
                    params![
                        change.resource_id.clone(),
                        change.resource_key2.clone(),
                        payload,
                        meta
                    ],
                )
                .await?;
            }
            resource_types::SCENARIO => {
                let sql = statements::upsert_scenario_sql_with_naming(Backend::Sqlite, &naming);
                conn.execute(
                    sql.as_ref(),
                    params![change.resource_id.clone(), payload, meta],
                )
                .await?;
            }
            resource_types::STATEMENT_MODEL => {
                let sql =
                    statements::upsert_statement_model_sql_with_naming(Backend::Sqlite, &naming);
                conn.execute(
                    sql.as_ref(),
                    params![change.resource_id.clone(), payload, meta],
                )
                .await?;
            }
            resource_types::METRIC_REGISTRY => {
                let sql =
                    statements::upsert_metric_registry_sql_with_naming(Backend::Sqlite, &naming);
                conn.execute(
                    sql.as_ref(),
                    params![change.resource_id.clone(), payload, meta],
                )
                .await?;
            }
            resource_types::SERIES_META => {
                let (namespace, kind, series_id) = parse_series_resource_id(&change.resource_id)?;
                let sql = statements::upsert_series_meta_sql_with_naming(Backend::Sqlite, &naming);
                conn.execute(
                    sql.as_ref(),
                    params![namespace, kind, series_id, change.meta.to_string()],
                )
                .await?;
            }
            _ => {
                return Err(Error::Invariant(format!(
                    "Unsupported resource_type: {}",
                    change.resource_type
                )));
            }
        }
        Ok(())
    }
}
