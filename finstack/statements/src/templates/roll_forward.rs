//! Roll-forward pattern implementation.

use crate::builder::ModelBuilder;
use crate::error::Result;
use crate::types::{NodeSpec, NodeType};

/// Add a roll-forward structure to the model.
///
/// This creates:
/// - `{name}_beg`: Beginning balance (linked to previous period's ending balance)
/// - `{name}_end`: Ending balance (Begin + Increases - Decreases)
///
/// # Arguments
/// * `builder` - Model builder
/// * `name` - Base name for the roll-forward (e.g., "arr")
/// * `increases` - List of node IDs that increase the balance
/// * `decreases` - List of node IDs that decrease the balance
pub fn add_roll_forward<State>(
    mut builder: ModelBuilder<State>,
    name: &str,
    increases: &[&str],
    decreases: &[&str],
) -> Result<ModelBuilder<State>> {
    let beg_node_id = format!("{}_beg", name);
    let end_node_id = format!("{}_end", name);

    // 1. Create Beginning Balance Node
    // Formula: lag(end_node, 1)
    // Use coalesce to handle the first period (defaults to 0 if no history)
    let beg_formula = format!("coalesce(lag({}, 1), 0.0)", end_node_id);
    let beg_node = NodeSpec::new(beg_node_id.as_str(), NodeType::Calculated)
        .with_name(format!("{} (Beginning)", name))
        .with_formula(beg_formula);

    // 2. Create Ending Balance Node
    // Formula: beg + sum(increases) - sum(decreases)
    let mut end_formula = beg_node_id.clone();

    if !increases.is_empty() {
        end_formula.push_str(" + ");
        end_formula.push_str(&increases.join(" + "));
    }

    if !decreases.is_empty() {
        end_formula.push_str(" - (");
        end_formula.push_str(&decreases.join(" + "));
        end_formula.push(')');
    }

    let end_node = NodeSpec::new(end_node_id.as_str(), NodeType::Calculated)
        .with_name(format!("{} (Ending)", name))
        .with_formula(end_formula);

    // 3. Add nodes to builder
    builder.nodes.insert(beg_node_id, beg_node);
    builder.nodes.insert(end_node_id, end_node);

    Ok(builder)
}
