// Reuse the public production contract tests in this isolated dependency graph.
// The historical model remains a discriminator; these tests call real Core.
#[path = "../../../../core/tests/consumer_compiler_support.rs"]
mod production;
