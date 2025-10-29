-- Use the elt schema for all ELT operations
SET search_path TO elt, public;

-- Notion Pages stream table
-- Stores pages and their metadata from Notion workspaces

CREATE TABLE IF NOT EXISTS stream_notion_pages (
    -- Identity
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,

    -- Notion page identifiers
    page_id TEXT NOT NULL,  -- Notion's page ID (UUID format)
    url TEXT NOT NULL,  -- Direct link to the page

    -- Timing (from Notion)
    created_time TIMESTAMPTZ NOT NULL,
    last_edited_time TIMESTAMPTZ NOT NULL,

    -- People (creators/editors)
    created_by_id TEXT NOT NULL,
    created_by_name TEXT,
    last_edited_by_id TEXT NOT NULL,
    last_edited_by_name TEXT,

    -- Parent information (what contains this page)
    parent_type TEXT NOT NULL,  -- "database", "page", or "workspace"
    parent_id TEXT,  -- Database ID or Page ID (null for workspace)

    -- Status
    archived BOOLEAN DEFAULT false,

    -- Properties and metadata
    properties JSONB NOT NULL DEFAULT '{}'::jsonb,  -- Page properties (title, tags, etc)

    -- Full data backup
    raw_json JSONB,  -- Complete page object from Notion API

    -- Our timestamps
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Prevent duplicate pages
    UNIQUE(source_id, page_id)
);

-- Indexes for common queries
CREATE INDEX idx_notion_pages_source ON stream_notion_pages(source_id);
CREATE INDEX idx_notion_pages_page_id ON stream_notion_pages(page_id);
CREATE INDEX idx_notion_pages_last_edited ON stream_notion_pages(last_edited_time DESC);
CREATE INDEX idx_notion_pages_created ON stream_notion_pages(created_time DESC);
CREATE INDEX idx_notion_pages_archived ON stream_notion_pages(archived) WHERE archived = false;
CREATE INDEX idx_notion_pages_parent ON stream_notion_pages(parent_type, parent_id);

-- Index for finding recently synced pages
CREATE INDEX idx_notion_pages_sync_time ON stream_notion_pages(source_id, synced_at DESC);

-- Full-text search on properties (for page titles and content)
CREATE INDEX idx_notion_pages_properties_search ON stream_notion_pages USING GIN (properties jsonb_path_ops);

-- Trigger for updated_at
CREATE TRIGGER stream_notion_pages_updated_at
    BEFORE UPDATE ON stream_notion_pages
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- Comments
COMMENT ON TABLE stream_notion_pages IS 'Notion pages with metadata, properties, and relationships';
COMMENT ON COLUMN stream_notion_pages.page_id IS 'Notion page ID (UUID) - unique identifier';
COMMENT ON COLUMN stream_notion_pages.parent_type IS 'Type of parent container: database, page, or workspace';
COMMENT ON COLUMN stream_notion_pages.parent_id IS 'ID of parent database or page (null if workspace is parent)';
COMMENT ON COLUMN stream_notion_pages.properties IS 'Page properties including title, tags, dates, and custom fields';
COMMENT ON COLUMN stream_notion_pages.raw_json IS 'Complete page object from Notion API for data recovery';
