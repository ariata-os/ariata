"""Auto-generated registry of sources and streams."""

# Generated at: 2025-08-25T22:09:36.569561Z

REGISTRY = {
  "sources": {
    "google": {
      "name": "google",
      "display_name": "Google",
      "icon": "ri:google-fill",
      "video": "google2.webm",
      "platform": "cloud",
      "company": "google",
      "description": "Google services including Calendar, Gmail, and Drive",
      "auth": {
        "type": "oauth2",
        "provider": "google",
        "auth_url": "https://accounts.google.com/o/oauth2/v2/auth",
        "token_url": "https://oauth2.googleapis.com/token",
        "revoke_url": "https://oauth2.googleapis.com/revoke",
        "auth_proxy": "https://auth.ariata.com/google/auth",
        "token_refresh_window": 300
      },
      "streams": [
        {
          "name": "calendar",
          "display_name": "Google Calendar",
          "description": "Calendar events and appointments",
          "required_scopes": [
            "https://www.googleapis.com/auth/calendar.readonly",
            "https://www.googleapis.com/auth/calendar.events.readonly"
          ]
        }
      ],
      "api": {
        "base_url": "https://www.googleapis.com",
        "version": "v3",
        "rate_limits": {
          "per_user_per_100_seconds": 100000,
          "per_project_per_100_seconds": 1000000
        },
        "timeout": 30000
      },
      "sync": {
        "type": "pull",
        "page_size": 100
      },
      "streams_config": [
        {
          "name": "google_calendar",
          "display_name": "Google Calendar",
          "description": "Calendar events and appointments",
          "required_scopes": []
        }
      ]
    },
    "ios": {
      "name": "ios",
      "display_name": "iPhone",
      "icon": "ri:smartphone-line",
      "video": "ios.webm",
      "platform": "device",
      "company": "apple",
      "description": "All data from your iPhone including location, sound, and voice",
      "auth": {
        "type": "device_token",
        "required_fields": [
          "device_id",
          "device_name",
          "os_version"
        ],
        "device_setup": {
          "title": "Connect your iPhone",
          "app_store_url": null,
          "setup_steps": [
            {
              "step": "app",
              "label": "Open the Ariata app on your iPhone",
              "description": "Download from TestFlight or the App Store"
            },
            {
              "step": "settings",
              "label": "Go to Settings → Connection",
              "description": "Navigate to the connection settings"
            },
            {
              "step": "token",
              "label": "Enter your device token",
              "description": "Paste the token: {TOKEN}"
            },
            {
              "step": "connect",
              "label": "The app will connect automatically",
              "description": "Wait for the connection to establish"
            }
          ],
          "show_api_endpoint": true
        }
      },
      "streams": [
        {
          "name": "location",
          "display_name": "Core Location",
          "description": "GPS, altitude, and speed data"
        },
        {
          "name": "healthkit",
          "display_name": "HealthKit",
          "description": "Health metrics including heart rate, steps, sleep"
        },
        {
          "name": "mic",
          "display_name": "Microphone",
          "description": "Audio capture and transcription"
        }
      ],
      "requirements": {
        "min_os_version": "14.0",
        "capabilities": [
          "location_services",
          "healthkit",
          "microphone"
        ],
        "permissions": [
          "location_always",
          "health_read",
          "microphone"
        ]
      },
      "sync": {
        "batch_upload_interval": 300,
        "max_batch_size": 1000,
        "compression": "gzip",
        "retry_policy": {
          "max_retries": 3,
          "backoff_multiplier": 2
        }
      },
      "streams_config": [
        {
          "name": "ios_healthkit",
          "display_name": "iOS HealthKit",
          "description": "Health data including heart rate, steps, workouts, sleep, and energy",
          "required_scopes": []
        },
        {
          "name": "ios_location",
          "display_name": "iOS Core Location",
          "description": "Raw CoreLocation data including GPS, altitude, speed, and activity",
          "required_scopes": []
        },
        {
          "name": "ios_mic",
          "display_name": "iOS Microphone Audio",
          "description": "Audio chunks for transcription and analysis",
          "required_scopes": []
        }
      ]
    },
    "mac": {
      "name": "mac",
      "display_name": "Mac",
      "icon": "ri:mac-line",
      "video": "mac2.webm",
      "platform": "device",
      "company": "apple",
      "description": "Application usage and activity from your Mac",
      "auth": {
        "type": "device_token",
        "required_fields": [
          "device_id",
          "device_name",
          "os_version"
        ],
        "device_setup": {
          "title": "Install Mac Monitor",
          "download_url": "https://github.com/ariata-os/ariata/releases/latest/download/install.sh",
          "install_command": "curl -sSL https://github.com/ariata-os/ariata/releases/latest/download/install.sh | bash",
          "setup_steps": [
            {
              "step": "install",
              "label": "Install the Mac CLI tool",
              "command": "curl -sSL https://github.com/ariata-os/ariata/releases/latest/download/install.sh | bash"
            },
            {
              "step": "init",
              "label": "Initialize with your token",
              "command": "ariata-mac init {TOKEN}"
            },
            {
              "step": "daemon",
              "label": "Start monitoring (runs in background)",
              "command": "ariata-mac daemon"
            }
          ],
          "show_token_inline": true
        }
      },
      "streams": [
        {
          "name": "apps",
          "display_name": "Application Activity",
          "description": "Track which applications are in focus and for how long"
        },
        {
          "name": "messages",
          "display_name": "Messages",
          "description": "iMessage and SMS conversation history"
        }
      ],
      "requirements": {
        "min_os_version": "11.0",
        "capabilities": [
          "accessibility_api",
          "screen_recording",
          "full_disk_access"
        ],
        "permissions": [
          "accessibility",
          "automation",
          "full_disk_access"
        ]
      },
      "agent": {
        "type": "background_service",
        "launch_at_login": true,
        "update_channel": "stable"
      },
      "sync": {
        "batch_upload_interval": 300,
        "max_batch_size": 500,
        "compression": "gzip",
        "local_buffer": {
          "max_size_mb": 100,
          "retention_days": 7
        },
        "retry_policy": {
          "max_retries": 5,
          "backoff_multiplier": 2
        }
      },
      "monitoring": {
        "heartbeat_interval": 60,
        "crash_recovery": true,
        "auto_restart": true
      },
      "streams_config": [
        {
          "name": "mac_apps",
          "display_name": "Mac Applications",
          "description": "Application focus and usage events",
          "required_scopes": []
        },
        {
          "name": "mac_messages",
          "display_name": "Messages",
          "description": "iMessage and SMS conversation history",
          "required_scopes": []
        }
      ]
    },
    "notion": {
      "name": "notion",
      "display_name": "Notion",
      "icon": "ri:notion-fill",
      "video": "notion.webm",
      "platform": "cloud",
      "company": "notion",
      "description": "Notion workspace pages, databases, and blocks",
      "auth": {
        "type": "oauth2",
        "provider": "notion",
        "auth_url": "https://api.notion.com/v1/oauth/authorize",
        "token_url": "https://api.notion.com/v1/oauth/token",
        "auth_proxy": "https://auth.ariata.com/notion/auth",
        "public_integration": true,
        "token_refresh_window": 0
      },
      "streams": [
        {
          "name": "pages",
          "display_name": "Notion Pages",
          "description": "Pages and databases from Notion workspace",
          "required_permissions": [
            "read_content",
            "read_user"
          ]
        }
      ],
      "api": {
        "base_url": "https://api.notion.com",
        "version": "v1",
        "rate_limits": {
          "requests_per_second": 3
        },
        "timeout": 30000,
        "headers": {
          "Notion-Version": "2022-06-28"
        }
      },
      "sync": {
        "type": "pull"
      },
      "streams_config": [
        {
          "name": "notion_pages",
          "display_name": "Notion Pages",
          "description": "Pages and databases from Notion workspace",
          "required_scopes": []
        }
      ]
    },
    "strava": {
      "name": "strava",
      "display_name": "Strava",
      "icon": "simple-icons:strava",
      "video": "strava.webm",
      "platform": "cloud",
      "company": "strava",
      "description": "Fitness activities, workouts, and performance metrics from Strava",
      "auth": {
        "type": "oauth2",
        "provider": "strava",
        "auth_url": "https://www.strava.com/oauth/authorize",
        "token_url": "https://www.strava.com/oauth/token",
        "revoke_url": "https://www.strava.com/oauth/deauthorize",
        "auth_proxy": "https://auth.ariata.com/strava/auth",
        "token_refresh_window": 300
      },
      "streams": [
        {
          "name": "activities",
          "display_name": "Activities",
          "description": "Running, cycling, and other fitness activities"
        }
      ],
      "api": {
        "base_url": "https://www.strava.com/api",
        "version": "v3",
        "rate_limits": {
          "per_15_minutes": 100,
          "per_day": 1000
        },
        "timeout": 30000
      },
      "sync": {
        "type": "pull",
        "page_size": 30
      },
      "streams_config": [
        {
          "name": "strava_activities",
          "display_name": "Activities",
          "description": "Running, cycling, and other fitness activities",
          "required_scopes": []
        }
      ]
    }
  },
  "streams": {
    "google_calendar": {
      "name": "google_calendar",
      "source": "google",
      "display_name": "Google Calendar",
      "description": "Calendar events and appointments",
      "ingestion": {
        "type": "pull",
        "batch_type": "events",
        "raw_data_type": "CalendarEvent"
      },
      "api": {
        "endpoint": "https://www.googleapis.com/calendar/v3",
        "rate_limit": 100,
        "pagination": true,
        "max_results": 250
      },
      "sync": {
        "schedule": "*/15 * * * *",
        "type": "token"
      },
      "processing": {
        "normalization": true
      },
      "processor": {
        "table_name": "stream_google_calendar"
      },
      "schema": {
        "table_name": "stream_google_calendar",
        "description": "Calendar events and appointments from Google Calendar",
        "columns": [
          {
            "name": "event_id",
            "type": "string",
            "max_length": 200,
            "nullable": false,
            "description": "Google Calendar event ID"
          },
          {
            "name": "calendar_id",
            "type": "string",
            "max_length": 200,
            "nullable": false,
            "description": "Calendar ID (email or calendar identifier)"
          },
          {
            "name": "ical_uid",
            "type": "string",
            "max_length": 200,
            "nullable": true,
            "description": "iCalendar UID for the event"
          },
          {
            "name": "summary",
            "type": "string",
            "max_length": 500,
            "nullable": true,
            "description": "Event title/summary"
          },
          {
            "name": "description",
            "type": "text",
            "nullable": true,
            "description": "Event description"
          },
          {
            "name": "location",
            "type": "string",
            "max_length": 500,
            "nullable": true,
            "description": "Event location"
          },
          {
            "name": "status",
            "type": "string",
            "max_length": 20,
            "nullable": true,
            "description": "Event status (confirmed, tentative, cancelled)"
          },
          {
            "name": "start_time",
            "type": "timestamp",
            "nullable": false,
            "description": "Event start time"
          },
          {
            "name": "end_time",
            "type": "timestamp",
            "nullable": false,
            "description": "Event end time"
          },
          {
            "name": "all_day",
            "type": "boolean",
            "nullable": true,
            "default": false,
            "description": "Whether this is an all-day event"
          },
          {
            "name": "timezone",
            "type": "string",
            "max_length": 50,
            "nullable": true,
            "description": "Event timezone"
          },
          {
            "name": "html_link",
            "type": "string",
            "max_length": 500,
            "nullable": true,
            "description": "Link to event in Google Calendar"
          },
          {
            "name": "created_time",
            "type": "timestamp",
            "nullable": true,
            "description": "When the event was created"
          },
          {
            "name": "updated_time",
            "type": "timestamp",
            "nullable": true,
            "description": "When the event was last updated"
          },
          {
            "name": "event_type",
            "type": "string",
            "max_length": 50,
            "nullable": true,
            "description": "Type of event (default, outOfOffice, focusTime, etc.)"
          },
          {
            "name": "creator",
            "type": "jsonb",
            "nullable": true,
            "description": "Event creator information"
          },
          {
            "name": "organizer",
            "type": "jsonb",
            "nullable": true,
            "description": "Event organizer information"
          },
          {
            "name": "attendees",
            "type": "jsonb",
            "nullable": true,
            "description": "List of attendees with response status"
          },
          {
            "name": "reminders",
            "type": "jsonb",
            "nullable": true,
            "description": "Reminder settings"
          },
          {
            "name": "recurrence",
            "type": "jsonb",
            "nullable": true,
            "description": "Recurrence rules (RRULE)"
          },
          {
            "name": "conference_data",
            "type": "jsonb",
            "nullable": true,
            "description": "Video/phone conference details"
          },
          {
            "name": "full_event",
            "type": "jsonb",
            "nullable": true,
            "description": "Complete event object for unmapped fields"
          }
        ],
        "indexes": [
          {
            "columns": [
              "timestamp"
            ],
            "type": "btree",
            "description": "Primary time-series index"
          },
          {
            "columns": [
              "start_time"
            ],
            "type": "btree",
            "description": "Index for event start times"
          },
          {
            "columns": [
              "event_id"
            ],
            "type": "btree",
            "description": "Index for event lookups"
          }
        ],
        "storage": {
          "strategy": "postgres_only",
          "minio_fields": [],
          "note": "All fields stay in PostgreSQL - calendar events are structured data"
        }
      }
    },
    "ios_healthkit": {
      "name": "ios_healthkit",
      "source": "ios",
      "display_name": "iOS HealthKit",
      "description": "Health data including heart rate, steps, workouts, sleep, and energy",
      "ingestion": {
        "type": "push",
        "batch_type": "array",
        "raw_data_type": "HKSample"
      },
      "sync": {
        "type": "date_range",
        "initial_sync_days": 90,
        "schedule": "*/5 * * * *"
      },
      "processor": {
        "table_name": "stream_ios_healthkit"
      },
      "processing": {
        "deduplication": {
          "strategy": "content_hash"
        },
        "normalization": true,
        "validation": {
          "required_fields": [
            "type",
            "value",
            "timestamp"
          ],
          "ranges": {
            "heart_rate": [
              30,
              250
            ],
            "steps": [
              0,
              100000
            ],
            "active_energy": [
              0,
              10000
            ],
            "heart_rate_variability": [
              0,
              500
            ]
          }
        }
      },
      "storage": {
        "retention_days": 365,
        "compression": "gzip",
        "format": "json"
      },
      "schema": {
        "table_name": "stream_ios_healthkit",
        "description": "Health metrics from iOS HealthKit including heart rate, HRV, steps, and activity",
        "columns": [
          {
            "name": "heart_rate",
            "type": "float",
            "nullable": true,
            "description": "Heart rate in beats per minute"
          },
          {
            "name": "hrv",
            "type": "float",
            "nullable": true,
            "description": "Heart rate variability in milliseconds"
          },
          {
            "name": "activity_type",
            "type": "string",
            "max_length": 50,
            "nullable": true,
            "description": "Type of activity (sleeping, walking, running, etc.)"
          },
          {
            "name": "confidence",
            "type": "float",
            "nullable": true,
            "min": 0.0,
            "max": 1.0,
            "description": "Confidence level of the measurement"
          },
          {
            "name": "steps",
            "type": "integer",
            "nullable": true,
            "description": "Number of steps"
          },
          {
            "name": "active_energy",
            "type": "float",
            "nullable": true,
            "description": "Active energy burned in kcal"
          },
          {
            "name": "sleep_stage",
            "type": "string",
            "max_length": 20,
            "nullable": true,
            "description": "Sleep stage (awake, light, deep, rem)"
          },
          {
            "name": "workout_type",
            "type": "string",
            "max_length": 50,
            "nullable": true,
            "description": "Type of workout activity"
          },
          {
            "name": "workout_duration",
            "type": "integer",
            "nullable": true,
            "description": "Workout duration in seconds"
          },
          {
            "name": "device_name",
            "type": "string",
            "max_length": 100,
            "nullable": true,
            "description": "Name of the device that recorded the data"
          },
          {
            "name": "raw_data",
            "type": "jsonb",
            "nullable": true,
            "description": "Additional fields not mapped to columns"
          }
        ],
        "indexes": [
          {
            "columns": [
              "timestamp"
            ],
            "type": "btree",
            "description": "Primary time-series index"
          }
        ],
        "storage": {
          "strategy": "postgres_only",
          "minio_fields": [],
          "note": "All fields stay in PostgreSQL - health metrics are small structured data",
          "partitioning": {
            "enabled": false,
            "by": "timestamp",
            "interval": "month"
          }
        }
      }
    },
    "ios_location": {
      "name": "ios_location",
      "source": "ios",
      "display_name": "iOS Core Location",
      "description": "Raw CoreLocation data including GPS, altitude, speed, and activity",
      "ingestion": {
        "type": "push",
        "batch_type": "array",
        "raw_data_type": "CLLocation"
      },
      "sync": {
        "type": "none",
        "schedule": "*/5 * * * *"
      },
      "processing": {
        "deduplication": {
          "strategy": "single"
        },
        "normalization": true,
        "validation": {
          "required_fields": [
            "latitude",
            "longitude",
            "timestamp"
          ],
          "ranges": {
            "latitude": [
              -90,
              90
            ],
            "longitude": [
              -180,
              180
            ],
            "altitude": [
              -500,
              9000
            ],
            "speed": [
              0,
              500
            ]
          }
        }
      },
      "processor": {
        "table_name": "stream_ios_location"
      },
      "storage": {
        "retention_days": 90,
        "compression": "gzip",
        "format": "json"
      },
      "schema": {
        "table_name": "stream_ios_location",
        "description": "GPS and location data from iOS Core Location",
        "columns": [
          {
            "name": "latitude",
            "type": "float",
            "nullable": false,
            "description": "Latitude coordinate"
          },
          {
            "name": "longitude",
            "type": "float",
            "nullable": false,
            "description": "Longitude coordinate"
          },
          {
            "name": "altitude",
            "type": "float",
            "nullable": true,
            "description": "Altitude in meters"
          },
          {
            "name": "horizontal_accuracy",
            "type": "float",
            "nullable": true,
            "description": "Horizontal accuracy in meters"
          },
          {
            "name": "vertical_accuracy",
            "type": "float",
            "nullable": true,
            "description": "Vertical accuracy in meters"
          },
          {
            "name": "speed",
            "type": "float",
            "nullable": true,
            "description": "Speed in meters per second"
          },
          {
            "name": "course",
            "type": "float",
            "nullable": true,
            "description": "Course/heading in degrees from true north"
          },
          {
            "name": "floor",
            "type": "integer",
            "nullable": true,
            "description": "Floor level in building"
          },
          {
            "name": "activity_type",
            "type": "string",
            "max_length": 50,
            "nullable": true,
            "description": "Type of activity (stationary, walking, running, automotive, etc.)"
          },
          {
            "name": "address",
            "type": "string",
            "max_length": 500,
            "nullable": true,
            "description": "Reverse geocoded address"
          },
          {
            "name": "place_name",
            "type": "string",
            "max_length": 200,
            "nullable": true,
            "description": "Name of the place/venue"
          },
          {
            "name": "raw_data",
            "type": "jsonb",
            "nullable": true,
            "description": "Additional location metadata"
          }
        ],
        "indexes": [
          {
            "columns": [
              "timestamp"
            ],
            "type": "btree",
            "description": "Primary time-series index"
          }
        ],
        "storage": {
          "strategy": "postgres_only",
          "minio_fields": [],
          "note": "All fields stay in PostgreSQL - no binary data in location streams"
        }
      }
    },
    "ios_mic": {
      "name": "ios_mic",
      "source": "ios",
      "display_name": "iOS Microphone Audio",
      "description": "Audio chunks for transcription and analysis",
      "ingestion": {
        "type": "push",
        "batch_type": "chunks",
        "raw_data_type": "AudioChunk",
        "chunk_duration_ms": 5000
      },
      "sync": {
        "type": "none",
        "schedule": "*/5 * * * *"
      },
      "processor": {
        "table_name": "stream_ios_mic",
        "minio_fields": [
          "audio_data"
        ]
      },
      "processing": {
        "deduplication": false,
        "normalization": true,
        "validation": {
          "required_fields": [
            "audio_data",
            "timestamp",
            "sample_rate"
          ],
          "audio": {
            "sample_rate": 16000,
            "encoding": "pcm16",
            "channels": 1
          }
        }
      },
      "storage": {
        "retention_days": 7,
        "compression": "opus",
        "format": "binary"
      },
      "schema": {
        "table_name": "stream_ios_mic",
        "description": "Audio metadata and transcriptions from iOS microphone",
        "columns": [
          {
            "name": "recording_id",
            "type": "string",
            "max_length": 100,
            "nullable": false,
            "description": "Unique identifier for the recording"
          },
          {
            "name": "timestamp_start",
            "type": "timestamp",
            "nullable": false,
            "description": "Start time of the recording"
          },
          {
            "name": "timestamp_end",
            "type": "timestamp",
            "nullable": false,
            "description": "End time of the recording"
          },
          {
            "name": "duration",
            "type": "integer",
            "nullable": false,
            "description": "Duration in milliseconds"
          },
          {
            "name": "overlap_duration",
            "type": "float",
            "nullable": true,
            "description": "Overlap duration with previous recording in seconds"
          },
          {
            "name": "audio_format",
            "type": "string",
            "max_length": 10,
            "nullable": true,
            "description": "Audio format (wav, mp3, etc.)"
          },
          {
            "name": "sample_rate",
            "type": "integer",
            "nullable": true,
            "description": "Sample rate in Hz"
          },
          {
            "name": "audio_level",
            "type": "float",
            "nullable": true,
            "description": "Average audio level in dB"
          },
          {
            "name": "peak_level",
            "type": "float",
            "nullable": true,
            "description": "Peak audio level in dB"
          },
          {
            "name": "transcription_text",
            "type": "text",
            "nullable": true,
            "description": "Transcribed text from audio"
          },
          {
            "name": "transcription_confidence",
            "type": "float",
            "nullable": true,
            "min": 0.0,
            "max": 1.0,
            "description": "Confidence score of transcription"
          },
          {
            "name": "language",
            "type": "string",
            "max_length": 10,
            "nullable": true,
            "description": "Detected language code"
          },
          {
            "name": "minio_path",
            "type": "string",
            "max_length": 500,
            "nullable": true,
            "description": "Path to audio file in MinIO storage"
          },
          {
            "name": "file_size",
            "type": "integer",
            "nullable": true,
            "description": "Size of audio file in bytes"
          },
          {
            "name": "raw_data",
            "type": "jsonb",
            "nullable": true,
            "description": "Additional metadata"
          }
        ],
        "indexes": [
          {
            "columns": [
              "timestamp"
            ],
            "type": "btree",
            "description": "Primary time-series index using timestamp field"
          },
          {
            "columns": [
              "timestamp_start"
            ],
            "type": "btree",
            "description": "Index for start time queries"
          }
        ],
        "storage": {
          "strategy": "hybrid",
          "minio_fields": [
            "audio_data"
          ],
          "reference_pattern": "assets/{stream_name}/{year}/{month}/{day}/{field_name}_{uuid}.{ext}",
          "note": "Deterministic storage - audio_data always in MinIO, everything else in PostgreSQL"
        }
      }
    },
    "mac_apps": {
      "name": "mac_apps",
      "source": "mac",
      "display_name": "Mac Applications",
      "description": "Application focus and usage events",
      "ingestion": {
        "type": "push",
        "batch_type": "events",
        "raw_data_type": "AppEvent"
      },
      "processor": {
        "table_name": "stream_mac_apps"
      },
      "processing": {
        "deduplication": {
          "strategy": "single"
        },
        "normalization": true,
        "validation": {
          "required_fields": [
            "app_name",
            "bundle_id",
            "timestamp",
            "event_type"
          ],
          "event_types": [
            "focus_gained",
            "focus_lost",
            "launch",
            "quit"
          ]
        }
      },
      "storage": {
        "retention_days": 30,
        "compression": "gzip",
        "format": "json"
      },
      "schema": {
        "table_name": "stream_mac_apps",
        "description": "Application focus events from macOS",
        "columns": [
          {
            "name": "app_name",
            "type": "string",
            "max_length": 200,
            "nullable": false,
            "description": "Application name"
          },
          {
            "name": "bundle_id",
            "type": "string",
            "max_length": 200,
            "nullable": true,
            "description": "macOS bundle identifier"
          },
          {
            "name": "event_type",
            "type": "string",
            "max_length": 50,
            "nullable": false,
            "description": "Event type (focus_gained, focus_lost, launch, quit)"
          }
        ],
        "indexes": [
          {
            "columns": [
              "timestamp"
            ],
            "type": "btree",
            "description": "Primary time-series index"
          },
          {
            "columns": [
              "app_name",
              "timestamp"
            ],
            "type": "btree",
            "description": "Index for app-specific queries"
          },
          {
            "columns": [
              "event_type",
              "timestamp"
            ],
            "type": "btree",
            "description": "Index for event type queries"
          }
        ],
        "storage": {
          "strategy": "postgres_only",
          "minio_fields": [],
          "note": "All fields stay in PostgreSQL - app usage data is small and queryable"
        }
      }
    },
    "mac_messages": {
      "name": "mac_messages",
      "source": "mac",
      "display_name": "Messages",
      "description": "iMessage and SMS conversation history",
      "ingestion": {
        "type": "push",
        "batch_type": "events",
        "raw_data_type": "MessageEvent"
      },
      "processor": {
        "table_name": "stream_mac_messages"
      },
      "processing": {
        "deduplication": {
          "strategy": "unique_key",
          "key": [
            "message_id"
          ]
        },
        "normalization": true,
        "validation": {
          "required_fields": [
            "message_id",
            "chat_id",
            "date",
            "text"
          ]
        }
      },
      "storage": {
        "retention_days": 365,
        "compression": "gzip",
        "format": "json"
      },
      "schema": {
        "table_name": "stream_mac_messages",
        "description": "iMessage and SMS messages from macOS",
        "columns": [
          {
            "name": "message_id",
            "type": "string",
            "max_length": 200,
            "nullable": false,
            "description": "Unique message identifier (GUID)"
          },
          {
            "name": "chat_id",
            "type": "string",
            "max_length": 200,
            "nullable": false,
            "description": "Chat/conversation identifier"
          },
          {
            "name": "handle_id",
            "type": "string",
            "max_length": 200,
            "nullable": true,
            "description": "Contact handle (phone/email)"
          },
          {
            "name": "text",
            "type": "text",
            "nullable": true,
            "description": "Message content"
          },
          {
            "name": "service",
            "type": "string",
            "max_length": 50,
            "nullable": true,
            "description": "Service type (iMessage, SMS)"
          },
          {
            "name": "is_from_me",
            "type": "boolean",
            "nullable": false,
            "default": false,
            "description": "Whether message was sent by user"
          },
          {
            "name": "date",
            "type": "timestamp",
            "nullable": false,
            "description": "Message timestamp"
          },
          {
            "name": "date_read",
            "type": "timestamp",
            "nullable": true,
            "description": "When message was read"
          },
          {
            "name": "date_delivered",
            "type": "timestamp",
            "nullable": true,
            "description": "When message was delivered"
          },
          {
            "name": "is_read",
            "type": "boolean",
            "nullable": true,
            "default": false,
            "description": "Whether message has been read"
          },
          {
            "name": "is_delivered",
            "type": "boolean",
            "nullable": true,
            "default": false,
            "description": "Whether message was delivered"
          },
          {
            "name": "is_sent",
            "type": "boolean",
            "nullable": true,
            "default": false,
            "description": "Whether message was sent successfully"
          },
          {
            "name": "cache_has_attachments",
            "type": "boolean",
            "nullable": true,
            "default": false,
            "description": "Whether message has attachments"
          },
          {
            "name": "attachment_count",
            "type": "integer",
            "nullable": true,
            "description": "Number of attachments"
          },
          {
            "name": "attachment_info",
            "type": "jsonb",
            "nullable": true,
            "description": "Attachment metadata (filenames, types, sizes)"
          },
          {
            "name": "group_title",
            "type": "string",
            "max_length": 500,
            "nullable": true,
            "description": "Group chat name if applicable"
          },
          {
            "name": "associated_message_guid",
            "type": "string",
            "max_length": 200,
            "nullable": true,
            "description": "Related message ID (for replies/reactions)"
          },
          {
            "name": "associated_message_type",
            "type": "integer",
            "nullable": true,
            "description": "Type of association (reply, reaction, etc)"
          },
          {
            "name": "expressive_send_style_id",
            "type": "string",
            "max_length": 100,
            "nullable": true,
            "description": "Message effect style (invisible ink, etc)"
          },
          {
            "name": "raw_data",
            "type": "jsonb",
            "nullable": true,
            "description": "Additional unmapped fields"
          }
        ],
        "indexes": [
          {
            "columns": [
              "timestamp"
            ],
            "type": "btree",
            "description": "Primary time-series index"
          },
          {
            "columns": [
              "message_id"
            ],
            "type": "btree",
            "unique": true,
            "description": "Unique message ID index"
          },
          {
            "columns": [
              "chat_id",
              "date"
            ],
            "type": "btree",
            "description": "Index for chat-specific queries"
          },
          {
            "columns": [
              "is_from_me",
              "date"
            ],
            "type": "btree",
            "description": "Index for sent/received filtering"
          },
          {
            "columns": [
              "source_id",
              "message_id"
            ],
            "type": "btree",
            "unique": true,
            "description": "Composite unique constraint for deduplication"
          }
        ],
        "storage": {
          "strategy": "hybrid",
          "minio_fields": [
            "attachment_info",
            "raw_data"
          ],
          "note": "Message text stays in PostgreSQL, attachments metadata in MinIO"
        }
      }
    },
    "notion_pages": {
      "name": "notion_pages",
      "source": "notion",
      "display_name": "Notion Pages",
      "description": "Pages and databases from Notion workspace",
      "ingestion": {
        "type": "pull",
        "batch_type": "pages",
        "raw_data_type": "NotionPage"
      },
      "processor": {
        "table_name": "stream_notion_pages",
        "minio_fields": [
          "blocks",
          "attachments"
        ]
      },
      "api": {
        "endpoint": "https://api.notion.com/v1",
        "rate_limit": 3,
        "pagination": true,
        "page_size": 100
      },
      "sync": {
        "schedule": "*/30 * * * *",
        "type": "token"
      },
      "processing": {
        "extract_text": true,
        "extract_metadata": true,
        "versioning": true,
        "deduplication": {
          "key": [
            "page_id",
            "last_edited_time"
          ],
          "strategy": "content_hash"
        },
        "normalization": true,
        "validation": {
          "required_fields": [
            "id",
            "object",
            "created_time",
            "last_edited_time"
          ]
        }
      },
      "schema": {
        "table_name": "stream_notion_pages",
        "description": "Pages and databases from Notion workspace",
        "columns": [
          {
            "name": "page_id",
            "type": "string",
            "max_length": 100,
            "nullable": false,
            "description": "Notion page UUID"
          },
          {
            "name": "parent_id",
            "type": "string",
            "max_length": 100,
            "nullable": true,
            "description": "Parent page or workspace ID"
          },
          {
            "name": "parent_type",
            "type": "string",
            "max_length": 20,
            "nullable": true,
            "description": "Type of parent (page, database, workspace)"
          },
          {
            "name": "title",
            "type": "string",
            "max_length": 500,
            "nullable": true,
            "description": "Page title"
          },
          {
            "name": "object_type",
            "type": "string",
            "max_length": 20,
            "nullable": true,
            "description": "Object type (page, database)"
          },
          {
            "name": "archived",
            "type": "boolean",
            "nullable": true,
            "default": false,
            "description": "Whether the page is archived"
          },
          {
            "name": "url",
            "type": "string",
            "max_length": 500,
            "nullable": true,
            "description": "Public URL if shared"
          },
          {
            "name": "created_time",
            "type": "timestamp",
            "nullable": true,
            "description": "When the page was created in Notion"
          },
          {
            "name": "created_by",
            "type": "string",
            "max_length": 100,
            "nullable": true,
            "description": "User ID who created the page"
          },
          {
            "name": "last_edited_time",
            "type": "timestamp",
            "nullable": true,
            "description": "When the page was last edited in Notion"
          },
          {
            "name": "last_edited_by",
            "type": "string",
            "max_length": 100,
            "nullable": true,
            "description": "User ID who last edited the page"
          },
          {
            "name": "content_text",
            "type": "text",
            "nullable": true,
            "description": "Extracted plain text content"
          },
          {
            "name": "content_markdown",
            "type": "text",
            "nullable": true,
            "description": "Content converted to Markdown"
          },
          {
            "name": "properties",
            "type": "jsonb",
            "nullable": true,
            "description": "Database properties and values"
          },
          {
            "name": "icon",
            "type": "jsonb",
            "nullable": true,
            "description": "Page icon (emoji or image)"
          },
          {
            "name": "cover",
            "type": "jsonb",
            "nullable": true,
            "description": "Page cover image"
          },
          {
            "name": "parent",
            "type": "jsonb",
            "nullable": true,
            "description": "Full parent relationship data"
          },
          {
            "name": "blocks",
            "type": "jsonb",
            "nullable": true,
            "description": "Page content blocks (if small)"
          },
          {
            "name": "minio_path",
            "type": "string",
            "max_length": 500,
            "nullable": true,
            "description": "Path to full content in MinIO (if large)"
          },
          {
            "name": "full_page",
            "type": "jsonb",
            "nullable": true,
            "description": "Complete page object for unmapped fields"
          }
        ],
        "indexes": [
          {
            "columns": [
              "timestamp"
            ],
            "type": "btree",
            "description": "Primary time-series index"
          },
          {
            "columns": [
              "last_edited_time"
            ],
            "type": "btree",
            "description": "Index for edit time queries"
          },
          {
            "columns": [
              "page_id"
            ],
            "type": "btree",
            "description": "Index for page lookups"
          }
        ],
        "storage": {
          "strategy": "hybrid",
          "minio_fields": [
            "blocks",
            "attachments"
          ],
          "reference_pattern": "assets/{stream_name}/{year}/{month}/{day}/{page_id}_{field_name}.json",
          "note": "Deterministic storage - large content/attachments in MinIO, metadata in PostgreSQL"
        }
      }
    },
    "strava_activities": {
      "name": "strava_activities",
      "source": "strava",
      "display_name": "Activities",
      "description": "Running, cycling, and other fitness activities",
      "ingestion": {
        "type": "pull",
        "batch_type": "activities",
        "raw_data_type": "StravaActivity"
      },
      "api": {
        "endpoint": "https://www.strava.com/api/v3",
        "rate_limit": 100,
        "pagination": true,
        "max_results": 30
      },
      "sync": {
        "schedule": "*/30 * * * *",
        "type": "date_range",
        "initial_sync_days": 90
      },
      "processing": {
        "normalization": true
      },
      "processor": {
        "table_name": "stream_strava_activities"
      },
      "schema": {
        "table_name": "stream_strava_activities",
        "description": "Fitness activities and workouts from Strava",
        "columns": [
          {
            "name": "activity_id",
            "type": "bigint",
            "nullable": false,
            "description": "Strava activity ID"
          },
          {
            "name": "external_id",
            "type": "string",
            "max_length": 200,
            "nullable": true,
            "description": "External ID from device/app"
          },
          {
            "name": "name",
            "type": "string",
            "max_length": 500,
            "nullable": true,
            "description": "Activity name/title"
          },
          {
            "name": "type",
            "type": "string",
            "max_length": 50,
            "nullable": true,
            "description": "Activity type (Run, Ride, Swim, etc.)"
          },
          {
            "name": "sport_type",
            "type": "string",
            "max_length": 50,
            "nullable": true,
            "description": "Specific sport type"
          },
          {
            "name": "workout_type",
            "type": "integer",
            "nullable": true,
            "description": "Workout type code"
          },
          {
            "name": "distance",
            "type": "float",
            "nullable": true,
            "description": "Distance in meters"
          },
          {
            "name": "moving_time",
            "type": "integer",
            "nullable": true,
            "description": "Moving time in seconds"
          },
          {
            "name": "elapsed_time",
            "type": "integer",
            "nullable": true,
            "description": "Total elapsed time in seconds"
          },
          {
            "name": "total_elevation_gain",
            "type": "float",
            "nullable": true,
            "description": "Total elevation gain in meters"
          },
          {
            "name": "elev_high",
            "type": "float",
            "nullable": true,
            "description": "Highest elevation in meters"
          },
          {
            "name": "elev_low",
            "type": "float",
            "nullable": true,
            "description": "Lowest elevation in meters"
          },
          {
            "name": "average_speed",
            "type": "float",
            "nullable": true,
            "description": "Average speed in meters per second"
          },
          {
            "name": "max_speed",
            "type": "float",
            "nullable": true,
            "description": "Maximum speed in meters per second"
          },
          {
            "name": "average_heartrate",
            "type": "float",
            "nullable": true,
            "description": "Average heart rate in bpm"
          },
          {
            "name": "max_heartrate",
            "type": "float",
            "nullable": true,
            "description": "Maximum heart rate in bpm"
          },
          {
            "name": "average_cadence",
            "type": "float",
            "nullable": true,
            "description": "Average cadence"
          },
          {
            "name": "average_watts",
            "type": "float",
            "nullable": true,
            "description": "Average power in watts"
          },
          {
            "name": "kilojoules",
            "type": "float",
            "nullable": true,
            "description": "Total work in kilojoules"
          },
          {
            "name": "start_date",
            "type": "timestamp",
            "nullable": false,
            "description": "Activity start time (UTC)"
          },
          {
            "name": "start_date_local",
            "type": "timestamp",
            "nullable": true,
            "description": "Activity start time (local)"
          },
          {
            "name": "timezone",
            "type": "string",
            "max_length": 50,
            "nullable": true,
            "description": "Timezone of the activity"
          },
          {
            "name": "achievement_count",
            "type": "integer",
            "nullable": true,
            "description": "Number of achievements"
          },
          {
            "name": "kudos_count",
            "type": "integer",
            "nullable": true,
            "description": "Number of kudos received"
          },
          {
            "name": "comment_count",
            "type": "integer",
            "nullable": true,
            "description": "Number of comments"
          },
          {
            "name": "start_latlng",
            "type": "jsonb",
            "nullable": true,
            "description": "Starting coordinates [lat, lng]"
          },
          {
            "name": "end_latlng",
            "type": "jsonb",
            "nullable": true,
            "description": "Ending coordinates [lat, lng]"
          },
          {
            "name": "map",
            "type": "jsonb",
            "nullable": true,
            "description": "Map polyline and summary"
          },
          {
            "name": "splits_metric",
            "type": "jsonb",
            "nullable": true,
            "description": "Kilometer splits"
          },
          {
            "name": "splits_standard",
            "type": "jsonb",
            "nullable": true,
            "description": "Mile splits"
          },
          {
            "name": "segment_efforts",
            "type": "jsonb",
            "nullable": true,
            "description": "Segment efforts within activity"
          },
          {
            "name": "gear",
            "type": "jsonb",
            "nullable": true,
            "description": "Equipment used"
          },
          {
            "name": "photos",
            "type": "jsonb",
            "nullable": true,
            "description": "Activity photos metadata"
          },
          {
            "name": "stats",
            "type": "jsonb",
            "nullable": true,
            "description": "Additional statistics"
          },
          {
            "name": "full_activity",
            "type": "jsonb",
            "nullable": true,
            "description": "Complete activity object for unmapped fields"
          }
        ],
        "indexes": [
          {
            "columns": [
              "timestamp"
            ],
            "type": "btree",
            "description": "Primary time-series index"
          },
          {
            "columns": [
              "start_date"
            ],
            "type": "btree",
            "description": "Index for activity start times"
          },
          {
            "columns": [
              "activity_id"
            ],
            "type": "btree",
            "description": "Index for activity lookups"
          }
        ],
        "storage": {
          "strategy": "postgres_only",
          "minio_fields": [],
          "note": "All fields stay in PostgreSQL - Strava data is metrics and JSON"
        }
      }
    }
  },
  "metadata": {
    "generated_at": "2025-08-25T22:09:36.569561Z",
    "version": "2.0.0"
  }
}
