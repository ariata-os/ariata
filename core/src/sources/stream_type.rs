use crate::sources::{PullStream, PushStream};

/// Discriminated union representing either a pull or push stream
///
/// This enum allows the system to work with streams polymorphically while
/// maintaining type safety and enabling compile-time guarantees about which
/// operations are valid for which stream types.
///
/// # Examples
///
/// ```ignore
/// match stream_type {
///     StreamType::Pull(stream) => {
///         // Only pull operations are valid
///         stream.sync_pull(SyncMode::Incremental).await?;
///     }
///     StreamType::Push(stream) => {
///         // Only push operations are valid
///         stream.receive_push(payload).await?;
///     }
/// }
/// ```
pub enum StreamType {
    /// Backend-initiated pull from external API
    ///
    /// Examples: Google Calendar, Gmail, Notion
    /// - Backend controls when sync happens (via scheduler)
    /// - Uses OAuth tokens or API keys
    /// - Supports incremental sync with cursors
    Pull(Box<dyn PullStream>),

    /// Client-initiated push from device
    ///
    /// Examples: Mac apps, iOS location, iMessage
    /// - Client controls when sync happens (whenever it has new data)
    /// - Uses device tokens for authentication
    /// - Backend is passive receiver
    Push(Box<dyn PushStream>),
}

impl StreamType {
    /// Get the source name regardless of stream type
    pub fn source_name(&self) -> &str {
        match self {
            StreamType::Pull(stream) => stream.source_name(),
            StreamType::Push(stream) => stream.source_name(),
        }
    }

    /// Get the stream name regardless of stream type
    pub fn stream_name(&self) -> &str {
        match self {
            StreamType::Pull(stream) => stream.stream_name(),
            StreamType::Push(stream) => stream.stream_name(),
        }
    }

    /// Get the table name regardless of stream type
    pub fn table_name(&self) -> &str {
        match self {
            StreamType::Pull(stream) => stream.table_name(),
            StreamType::Push(stream) => stream.table_name(),
        }
    }

    /// Check if this is a pull stream
    pub fn is_pull(&self) -> bool {
        matches!(self, StreamType::Pull(_))
    }

    /// Check if this is a push stream
    pub fn is_push(&self) -> bool {
        matches!(self, StreamType::Push(_))
    }

    /// Get a reference to the PullStream if this is a pull stream
    pub fn as_pull(&self) -> Option<&dyn PullStream> {
        match self {
            StreamType::Pull(stream) => Some(&**stream),
            StreamType::Push(_) => None,
        }
    }

    /// Get a reference to the PushStream if this is a push stream
    pub fn as_push(&self) -> Option<&dyn PushStream> {
        match self {
            StreamType::Pull(_) => None,
            StreamType::Push(stream) => Some(&**stream),
        }
    }

    /// Get a mutable reference to the PullStream if this is a pull stream
    ///
    /// This is needed for operations that require mutable access, such as
    /// PullStream::load_config() which configures OAuth tokens and state.
    pub fn as_pull_mut(&mut self) -> Option<&mut dyn PullStream> {
        match self {
            StreamType::Pull(stream) => Some(&mut **stream),
            StreamType::Push(_) => None,
        }
    }

    /// Get a mutable reference to the PushStream if this is a push stream
    pub fn as_push_mut(&mut self) -> Option<&mut dyn PushStream> {
        match self {
            StreamType::Pull(_) => None,
            StreamType::Push(stream) => Some(&mut **stream),
        }
    }
}

impl std::fmt::Debug for StreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamType::Pull(stream) => {
                f.debug_struct("StreamType::Pull")
                    .field("source", &stream.source_name())
                    .field("stream", &stream.stream_name())
                    .field("table", &stream.table_name())
                    .finish()
            }
            StreamType::Push(stream) => {
                f.debug_struct("StreamType::Push")
                    .field("source", &stream.source_name())
                    .field("stream", &stream.stream_name())
                    .field("table", &stream.table_name())
                    .finish()
            }
        }
    }
}
