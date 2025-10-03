import { json } from "@sveltejs/kit";
import type { RequestHandler } from "./$types";
import { db } from "$lib/db/client";
import { sources, streamConfigs, streams } from "$lib/db/schema";
import { eq, and } from "drizzle-orm";

export const POST: RequestHandler = async ({ request }) => {
  const deviceToken = request.headers.get("x-device-token");

  if (!deviceToken) {
    return json(
      {
        success: false,
        error: "Missing device token",
      },
      { status: 401 },
    );
  }

  try {
    // Parse request body for device info
    let deviceInfo: any = {};
    try {
      deviceInfo = await request.json();
    } catch {
      // Body is optional
    }

    // Normalize token to uppercase for case-insensitive comparison
    const normalizedToken = deviceToken.toUpperCase();
    console.log("[DEVICE VERIFICATION] Starting verification");

    // Find device source by token (case-insensitive)
    const [deviceSource] = await db
      .select()
      .from(sources)
      .where(eq(sources.deviceToken, normalizedToken))
      .limit(1);

    if (!deviceSource) {
      console.log("[DEVICE VERIFICATION] FAILED - Token not found in database");
      console.log("[DEVICE VERIFICATION] Token searched:", normalizedToken);
      return json(
        {
          success: false,
          error:
            "Device token not found. Please generate a new token in the web app.",
        },
        { status: 404 },
      );
    }

    console.log("[DEVICE VERIFICATION] Source found:", {
      id: deviceSource.id,
      sourceName: deviceSource.sourceName,
      instanceName: deviceSource.instanceName,
      status: deviceSource.status,
    });

    // Get stream configurations for this source
    const streamConfigList = await db
      .select()
      .from(streamConfigs)
      .where(eq(streamConfigs.sourceName, deviceSource.sourceName));

    // Check if streams have been configured (user has clicked "Save & Start Syncing")
    // Join with stream_configs to get the stream names
    const configuredStreams = await db
      .select({
        id: streams.id,
        streamConfigId: streams.streamConfigId,
        streamName: streamConfigs.streamName,
        enabled: streams.enabled,
        initialSyncDays: streams.initialSyncDays,
      })
      .from(streams)
      .leftJoin(streamConfigs, eq(streams.streamConfigId, streamConfigs.id))
      .where(eq(streams.sourceId, deviceSource.id));

    // Configuration is complete when:
    // 1. Source status is 'active' (set when user saves stream configuration)
    // 2. There are configured streams in the database
    const configurationComplete =
      deviceSource.status === "active" && configuredStreams.length > 0;

    // Update device source with connection info
    await db
      .update(sources)
      .set({
        lastSyncStatus: "success",
        lastSyncAt: new Date(),
        sourceMetadata: {
          ...deviceSource.sourceMetadata,
          deviceInfo: deviceInfo,
          firstVerifiedAt:
            deviceSource.sourceMetadata?.firstVerifiedAt ||
            new Date().toISOString(),
          lastVerifiedAt: new Date().toISOString(),
        },
      })
      .where(eq(sources.id, deviceSource.id));

    // Build stream configuration with enabled status and initial sync days
    const streamConfiguration: Record<string, any> = {};
    
    // Create a map of configured streams for quick lookup
    const configuredStreamMap = new Map(
      configuredStreams.map(s => [s.streamName, s])
    );
    
    console.log('[DEVICE VERIFICATION] Configured streams from DB:', configuredStreams);
    console.log('[DEVICE VERIFICATION] Stream configs available:', streamConfigList.map(s => s.streamName));
    
    streamConfigList.forEach((streamConfig) => {
      const streamKey = streamConfig.streamName.replace(
        `${deviceSource.sourceName}_`,
        "",
      );
      
      // Get the configured stream if it exists
      const configuredStream = configuredStreamMap.get(streamConfig.streamName);
      
      console.log(`[DEVICE VERIFICATION] Stream ${streamConfig.streamName} -> key: ${streamKey}, enabled: ${configuredStream?.enabled ?? false}`);
      
      streamConfiguration[streamKey] = {
        enabled: configuredStream?.enabled ?? false,
        initialSyncDays: configuredStream?.initialSyncDays ?? 90,
        displayName: streamConfig.displayName,
      };
    });

    // Return success with configuration status
    const responseData = {
      success: true,
      configurationComplete,
      source: {
        id: deviceSource.id,
        name: deviceSource.instanceName,
        sourceName: deviceSource.sourceName,
        status: deviceSource.status,
      },
      streams: streamConfigList.map((s) => {
        const configuredStream = configuredStreamMap.get(s.streamName);
        return {
          name: s.streamName,
          displayName: s.displayName,
          ingestionType: s.ingestionType,
          cronSchedule: s.cronSchedule,
          status: s.status,
          enabled: configuredStream?.enabled ?? false,
          initialSyncDays: configuredStream?.initialSyncDays ?? 90,
        };
      }),
      configuredStreamCount: configuredStreams.length,
      configuration: {
        streams: streamConfiguration,
        endpoints: {
          ingest: "/api/ingest",
          verify: "/api/device/verify",
        },
      },
      message: configurationComplete
        ? "Device verified and configuration complete"
        : "Device verified - waiting for stream configuration",
    };

    console.log("[DEVICE VERIFICATION] SUCCESS - Returning response:", {
      configurationComplete,
      sourceStatus: deviceSource.status,
      streamCount: configuredStreams.length,
    });

    return json(responseData);
  } catch (error) {
    console.error("Device verification error:", error);
    return json(
      {
        success: false,
        error: "Failed to verify device",
      },
      { status: 500 },
    );
  }
};

// GET endpoint for checking device status
export const GET: RequestHandler = async ({ url }) => {
  const deviceToken = url.searchParams.get("token");

  if (!deviceToken) {
    return json(
      {
        success: false,
        error: "Token parameter required",
      },
      { status: 400 },
    );
  }

  try {
    const [deviceSource] = await db
      .select({
        id: sources.id,
        lastSyncStatus: sources.lastSyncStatus,
        instanceName: sources.instanceName,
        lastSyncAt: sources.lastSyncAt,
        sourceMetadata: sources.sourceMetadata,
      })
      .from(sources)
      .where(eq(sources.deviceToken, deviceToken))
      .limit(1);

    if (!deviceSource) {
      return json(
        {
          success: false,
          connected: false,
          error: "Device not found",
        },
        { status: 404 },
      );
    }

    const isConnected =
      deviceSource.lastSyncStatus === "success" &&
      deviceSource.sourceMetadata?.lastVerifiedAt;

    return json({
      success: true,
      connected: isConnected,
      device: {
        id: deviceSource.id,
        name: deviceSource.instanceName,
        status: deviceSource.lastSyncStatus || "pending",
        lastSyncAt: deviceSource.lastSyncAt,
        verifiedAt: deviceSource.sourceMetadata?.lastVerifiedAt || null,
      },
    });
  } catch (error) {
    console.error("Device status check error:", error);
    return json(
      {
        success: false,
        error: "Failed to check device status",
      },
      { status: 500 },
    );
  }
};
