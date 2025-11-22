//! Transcription ontology
//!
//! Audio transcriptions from iOS microphone recordings.
//! High fidelity - physical voice activity cannot be faked.

use crate::ontologies::{NarrativeRole, Ontology, OntologyBuilder, OntologyDescriptor};

pub struct TranscriptionOntology;

impl OntologyDescriptor for TranscriptionOntology {
    fn descriptor() -> Ontology {
        OntologyBuilder::new("speech_transcription")
            .display_name("Voice Transcriptions")
            .description("Transcribed audio from microphone recordings")
            .domain("speech")
            .table_name("speech_transcription")
            .source_streams(vec!["stream_ios_microphone"])
            .narrative_role(NarrativeRole::Substance)
            // Interval detection - each conversation record has clear start/end
            .interval_boundaries(
                "recorded_at",
                "recorded_at + audio_duration_seconds * interval '1 second'",
                vec![], // no filters
                0.95,   // fidelity
                70,     // weight
                vec!["speaker_count", "language", "confidence_score"],
            )
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcription_ontology() {
        let ont = TranscriptionOntology::descriptor();
        assert_eq!(ont.name, "speech_transcription");
        assert_eq!(ont.boundary.fidelity, 0.95);
    }
}
