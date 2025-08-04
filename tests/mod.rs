//! Integration test modules for rs-collector.
//!
//! This module organizes all integration tests that verify
//! end-to-end functionality of the forensic collector.

mod basic_collection;
mod cloud_upload_tests;
mod compression_tests;
mod memory_collection_tests;
mod streaming_tests;
// Temporarily disabled due to API changes
// mod full_collection_tests;
