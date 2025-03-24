use std::sync::Arc;

use anyhow::Result;
use log::warn;
use rusoto_core::Region;
use rusoto_s3::S3Client;

/// Create an S3 client with the specified region and profile
pub fn create_s3_client(region_name: Option<&str>, profile: Option<&str>) -> Result<Arc<S3Client>> {
    let region = match region_name {
        Some(name) => {
            match name.parse::<Region>() {
                Ok(r) => r,
                Err(_) => {
                    warn!("Invalid region '{}', using default", name);
                    Region::default()
                }
            }
        },
        None => Region::default(),
    };
    
    // Create S3 client with profile if specified
    let s3_client = if let Some(profile_name) = profile {
        match rusoto_credential::ProfileProvider::new() {
            Ok(mut provider) => {
                provider.set_profile(profile_name);
                Arc::new(S3Client::new_with(
                    rusoto_core::HttpClient::new().expect("Failed to create HTTP client"),
                    provider,
                    region.clone()
                ))
            },
            Err(e) => {
                warn!("Failed to create AWS profile provider: {}, using default", e);
                Arc::new(S3Client::new(region.clone()))
            }
        }
    } else {
        Arc::new(S3Client::new(region.clone()))
    };
    
    Ok(s3_client)
}
