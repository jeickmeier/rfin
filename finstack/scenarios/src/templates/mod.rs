//! Historical stress test template types and builders.

mod builder;
mod json;
mod loader;
mod metadata;
mod registry;

pub use builder::ScenarioSpecBuilder;
pub use metadata::{AssetClass, Severity, TemplateMetadata};
pub use registry::{RegisteredTemplate, TemplateRegistry};

/// Register built-in templates into a registry.
fn register_builtins(registry: &mut TemplateRegistry) -> crate::Result<()> {
    let documents = loader::load_embedded_documents()?;

    for document in documents {
        registry.register_json_document(document)?;
    }

    Ok(())
}
