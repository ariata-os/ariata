//! Library-level API functions.

pub mod applet_events;
pub mod applet_source;
pub mod annotations;
pub mod ai_calls;
pub mod ai_complete;
pub mod assistant_profile;
pub mod audit;
pub mod backup_status;
pub use backup_status::get_backup_status;
pub mod auth;
pub mod billing_state;
pub mod box_status;
pub mod devices;
pub mod display;
pub mod pair;
pub mod settings_byo;
pub mod setup;
pub mod sudo;
pub mod web_bundle;
pub mod chat;
pub mod chat_permissions;
pub mod chat_usage;
pub mod chats;
pub mod code;
pub mod compaction;
pub mod credentials;
pub mod image_gen;
pub mod day_summary;
pub mod developer;
pub mod drive;
pub mod entities;
pub mod web_search;
pub mod home;
pub mod internal;
pub mod lake;
pub mod media;
pub mod model_catalog;
pub mod model_choice;
pub mod models;
pub mod entity_article_gen;
pub mod assistant_memories;
pub mod circumstances;
pub mod narrative_draft;
pub mod bookmarks;
pub mod pages;
pub mod personas;
pub mod updates;
pub mod pins;
pub mod search_local;
pub mod places;
pub mod profile;
pub mod census;
pub mod identity;
pub mod setup_phrase;
pub mod network;
pub mod provision;
pub mod notebooks;
pub mod records;
pub mod refs;
pub mod source_auth;
pub mod subscription;
pub mod stream_health;
pub mod system_display;
pub mod system_telemetry;
pub mod system_update;
pub mod terminal;
pub mod token_estimation;
pub mod unsplash;
pub mod wiki;
pub mod lifeline;
pub mod link_session;
pub mod wiki_articles;
pub mod wiki_notes;

// Re-export all functions for convenience
pub use assistant_profile::{
    get_assistant_name, get_assistant_profile, update_assistant_profile,
    UpdateAssistantProfileRequest,
};
pub use auth::{session_handler, SessionResponse, SessionUser};
pub use code::{execute_code, ExecuteCodeRequest, ExecuteCodeResponse};
pub use credentials::{
    check_pairing_status, delete_pending_credential, list_credentials, list_pending_pairings,
    rename_credential, revoke_credential, CredentialListItem, DeviceInfo, PairingStatus,
    PendingPairing,
};
pub use drive::{
    check_quota as check_drive_quota,
    create_folder as create_drive_folder,
    delete_file as delete_drive_file,
    download_file as download_drive_file,
    download_file_stream as download_drive_file_stream,
    download_lake_object,
    empty_trash as empty_drive_trash,
    get_drive_usage,
    get_file_metadata as get_drive_file,
    is_lake_object_id,
    list_files as list_drive_files,
    list_media as list_drive_media,
    list_trash as list_drive_trash,
    move_file as move_drive_file,
    purge_file as purge_drive_file,
    purge_old_trash as purge_old_drive_trash,
    reconcile_usage as reconcile_drive_usage,
    restore_file as restore_drive_file,
    reextract_file as reextract_drive_file,
    upload_file as upload_drive_file,
    validate_drive_path,
    CreateFolderRequest as DriveCreateFolderRequest,
    DriveConfig,
    DriveFile,
    StagedUpload,
    DriveUsage,
    MoveFileRequest as DriveMoveFileRequest,
    UploadRequest as DriveUploadRequest,
};
pub use entities::{
    create_place, delete_place, get_place, list_places, set_home_place as set_home_place_entity,
    update_place, CreatePlaceRequest, CreatePlaceResponse, Place, UpdatePlaceRequest,
};
pub use media::{
    get_media, is_audio_type, is_image_type, is_supported_media_type, is_video_type, upload_media,
    MediaFile, UploadMediaRequest,
};
pub use stream_health::{stream_health, StreamHealth};
pub use models::{
    get_model, list_models, list_models_with_slots, ModelInfo, ModelsResponse,
};
pub use unsplash::{
    search as unsplash_search, SearchRequest as UnsplashSearchRequest,
    SearchResponse as UnsplashSearchResponse,
};

