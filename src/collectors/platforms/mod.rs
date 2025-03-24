pub mod common;
pub mod windows;
pub mod linux;
pub mod macos;

use log::info;

use crate::config::Artifact;
use crate::collectors::collector::ArtifactCollector;

/// Get the appropriate collector for the current platform
pub fn get_platform_collector() -> Box<dyn ArtifactCollector> {
    #[cfg(target_os = "windows")]
    {
        info!("Using Windows-specific collector");
        Box::new(windows::WindowsCollector::new())
    }
    #[cfg(target_os = "linux")]
    {
        info!("Using Linux-specific collector");
        Box::new(linux::LinuxCollector::new())
    }
    #[cfg(target_os = "macos")]
    {
        info!("Using macOS-specific collector");
        Box::new(macos::MacOSCollector::new())
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        info!("Using fallback collector for unsupported platform");
        Box::new(common::FallbackCollector::new())
    }
}

/// Filter artifacts based on the current platform
pub fn filter_artifacts_for_platform(artifacts: &[Artifact]) -> Vec<Artifact> {
    let platform_collector = get_platform_collector();
    
    artifacts.iter()
        .filter(|artifact| platform_collector.supports_artifact_type(&artifact.artifact_type))
        .cloned()
        .collect()
}
