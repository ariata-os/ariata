#!/usr/bin/env tsx
import * as dotenv from 'dotenv';
import * as path from 'path';
import { fileURLToPath } from 'url';
import * as yaml from 'js-yaml';
import * as fs from 'fs';
import * as crypto from 'crypto';

// Make crypto globally available for MinIO
if (typeof globalThis.crypto === 'undefined') {
	globalThis.crypto = crypto as any;
}

// Load environment variables BEFORE importing db client
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '../../..');
dotenv.config({ path: path.join(rootDir, '.env') });

// Set DATABASE_URL to use localhost for local development
if (!process.env.DATABASE_URL) {
  process.env.DATABASE_URL = 'postgresql://ariata_user:ariata_password@localhost:5432/ariata';
}

import { db } from '../src/lib/db/client';
import { sourceConfigs, streamConfigs } from '../src/lib/db/schema';
import { eq } from 'drizzle-orm';

async function seedDatabase() {
  console.log('🌱 Seeding development database...');
  
  // Load registry from sources directory (mounted at /sources in Docker)
  const registryPath = fs.existsSync('/sources/_generated_registry.yaml') 
    ? '/sources/_generated_registry.yaml'
    : path.join(rootDir, 'sources', '_generated_registry.yaml');
  const registryContent = fs.readFileSync(registryPath, 'utf-8');
  const registry = yaml.load(registryContent) as any;
  
  // Create default user for single-user app
  console.log('👤 Creating default user...');
  const { users } = await import('../src/lib/db/schema');
  
  // Check if user already exists
  const existingUser = await db
    .select()
    .from(users)
    .where(eq(users.email, 'user@ariata.app'))
    .limit(1);
  
  if (existingUser.length === 0) {
    await db.insert(users).values({
      firstName: 'Default',
      lastName: 'User',
      email: 'user@ariata.app',
      timezone: 'America/Chicago'
    });
    console.log('  ✅ Created default user');
  } else {
    console.log('  ℹ️  Default user already exists');
  }

  // Seed source configs
  for (const [sourceName, sourceData] of Object.entries(registry.sources)) {
    const existing = await db
      .select()
      .from(sourceConfigs)
      .where(eq(sourceConfigs.name, sourceName))
      .limit(1);
    
    // Always update or insert to ensure latest config
    if (true) { // Changed from: if (existing.length === 0)
      // Collect all required scopes from streams_config
      const streamScopes: string[] = [];
      const streamsConfig = (sourceData as any).streams_config;
      if (streamsConfig && Array.isArray(streamsConfig)) {
        for (const streamConfig of streamsConfig) {
          if (streamConfig.required_scopes && Array.isArray(streamConfig.required_scopes)) {
            streamScopes.push(...streamConfig.required_scopes);
          }
        }
      }
      
      const configData = {
        name: sourceName,
        company: sourceData.company,
        platform: sourceData.platform || 'cloud',
        deviceType: sourceData.device_type || undefined,
        displayName: sourceData.display_name,
        description: sourceData.description,
        icon: sourceData.icon,
        video: sourceData.video || null,
        authType: sourceData.auth?.type || 'oauth2',
        defaultSyncSchedule: sourceData.sync?.default_schedule || '0 * * * *', // Default hourly
        minSyncFrequency: sourceData.sync?.min_frequency || 300, // 5 minutes
        maxSyncFrequency: sourceData.sync?.max_frequency || 86400, // 24 hours
        oauthConfig: {
          ...sourceData.auth, // Include the full auth object to preserve device_setup and other auth configs
          requiredScopes: [...new Set(streamScopes)] // Deduplicate scopes (this will override if auth.required_scopes exists)
        },
        syncConfig: sourceData.sync,
      };
      
      if (existing.length > 0) {
        // Update existing config
        await db.update(sourceConfigs)
          .set(configData)
          .where(eq(sourceConfigs.name, sourceName));
        console.log(`✅ Updated source config: ${sourceName}`);
      } else {
        // Insert new config
        await db.insert(sourceConfigs).values(configData);
        console.log(`✅ Created source config: ${sourceName}`);
      }
    }
  }

  // Seed stream configs
  for (const [streamName, streamData] of Object.entries(registry.streams)) {
    const existing = await db
      .select()
      .from(streamConfigs)
      .where(eq(streamConfigs.streamName, streamName))
      .limit(1);
    
    if (existing.length === 0) {
      // Use ingestion type from registry data
      const ingestionType = streamData.ingestion?.type || 'pull';
      
      // Extract cron schedule from sync config if available
      const cronSchedule = streamData.sync?.schedule || 
                          (ingestionType === 'pull' ? '0 */6 * * *' : null);
      
      await db.insert(streamConfigs).values({
        streamName: streamName,
        sourceName: streamData.source,
        displayName: streamData.display_name,
        description: streamData.description,
        ingestionType: ingestionType,
        status: 'active',
        cronSchedule: cronSchedule,
        settings: {
          ingestion: streamData.ingestion,
          sync: streamData.sync,
          processing: streamData.processing,
          storage: streamData.storage,
          output_type: streamData.output_type,
        },
      });
      console.log(`✅ Created stream config: ${streamName}`);
    }
  }


  // Create test source instances for development
  console.log('🧪 Creating test source instances...');
  
  // Import sources table
  const { sources } = await import('../src/lib/db/schema');
  
  // Create test iOS device for development
  const testIosDevice = {
    id: '00000000-0000-0000-0000-000000000101',
    sourceName: 'ios',
    instanceName: "Test iPhone (Demo)",
    status: 'paused' as const,  // Set to paused to prevent sync attempts with test data
    deviceId: 'dev_iphone_001',
    deviceToken: 'DEV_TOKEN_001',
    deviceType: 'ios' as const,
    deviceLastSeen: new Date(),
    sourceMetadata: {
      os_version: '17.0',
      model: 'iPhone 15 Pro',
      app_version: '1.0.0',
      isTest: true  // Mark as test source
    }
  };
  
  const existingIos = await db
    .select()
    .from(sources)
    .where(eq(sources.deviceId, 'dev_iphone_001'))
    .limit(1);
  
  if (existingIos.length === 0) {
    await db.insert(sources).values(testIosDevice);
    console.log('  ✅ Created test iOS device');
  } else {
    console.log('  ℹ️  Test iOS device already exists');
  }
  
  // Create test Google Calendar source
  const testGoogleSource = {
    id: '00000000-0000-0000-0000-000000000102',
    sourceName: 'google',
    instanceName: "Test Google Account (Demo)",
    status: 'paused' as const,  // Set to paused to prevent sync attempts with fake OAuth tokens
    deviceId: 'test_google_account',
    deviceToken: 'DEV_TOKEN_GOOGLE_001',  // Add device token for test ingestion
    oauthAccessToken: 'test_access_token_google',
    oauthRefreshToken: 'test_refresh_token_google',
    oauthExpiresAt: new Date(Date.now() + 3600000), // 1 hour from now
    scopes: [
      'https://www.googleapis.com/auth/calendar.readonly',
      'https://www.googleapis.com/auth/calendar.events.readonly'
    ],
    sourceMetadata: {
      email: 'testuser@example.com',
      name: 'Test User',
      isTest: true  // Mark as test source
    }
  };
  
  const existingGoogle = await db
    .select()
    .from(sources)
    .where(eq(sources.deviceId, 'test_google_account'))
    .limit(1);
  
  if (existingGoogle.length === 0) {
    await db.insert(sources).values(testGoogleSource);
    console.log('  ✅ Created test Google source');
  } else {
    console.log('  ℹ️  Test Google source already exists');
  }
  
  // Create stream instances for test sources
  console.log('📊 Creating stream instances for test sources...');
  const { streams } = await import('../src/lib/db/schema');
  
  // Create iOS location stream
  const [iosLocationConfig] = await db
    .select()
    .from(streamConfigs)
    .where(eq(streamConfigs.streamName, 'ios_location'))
    .limit(1);
  
  if (iosLocationConfig) {
    const existingLocationStream = await db
      .select()
      .from(streams)
      .where(eq(streams.sourceId, testIosDevice.id))
      .where(eq(streams.streamConfigId, iosLocationConfig.id))
      .limit(1);
    
    if (existingLocationStream.length === 0) {
      await db.insert(streams).values({
        sourceId: testIosDevice.id,
        streamConfigId: iosLocationConfig.id,
        enabled: true
      });
      console.log('  ✅ Created iOS location stream');
    }
  }
  
  // Create iOS mic stream with disabled transcription
  const [iosMicConfig] = await db
    .select()
    .from(streamConfigs)
    .where(eq(streamConfigs.streamName, 'ios_mic'))
    .limit(1);
  
  if (iosMicConfig) {
    const existingMicStream = await db
      .select()
      .from(streams)
      .where(eq(streams.sourceId, testIosDevice.id))
      .where(eq(streams.streamConfigId, iosMicConfig.id))
      .limit(1);
    
    if (existingMicStream.length === 0) {
      await db.insert(streams).values({
        sourceId: testIosDevice.id,
        streamConfigId: iosMicConfig.id,
        enabled: true,
        settings: {
          transcriptionEnabled: false // Disable transcription for test data
        }
      });
      console.log('  ✅ Created iOS mic stream (transcription disabled)');
    }
  }
  
  // Create Google Calendar stream
  const [googleCalendarConfig] = await db
    .select()
    .from(streamConfigs)
    .where(eq(streamConfigs.streamName, 'google_calendar'))
    .limit(1);
  
  if (googleCalendarConfig) {
    const existingCalendarStream = await db
      .select()
      .from(streams)
      .where(eq(streams.sourceId, testGoogleSource.id))
      .where(eq(streams.streamConfigId, googleCalendarConfig.id))
      .limit(1);
    
    if (existingCalendarStream.length === 0) {
      await db.insert(streams).values({
        sourceId: testGoogleSource.id,
        streamConfigId: googleCalendarConfig.id,
        enabled: true
      });
      console.log('  ✅ Created Google Calendar stream');
    }
  }
  
  console.log('✅ Database seeding complete!');
  
  // Seed test data via ingest endpoint
  await seedTestData();
}