pub use chat_permissions::{
    add_permission, clear_permissions, has_permission, list_permissions, remove_permission,
    AddPermissionRequest, ChatEditPermission, PermissionListResponse, PermissionResponse,
};
pub use chats::{
    append_message, create_chat, create_chat_from_request, delete_chat, generate_title, get_chat,
    list_chats, update_chat, update_messages, Chat, ChatDetailResponse, ChatListItem,
    ChatListResponse, ChatMessage, ConversationMeta, CreateChatRequest, CreateChatResponse,
    DeleteChatResponse, GenerateTitleRequest, GenerateTitleResponse, IntentMetadata,
    MessageResponse, TimeRange, TitleMessage, ToolCall, UpdateChatRequest, UpdateChatResponse,
};
pub use internal::ensure_server_status;
pub use pages::{
    create_page,
    create_page_share,
    create_version,
    delete_page,
    delete_page_share,
    get_page,
    get_page_backlinks,
    get_page_share,
    get_shared_page,
    get_version,
    list_pages,
    list_versions,
    search_refs,
    update_page,
    validate_shared_file,
    Backlink,
    BacklinksResponse,
    CreatePageRequest,
    CreateVersionRequest,
    RefSearchResponse,
    RefSearchResult,
    Page,
    PageListResponse,
    PageShare,
    PageSummary,
    PageVersionDetail,
    PageVersionSummary,
    PageVersionsListResponse,
    SharedPage,
    UpdatePageRequest,
};
pub use places::{
    autocomplete, get_place_details, AutocompletePrediction, AutocompleteRequest,
    AutocompleteResponse, PlaceDetailsRequest, PlaceDetailsResponse,
};
pub use updates::{
    apply as apply_update, set_channel, status as update_status, ApplyResponse, SetChannelRequest,
};
pub use search_local::{search_local, LocalSearchRequest, LocalSearchResponse};
pub use bookmarks::{
    get_bookmark, list_bookmarks, save_bookmark, update_note, BookmarkDetail, BookmarkListItem,
    BookmarkPage, ListBookmarksQuery, SaveBookmarkRequest, SavedBookmark, UpdateNoteRequest,
};
pub use pins::{
    create_pin, delete_pin, list_pins, reorder_pins, update_pin, CreatePinRequest, Pin,
    UpdatePinRequest,
};
pub use annotations::{
    create_annotation, delete_annotation, export_file_annotations_md,
    export_notebook_annotations_md, get_annotation, list_annotations,
    list_notebook_annotations, update_annotation,
    Annotation, CreateAnnotationRequest, UpdateAnnotationRequest,
};
pub use notebooks::{
    add_notebook_item, create_notebook, delete_notebook, get_notebook, list_notebooks,
    remove_notebook_item, reorder_notebook_items, set_chat_notebook, touch_notebook,
    update_notebook, AddNotebookItemRequest, CreateNotebookRequest, Notebook, NotebookDetail,
    NotebookItem, NotebookListResponse, NotebookSummary, ReorderNotebookItemsRequest,
    UpdateNotebookRequest,
};

pub use chat_usage::{
    check_compaction_needed, get_chat_usage,
    record_chat_usage, ChatUsageInfo, CompactionStatus, UsageData,
};
pub use developer::{execute_sql, list_tables, ExecuteSqlRequest};
pub use personas::{
    create_persona, get_persona, get_persona_content, hide_persona, list_all_personas,
    list_personas, reset_personas, unhide_persona, update_persona, CreatePersonaRequest, Persona,
    PersonaListResponse, PersonasData, UpdatePersonaRequest,
};
pub use profile::{get_display_name, get_profile, update_profile, UpdateProfileRequest};
pub use system_update::CURRENT_COMMIT;
pub use token_estimation::{
    estimate_message_tokens, estimate_session_context, estimate_tokens, ContextEstimate,
    ContextStatus,
};
pub use home::{get_calendar_upcoming, get_current_weather, get_unnamed_places};
pub use wiki::{
    create_temporal_event,
    delete_auto_events_for_day,
    delete_temporal_event,
    get_narrative_identity,
    NarrativeIdentity,
    get_day_chats,
    get_day_events,
    get_day_sources,
    get_day_streams,
    get_timeline_day,
    get_today_streams,
    get_events_by_date,
    get_or_create_day,
    get_organization,
    get_person,
    get_wiki_place,
    day_activity,
    DayActivity,
    on_this_day,
    OnThisDayEntry,
    get_entity_records_page,
    get_entity_record_facets,
    EntityRecord,
    EntityRecordsPage,
    EntityRecordFacet,
    list_days,
    list_organizations,
    list_people,
    list_wiki_places,
    resolve_id,
    update_day,
    update_organization,
    update_person,
    update_temporal_event,
    update_wiki_place,
    CreateTemporalEventRequest,
    DaySource,
    DayStream,
    DayStreamsResponse,
    IdResolution,
    StreamRecord,
    TemporalEvent,
    UpdateTemporalEventRequest,
    UpdateWikiDayRequest,
    UpdateWikiOrganizationRequest,
    UpdateWikiPersonRequest,
    UpdateWikiPlaceRequest,
    WikiDay,
    WikiOrganization,
    WikiOrganizationListItem,
    WikiPerson,
    WikiPersonListItem,
    WikiPlace,
    WikiPlaceListItem,
};
