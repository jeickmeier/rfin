//! Build script delegating currency code-generation to a sub-module.

mod currency_build;

fn main() -> std::io::Result<()> {
    currency_build::generate()
}
