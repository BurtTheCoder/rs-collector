// Export all items from the submodules
mod collector;
mod walker;
mod helpers;

// Re-export the main collector
pub use collector::RegexCollector;
