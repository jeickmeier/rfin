//! GovernanceStore implementation for PostgresStore.

use crate::governance::{
    ActorKind, ChangeKind, ResourceChange, ResourceChangeInsert, ResourceEntity, ResourceShare,
    SharePermission, ShareType, UserRole, VisibilityScope, WorkflowBinding, WorkflowState,
    WorkflowTransition,
};
use crate::store::GovernanceStore;
use crate::{Error, Result};
use async_trait::async_trait;

use super::store::{quote_ident, PostgresStore};

fn parse_share_row_pg(row: &tokio_postgres::Row) -> Result<ResourceShare> {
    let share_type: String = row.get(2);
    let permission: String = row.get(5);
    Ok(ResourceShare {
        resource_type: row.get(0),
        resource_id: row.get(1),
        share_type: parse_share_type(share_type)?,
        share_id: row.get(3),
        share_scope_id: row.get(4),
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

fn table_name(store: &PostgresStore, base: &str) -> String {
    quote_ident(&store.naming().resolve(base))
}

#[async_trait]
impl GovernanceStore for PostgresStore {
    async fn get_resource_entity(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<ResourceEntity>> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "resource_entities");
        let sql = format!(
            "SELECT resource_type, resource_id, owner_user_id, visibility_scope, visibility_id \
             FROM {table} WHERE resource_type = $1 AND resource_id = $2"
        );
        let row = conn
            .query_opt(&sql, &[&resource_type, &resource_id])
            .await?;
        match row {
            Some(row) => {
                let scope: String = row.get(3);
                Ok(Some(ResourceEntity {
                    resource_type: row.get(0),
                    resource_id: row.get(1),
                    owner_user_id: row.get(2),
                    visibility_scope: parse_visibility(scope)?,
                    visibility_id: row.get(4),
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_resource_entities(&self, resource_type: &str) -> Result<Vec<ResourceEntity>> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "resource_entities");
        let sql = format!(
            "SELECT resource_type, resource_id, owner_user_id, visibility_scope, visibility_id \
             FROM {table} WHERE resource_type = $1"
        );
        let rows = conn.query(&sql, &[&resource_type]).await?;
        let mut out = Vec::new();
        for row in rows {
            let scope: String = row.get(3);
            out.push(ResourceEntity {
                resource_type: row.get(0),
                resource_id: row.get(1),
                owner_user_id: row.get(2),
                visibility_scope: parse_visibility(scope)?,
                visibility_id: row.get(4),
            });
        }
        Ok(out)
    }

    async fn upsert_resource_entity(&self, entity: &ResourceEntity) -> Result<()> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "resource_entities");
        let sql = format!(
            "INSERT INTO {table} (resource_type, resource_id, owner_user_id, visibility_scope, visibility_id) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (resource_type, resource_id) DO UPDATE SET \
                owner_user_id = EXCLUDED.owner_user_id, \
                visibility_scope = EXCLUDED.visibility_scope, \
                visibility_id = EXCLUDED.visibility_id, \
                updated_at = now()"
        );
        conn.execute(
            &sql,
            &[
                &entity.resource_type,
                &entity.resource_id,
                &entity.owner_user_id,
                &entity.visibility_scope.as_str(),
                &entity.visibility_id,
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
        let conn = self.get_conn().await?;
        let table = table_name(self, "resource_shares");
        let sql = format!(
            "SELECT resource_type, resource_id, share_type, share_id, share_scope_id, permission \
             FROM {table} WHERE resource_type = $1 AND resource_id = $2"
        );
        let rows = conn.query(&sql, &[&resource_type, &resource_id]).await?;
        let mut out = Vec::new();
        for row in rows {
            out.push(parse_share_row_pg(&row)?);
        }
        Ok(out)
    }

    async fn list_all_resource_shares(
        &self,
        resource_type: &str,
    ) -> Result<std::collections::HashMap<String, Vec<ResourceShare>>> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "resource_shares");
        let sql = format!(
            "SELECT resource_type, resource_id, share_type, share_id, share_scope_id, permission \
             FROM {table} WHERE resource_type = $1"
        );
        let rows = conn.query(&sql, &[&resource_type]).await?;
        let mut map: std::collections::HashMap<String, Vec<ResourceShare>> =
            std::collections::HashMap::new();
        for row in rows {
            let share = parse_share_row_pg(&row)?;
            map.entry(share.resource_id.clone())
                .or_default()
                .push(share);
        }
        Ok(map)
    }

    async fn list_user_roles(&self, user_id: &str) -> Result<Vec<UserRole>> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "auth_user_roles");
        let sql = format!("SELECT role_id, group_id FROM {table} WHERE user_id = $1");
        let rows = conn.query(&sql, &[&user_id]).await?;
        let mut out = Vec::new();
        for row in rows {
            let group_id: String = row.get(1);
            out.push(UserRole {
                role_id: row.get(0),
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
        let conn = self.get_conn().await?;
        let groups_table = table_name(self, "auth_user_groups");
        let roles_table = table_name(self, "auth_user_roles");
        let sql_groups = format!("SELECT group_id FROM {groups_table} WHERE user_id = $1");
        let sql_roles = format!(
            "SELECT DISTINCT group_id FROM {roles_table} WHERE user_id = $1 AND group_id != ''"
        );

        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for row in conn.query(&sql_groups, &[&user_id]).await? {
            let value: String = row.get(0);
            if seen.insert(value.clone()) {
                out.push(value);
            }
        }
        for row in conn.query(&sql_roles, &[&user_id]).await? {
            let value: String = row.get(0);
            if seen.insert(value.clone()) {
                out.push(value);
            }
        }
        Ok(out)
    }

    async fn list_workflow_bindings(&self, resource_type: &str) -> Result<Vec<WorkflowBinding>> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "workflow_bindings");
        let sql = format!(
            "SELECT id, resource_type, visibility_scope, visibility_id, change_kind, base_verified_source, policy_id, priority \
             FROM {table} WHERE resource_type = $1"
        );
        let rows = conn.query(&sql, &[&resource_type]).await?;
        let mut out = Vec::new();
        for row in rows {
            out.push(WorkflowBinding {
                id: row.get(0),
                resource_type: row.get(1),
                visibility_scope: row.get(2),
                visibility_id: row.get(3),
                change_kind: row.get(4),
                base_verified_source: row.get(5),
                policy_id: row.get(6),
                priority: row.get(7),
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
        let conn = self.get_conn().await?;
        let table = table_name(self, "workflow_transitions");
        let sql = format!(
            "SELECT id, policy_id, from_state, to_state, required_role_id, required_group_id, \
                allow_owner, allow_system_actor, require_verifier_not_owner, require_verifier_not_submitter, \
                require_distinct_from_last_actor \
             FROM {table} WHERE policy_id = $1 AND from_state = $2 AND to_state = $3"
        );
        let row = conn
            .query_opt(&sql, &[&policy_id, &from_state, &to_state])
            .await?;
        Ok(row.map(|row| WorkflowTransition {
            id: row.get(0),
            policy_id: row.get(1),
            from_state: row.get(2),
            to_state: row.get(3),
            required_role_id: row.get(4),
            required_group_id: row.get(5),
            allow_owner: row.get(6),
            allow_system_actor: row.get(7),
            require_verifier_not_owner: row.get(8),
            require_verifier_not_submitter: row.get(9),
            require_distinct_from_last_actor: row.get(10),
        }))
    }

    async fn get_workflow_state(
        &self,
        policy_id: &str,
        state_key: &str,
    ) -> Result<Option<WorkflowState>> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "workflow_states");
        let sql = format!(
            "SELECT policy_id, state_key, is_final, verified_source, system_only, category \
             FROM {table} WHERE policy_id = $1 AND state_key = $2"
        );
        let row = conn.query_opt(&sql, &[&policy_id, &state_key]).await?;
        Ok(row.map(|row| WorkflowState {
            policy_id: row.get(0),
            state_key: row.get(1),
            is_final: row.get(2),
            verified_source: row.get(3),
            system_only: row.get(4),
            category: row.get(5),
        }))
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
        let conn = self.get_conn().await?;
        let table = table_name(self, "workflow_events");
        let sql = format!(
            "INSERT INTO {table} (id, change_id, resource_type, resource_id, resource_key2, from_state, to_state, actor_kind, actor_id, note) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        );
        conn.execute(
            &sql,
            &[
                &event_id,
                &change_id,
                &resource_type,
                &resource_id,
                &resource_key2,
                &from_state,
                &to_state,
                &actor_kind.as_str(),
                &actor_id,
                &note,
            ],
        )
        .await?;
        Ok(())
    }

    async fn last_workflow_event_actor(&self, change_id: &str) -> Result<Option<String>> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "workflow_events");
        let sql = format!(
            "SELECT actor_id FROM {table} WHERE change_id = $1 ORDER BY at_ts DESC LIMIT 1"
        );
        let row = conn.query_opt(&sql, &[&change_id]).await?;
        Ok(row.map(|row| row.get(0)))
    }

    async fn insert_resource_change(&self, change: &ResourceChangeInsert) -> Result<()> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "resource_changes");
        let sql = format!(
            "INSERT INTO {table} (change_id, resource_type, resource_id, resource_key2, change_kind, workflow_policy_id, workflow_state, owner_user_id, created_by_kind, created_by_id, ingestion_source, ingestion_run_id, payload, meta) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"
        );
        conn.execute(
            &sql,
            &[
                &change.change_id,
                &change.resource_type,
                &change.resource_id,
                &change.resource_key2,
                &change.change_kind.as_str(),
                &change.workflow_policy_id,
                &change.workflow_state,
                &change.owner_user_id,
                &change.created_by_kind.as_str(),
                &change.created_by_id,
                &change.ingestion_source,
                &change.ingestion_run_id,
                &change.payload,
                &change.meta,
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
        let conn = self.get_conn().await?;
        let table = table_name(self, "resource_changes");
        let sql = format!(
            "UPDATE {table} SET workflow_state = $2, \
                workflow_policy_id = COALESCE($3, workflow_policy_id), \
                submitted_at = COALESCE($4, submitted_at), \
                applied_at = COALESCE($5, applied_at), \
                updated_at = now() \
             WHERE change_id = $1"
        );
        conn.execute(
            &sql,
            &[
                &change_id,
                &workflow_state,
                &workflow_policy_id,
                &submitted_at,
                &applied_at,
            ],
        )
        .await?;
        Ok(())
    }

    async fn get_resource_change(&self, change_id: &str) -> Result<Option<ResourceChange>> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "resource_changes");
        let sql = format!(
            "SELECT change_id, resource_type, resource_id, resource_key2, change_kind, workflow_policy_id, workflow_state, owner_user_id, created_by_kind, created_by_id, submitted_at, applied_at, base_etag, ingestion_source, ingestion_run_id, payload, meta \
             FROM {table} WHERE change_id = $1"
        );
        let row = conn.query_opt(&sql, &[&change_id]).await?;
        match row {
            Some(row) => {
                let change_kind: String = row.get(4);
                let created_by_kind: String = row.get(8);
                Ok(Some(ResourceChange {
                    change_id: row.get(0),
                    resource_type: row.get(1),
                    resource_id: row.get(2),
                    resource_key2: row.get(3),
                    change_kind: parse_change_kind(change_kind)?,
                    workflow_policy_id: row.get(5),
                    workflow_state: row.get(6),
                    owner_user_id: row.get(7),
                    created_by_kind: parse_actor_kind(created_by_kind)?,
                    created_by_id: row.get(9),
                    submitted_at: row.get(10),
                    applied_at: row.get(11),
                    base_etag: row.get(12),
                    ingestion_source: row.get(13),
                    ingestion_run_id: row.get(14),
                    payload: row.get(15),
                    meta: row.get(16),
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_resource_changes_for_owner(
        &self,
        owner_user_id: &str,
    ) -> Result<Vec<ResourceChange>> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "resource_changes");
        let sql = format!(
            "SELECT change_id, resource_type, resource_id, resource_key2, change_kind, workflow_policy_id, workflow_state, owner_user_id, created_by_kind, created_by_id, submitted_at, applied_at, base_etag, ingestion_source, ingestion_run_id, payload, meta \
             FROM {table} WHERE owner_user_id = $1"
        );
        let rows = conn.query(&sql, &[&owner_user_id]).await?;
        let mut out = Vec::new();
        for row in rows {
            let change_kind: String = row.get(4);
            let created_by_kind: String = row.get(8);
            out.push(ResourceChange {
                change_id: row.get(0),
                resource_type: row.get(1),
                resource_id: row.get(2),
                resource_key2: row.get(3),
                change_kind: parse_change_kind(change_kind)?,
                workflow_policy_id: row.get(5),
                workflow_state: row.get(6),
                owner_user_id: row.get(7),
                created_by_kind: parse_actor_kind(created_by_kind)?,
                created_by_id: row.get(9),
                submitted_at: row.get(10),
                applied_at: row.get(11),
                base_etag: row.get(12),
                ingestion_source: row.get(13),
                ingestion_run_id: row.get(14),
                payload: row.get(15),
                meta: row.get(16),
            });
        }
        Ok(out)
    }

    async fn latest_verified_state(
        &self,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<Option<String>> {
        let conn = self.get_conn().await?;
        let table = table_name(self, "resource_changes");
        let sql = format!(
            "SELECT workflow_state FROM {table} \
             WHERE resource_type = $1 AND resource_id = $2 \
               AND workflow_state IN ('VERIFIED','SYSTEM_VERIFIED') \
             ORDER BY applied_at DESC, updated_at DESC LIMIT 1"
        );
        let row = conn
            .query_opt(&sql, &[&resource_type, &resource_id])
            .await?;
        Ok(row.map(|row| row.get(0)))
    }

    async fn apply_change_to_verified(&self, change: &ResourceChange) -> Result<()> {
        use crate::governance::{parse_series_resource_id, resource_types};
        use crate::sql::{statements, Backend};

        let conn = self.get_conn().await?;
        let naming = self.naming().clone();
        match change.resource_type.as_str() {
            resource_types::INSTRUMENT => {
                let sql = statements::upsert_instrument_sql_with_naming(Backend::Postgres, &naming);
                conn.execute(
                    sql.as_ref(),
                    &[&change.resource_id, &change.payload, &change.meta],
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
                    statements::upsert_market_context_sql_with_naming(Backend::Postgres, &naming);
                conn.execute(
                    sql.as_ref(),
                    &[
                        &change.resource_id,
                        &change.resource_key2,
                        &change.payload,
                        &change.meta,
                    ],
                )
                .await?;
            }
            resource_types::PORTFOLIO => {
                if change.resource_key2.is_empty() {
                    return Err(Error::Invariant("Missing as_of for portfolio".to_string()));
                }
                let sql = statements::upsert_portfolio_sql_with_naming(Backend::Postgres, &naming);
                conn.execute(
                    sql.as_ref(),
                    &[
                        &change.resource_id,
                        &change.resource_key2,
                        &change.payload,
                        &change.meta,
                    ],
                )
                .await?;
            }
            resource_types::SCENARIO => {
                let sql = statements::upsert_scenario_sql_with_naming(Backend::Postgres, &naming);
                conn.execute(
                    sql.as_ref(),
                    &[&change.resource_id, &change.payload, &change.meta],
                )
                .await?;
            }
            resource_types::STATEMENT_MODEL => {
                let sql =
                    statements::upsert_statement_model_sql_with_naming(Backend::Postgres, &naming);
                conn.execute(
                    sql.as_ref(),
                    &[&change.resource_id, &change.payload, &change.meta],
                )
                .await?;
            }
            resource_types::METRIC_REGISTRY => {
                let sql =
                    statements::upsert_metric_registry_sql_with_naming(Backend::Postgres, &naming);
                conn.execute(
                    sql.as_ref(),
                    &[&change.resource_id, &change.payload, &change.meta],
                )
                .await?;
            }
            resource_types::SERIES_META => {
                let (namespace, kind, series_id) = parse_series_resource_id(&change.resource_id)?;
                let sql =
                    statements::upsert_series_meta_sql_with_naming(Backend::Postgres, &naming);
                conn.execute(sql.as_ref(), &[&namespace, &kind, &series_id, &change.meta])
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
