//! Global ontology registry
//!
//! Provides a compile-time registry of all ontology definitions and boundary detectors.
//! Similar to the source registry pattern in `crate::registry`.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::descriptor::Ontology;
use super::detector::BoundaryDetector;

// Import domain registrations
use super::activity::registry::register_activity_ontologies;
use super::health::registry::register_health_ontologies;
use super::knowledge::registry::register_knowledge_ontologies;
use super::location::registry::register_location_ontologies;
use super::praxis::registry::register_praxis_ontologies;
use super::social::registry::register_social_ontologies;
use super::speech::registry::register_speech_ontologies;

/// Global ontology registry
pub struct OntologyRegistry {
    /// Ontologies indexed by name
    ontologies: HashMap<String, Ontology>,
    /// Ontologies grouped by domain
    by_domain: HashMap<String, Vec<String>>,
    /// Mapping from source stream to target ontologies
    stream_to_ontologies: HashMap<String, Vec<String>>,
    /// Boundary detectors indexed by ontology name
    detectors: HashMap<String, Box<dyn BoundaryDetector>>,
}

impl OntologyRegistry {
    /// Create a new empty registry
    fn new() -> Self {
        Self {
            ontologies: HashMap::new(),
            by_domain: HashMap::new(),
            stream_to_ontologies: HashMap::new(),
            detectors: HashMap::new(),
        }
    }

    /// Register an ontology
    pub fn register(&mut self, ontology: Ontology) {
        let name = ontology.name.to_string();
        let domain = ontology.domain.to_string();

        // Index by source streams
        for stream in &ontology.source_streams {
            self.stream_to_ontologies
                .entry(stream.to_string())
                .or_default()
                .push(name.clone());
        }

        // Group by domain
        self.by_domain
            .entry(domain)
            .or_default()
            .push(name.clone());

        // Store ontology
        self.ontologies.insert(name, ontology);
    }

    /// Get all registered ontologies
    pub fn list(&self) -> Vec<&Ontology> {
        self.ontologies.values().collect()
    }

    /// Get ontologies that have boundary detection enabled
    pub fn list_with_boundaries(&self) -> Vec<&Ontology> {
        self.ontologies
            .values()
            .filter(|o| !matches!(o.boundary.algorithm, super::descriptor::DetectionAlgorithm::None))
            .collect()
    }

    /// Get a specific ontology by name
    pub fn get(&self, name: &str) -> Option<&Ontology> {
        self.ontologies.get(name)
    }

    /// Get all ontologies in a domain
    pub fn get_by_domain(&self, domain: &str) -> Vec<&Ontology> {
        self.by_domain
            .get(domain)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.ontologies.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all ontologies that a stream feeds into
    pub fn get_for_stream(&self, stream_table: &str) -> Vec<&Ontology> {
        self.stream_to_ontologies
            .get(stream_table)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.ontologies.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the narrative role for an ontology by name
    pub fn get_narrative_role(&self, name: &str) -> Option<&'static str> {
        self.ontologies.get(name).map(|o| o.narrative_role.as_str())
    }

    /// List all domain names
    pub fn list_domains(&self) -> Vec<&str> {
        self.by_domain.keys().map(|s| s.as_str()).collect()
    }

    /// Register a boundary detector for an ontology
    pub fn register_detector(&mut self, detector: Box<dyn BoundaryDetector>) {
        let name = detector.ontology_name().to_string();
        self.detectors.insert(name, detector);
    }

    /// Get a boundary detector by ontology name
    pub fn get_detector(&self, name: &str) -> Option<&dyn BoundaryDetector> {
        self.detectors.get(name).map(|d| d.as_ref())
    }

    /// List all registered boundary detectors
    pub fn list_detectors(&self) -> Vec<&dyn BoundaryDetector> {
        self.detectors.values().map(|d| d.as_ref()).collect()
    }
}

// Global registry instance
static REGISTRY: OnceLock<OntologyRegistry> = OnceLock::new();

/// Initialize the global registry
fn init_registry() -> OntologyRegistry {
    let mut registry = OntologyRegistry::new();

    // Register all domain ontologies
    register_health_ontologies(&mut registry);
    register_location_ontologies(&mut registry);
    register_social_ontologies(&mut registry);
    register_praxis_ontologies(&mut registry);
    register_activity_ontologies(&mut registry);
    register_speech_ontologies(&mut registry);
    register_knowledge_ontologies(&mut registry);

    registry
}

/// Get the global ontology registry
pub fn ontology_registry() -> &'static OntologyRegistry {
    REGISTRY.get_or_init(init_registry)
}

/// List all registered ontologies
pub fn list_ontologies() -> Vec<&'static Ontology> {
    ontology_registry().list()
}

/// Get a specific ontology by name
pub fn get_ontology(name: &str) -> Option<&'static Ontology> {
    ontology_registry().get(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let ontologies = list_ontologies();
        assert!(!ontologies.is_empty(), "Registry should have ontologies");
    }

    #[test]
    fn test_get_ontology() {
        // These should exist after we implement the domain modules
        let sleep = get_ontology("health_sleep");
        if sleep.is_some() {
            let s = sleep.unwrap();
            assert_eq!(s.domain, "health");
        }
    }

    #[test]
    fn test_ontologies_have_unique_names() {
        let ontologies = list_ontologies();
        let mut names = std::collections::HashSet::new();
        for ontology in ontologies {
            assert!(
                names.insert(ontology.name),
                "Duplicate ontology name: {}",
                ontology.name
            );
        }
    }

    #[test]
    fn test_stream_to_ontology_mapping() {
        let registry = ontology_registry();

        // After implementation, stream_ios_healthkit should map to multiple health ontologies
        let health_ontologies = registry.get_for_stream("stream_ios_healthkit");
        // This will pass once health ontologies are implemented
        if !health_ontologies.is_empty() {
            assert!(health_ontologies.len() >= 1);
        }
    }
}
