// Export all items from the submodules
mod collector;
mod helpers;
mod walker;

// Re-export the main collector
pub use collector::RegexCollector;