async function seedTestData() {
  console.log('\n📤 Seeding test data via ingest endpoint...');
  
  const testDir = fs.existsSync('/app/tests') ? '/app/tests' : path.join(rootDir, 'tests');
  // Force IPv4 to avoid connection issues with ::1
  let baseUrl = process.env.FRONTEND_URL || 'http://127.0.0.1:3000';
  // Replace localhost with 127.0.0.1 to force IPv4
  baseUrl = baseUrl.replace('://localhost', '://127.0.0.1');
  
  // Get device tokens from test sources
  const { sources } = await import('../src/lib/db/schema');
  
  const [iosDevice] = await db
    .select()
    .from(sources)
    .where(eq(sources.deviceId, 'dev_iphone_001'))
    .limit(1);
  
  const [googleSource] = await db
    .select()
    .from(sources)
    .where(eq(sources.deviceId, 'test_google_account'))
    .limit(1);
  
  if (!iosDevice || !googleSource) {
    console.log('  ⚠️  Test sources not found, skipping data seeding');
    return;
  }
  
  console.log(`  ✓ Found iOS device token: ${iosDevice.deviceToken?.substring(0, 8)}...`);
  console.log(`  ✓ Found Google device token: ${googleSource.deviceToken?.substring(0, 8)}...`)
  
  // Seed iOS Location data via ingestion endpoint
  try {
    const iosLocationFile = path.join(testDir, 'test_data_ios_location.json');
    if (fs.existsSync(iosLocationFile)) {
      console.log('  📍 Processing iOS Location data...');
      const testData = JSON.parse(fs.readFileSync(iosLocationFile, 'utf-8'));
      
      // Prepare payload for ingestion
      const payload = {
        stream_name: 'ios_location',
        data: testData.data,
        batch_metadata: {
          total_points: testData.data.length,
          date: '2025-07-01',
          start_time: testData.data[0].timestamp,
          end_time: testData.data[testData.data.length - 1].timestamp
        }
      };
      
      // Send to ingestion endpoint
      const response = await fetch(`${baseUrl}/api/ingest`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Device-Token': iosDevice.deviceToken || ''
        },
        body: JSON.stringify(payload)
      });
      
      if (response.ok) {
        const result = await response.json();
        console.log(`  ✅ iOS Location: Queued ${testData.data.length} location points for processing`);
      } else {
        const errorText = await response.text();
        console.log(`  ❌ iOS Location: Failed to ingest (${response.status}): ${errorText}`);
      }
    } else {
      console.log('  ⚠️  iOS Location test data not found');
    }
  } catch (error) {
    console.log(`  ❌ iOS Location error:`, error);
  }
  
  // Seed iOS Audio data via ingestion endpoint
  try {
    const iosAudioFile = path.join(testDir, 'test_data_ios_audio.json.gz');
    const iosAudioFileUncompressed = path.join(testDir, 'test_data_ios_audio.json');
    
    console.log('  🎤 Processing iOS Audio data...');
    
    // Try compressed first, fallback to uncompressed
    let testData;
    let useGzip = false;
    if (fs.existsSync(iosAudioFile)) {
      const zlib = await import('zlib');
      const gunzip = zlib.gunzipSync;
      const compressedData = fs.readFileSync(iosAudioFile);
      testData = JSON.parse(gunzip(compressedData).toString());
      useGzip = true;
    } else if (fs.existsSync(iosAudioFileUncompressed)) {
      testData = JSON.parse(fs.readFileSync(iosAudioFileUncompressed, 'utf-8'));
    }
    
    if (testData) {
      // Prepare payload for ingestion - iOS mic uses 'data' field for chunks
      const payload = {
        stream_name: 'ios_mic',
        data: testData.data, // Keep as 'data' - the processor will handle it
        batch_metadata: testData.batch_metadata || {
          total_chunks: testData.data.length,
          app_version: '1.0'
        }
      };
      
      // For large audio data, send as gzipped to avoid encoding issues
      const zlib = await import('zlib');
      const gzipData = zlib.gzipSync(JSON.stringify(payload));
      
      // Send to ingestion endpoint with gzip encoding
      const response = await fetch(`${baseUrl}/api/ingest`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Content-Encoding': 'gzip',
          'X-Device-Token': iosDevice.deviceToken || ''
        },
        body: gzipData
      });
      
      if (response.ok) {
        const result = await response.json();
        console.log(`  ✅ iOS Audio: Queued ${testData.data.length} audio chunks for processing (sent as gzip)`);
      } else {
        const errorText = await response.text();
        console.log(`  ❌ iOS Audio: Failed to ingest (${response.status}): ${errorText}`);
      }
    } else {
      console.log('  ⚠️  iOS Audio test data not found');
    }
  } catch (error) {
    console.log(`  ❌ iOS Audio error:`, error);
  }
  
  // Seed Google Calendar data via ingestion endpoint
  try {
    const googleCalendarFile = path.join(testDir, 'test_data_google_calendar.json');
    if (fs.existsSync(googleCalendarFile)) {
      console.log('  📅 Processing Google Calendar data...');
      const testData = JSON.parse(fs.readFileSync(googleCalendarFile, 'utf-8'));
      
      // Google Calendar expects 'data' field for events
      const payload = {
        stream_name: 'google_calendar',
        data: testData.events, // Use 'data' field consistently
        batch_metadata: {
          total_records: testData.events.length,
          date: '2025-07-01',
          sync_type: 'full'
        }
      };
      
      // Send to ingestion endpoint with Google device token
      const response = await fetch(`${baseUrl}/api/ingest`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Device-Token': googleSource.deviceToken || ''
        },
        body: JSON.stringify(payload)
      });
      
      if (response.ok) {
        const result = await response.json();
        console.log(`  ✅ Google Calendar: Queued ${testData.events.length} events for processing`);
      } else {
        const errorText = await response.text();
        console.log(`  ❌ Google Calendar: Failed to ingest (${response.status}): ${errorText}`);
      }
    } else {
      console.log('  ⚠️  Google Calendar test data not found');
    }
  } catch (error) {
    console.log(`  ❌ Google Calendar error:`, error);
  }
  
  console.log('\n✨ Test data seeding complete!');
}

async function main() {
  try {
    await seedDatabase();
    process.exit(0);
  } catch (error) {
    console.error('❌ Seeding failed:', error);
    process.exit(1);
  }
}

main();