/**
 * MCP Client for connecting to Ariata MCP HTTP server
 *
 * This client handles the StreamableHTTP protocol with stateful sessions.
 */

export interface McpTool {
	name: string;
	description?: string;
	inputSchema: any;
}

export interface McpResource {
	uri: string;
	name: string;
	title?: string;
	description?: string;
	mimeType?: string;
}

export interface McpResourceContents {
	uri: string;
	mimeType?: string;
	text?: string;
	blob?: string;
}

export interface McpServerInfo {
	name: string;
	version: string;
	protocolVersion?: string;
	capabilities?: {
		tools?: Record<string, any>;
		resources?: Record<string, any>;
		prompts?: Record<string, any>;
	};
	instructions?: string;
}

export interface McpCallToolResult {
	content: Array<{
		type: string;
		text?: string;
		[key: string]: any;
	}>;
	isError?: boolean;
}

/**
 * Fetch with timeout support
 *
 * @param url - The URL to fetch
 * @param options - Fetch options
 * @param timeout - Timeout in milliseconds (default: 30000ms / 30s)
 * @returns Promise<Response>
 */
async function fetchWithTimeout(
	url: string,
	options: RequestInit = {},
	timeout: number = 30000
): Promise<Response> {
	const controller = new AbortController();
	const timeoutId = setTimeout(() => controller.abort(), timeout);

	try {
		const response = await fetch(url, {
			...options,
			signal: controller.signal
		});
		clearTimeout(timeoutId);
		return response;
	} catch (error) {
		clearTimeout(timeoutId);
		if (error instanceof Error && error.name === 'AbortError') {
			throw new Error(`Request timed out after ${timeout}ms`);
		}
		throw error;
	}
}

/**
 * MCP Client for StreamableHTTP transport
 */
export class McpClient {
	private baseUrl: string;
	private sessionId: string | null = null;
	private serverInfo: McpServerInfo | null = null;
	private tools: Map<string, McpTool> = new Map();
	private resources: Map<string, McpResource> = new Map();
	private requestId = 1;

	constructor(baseUrl: string) {
		this.baseUrl = baseUrl;
	}

	/**
	 * Initialize the MCP connection
	 */
	async initialize(): Promise<McpServerInfo> {
		const response = await fetchWithTimeout(this.baseUrl, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Accept': 'application/json, text/event-stream'
			},
			body: JSON.stringify({
				jsonrpc: '2.0',
				id: this.requestId++,
				method: 'initialize',
				params: {
					protocolVersion: '2024-11-05',
					capabilities: {},
					clientInfo: {
						name: 'ariata-web',
						version: '0.1.0'
					}
				}
			})
		});

		if (!response.ok) {
			throw new Error(`Failed to initialize MCP client: ${response.statusText}`);
		}

		// Extract session ID from headers
		this.sessionId = response.headers.get('mcp-session-id');
		if (!this.sessionId) {
			throw new Error('No session ID received from MCP server');
		}

		// Parse SSE response
		const text = await response.text();
		const lines = text.split('\n');

		for (const line of lines) {
			if (line.startsWith('data: ')) {
				const data = JSON.parse(line.substring(6));
				if (data.result) {
					this.serverInfo = {
						name: data.result.serverInfo?.name || 'unknown',
						version: data.result.serverInfo?.version || '0.0.0',
						protocolVersion: data.result.protocolVersion,
						capabilities: data.result.capabilities,
						instructions: data.result.instructions
					};

					// Send initialized notification (required by MCP protocol)
					await this.sendInitializedNotification();

					return this.serverInfo;
				}
			}
		}

		throw new Error('Failed to parse initialize response');
	}

	/**
	 * Send initialized notification (required after initialize)
	 */
	private async sendInitializedNotification(): Promise<void> {
		if (!this.sessionId) {
			throw new Error('No session ID - cannot send initialized notification');
		}

		await fetchWithTimeout(this.baseUrl, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Accept': 'application/json, text/event-stream',
				'mcp-session-id': this.sessionId
			},
			body: JSON.stringify({
				jsonrpc: '2.0',
				method: 'notifications/initialized',
				params: {}
			})
		});
	}

	/**
	 * List available tools
	 */
	async listTools(): Promise<McpTool[]> {
		if (!this.sessionId) {
			throw new Error('Not initialized. Call initialize() first.');
		}

		const response = await fetchWithTimeout(this.baseUrl, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Accept': 'application/json, text/event-stream',
				'mcp-session-id': this.sessionId
			},
			body: JSON.stringify({
				jsonrpc: '2.0',
				id: this.requestId++,
				method: 'tools/list',
				params: {}
			})
		});

		if (!response.ok) {
			throw new Error(`Failed to list tools: ${response.statusText}`);
		}

		const text = await response.text();
		const lines = text.split('\n');

		for (const line of lines) {
			if (line.startsWith('data: ')) {
				const data = JSON.parse(line.substring(6));
				if (data.result?.tools) {
					const tools = data.result.tools;
					this.tools.clear();
					for (const tool of tools) {
						this.tools.set(tool.name, tool);
					}
					return tools;
				}
			}
		}

		return [];
	}

	/**
	 * List available resources
	 */
	async listResources(): Promise<McpResource[]> {
		if (!this.sessionId) {
			throw new Error('Not initialized. Call initialize() first.');
		}

		const response = await fetchWithTimeout(this.baseUrl, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Accept': 'application/json, text/event-stream',
				'mcp-session-id': this.sessionId
			},
			body: JSON.stringify({
				jsonrpc: '2.0',
				id: this.requestId++,
				method: 'resources/list',
				params: {}
			})
		});

		if (!response.ok) {
			throw new Error(`Failed to list resources: ${response.statusText}`);
		}

		const text = await response.text();
		const lines = text.split('\n');

		for (const line of lines) {
			if (line.startsWith('data: ')) {
				const data = JSON.parse(line.substring(6));
				if (data.result?.resources) {
					const resources = data.result.resources;
					this.resources.clear();
					for (const resource of resources) {
						this.resources.set(resource.uri, resource);
					}
					return resources;
				}
			}
		}

		return [];
	}

	/**
	 * Read a specific resource
	 */
	async readResource(uri: string): Promise<McpResourceContents[]> {
		if (!this.sessionId) {
			throw new Error('Not initialized. Call initialize() first.');
		}

		const response = await fetchWithTimeout(this.baseUrl, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Accept': 'application/json, text/event-stream',
				'mcp-session-id': this.sessionId
			},
			body: JSON.stringify({
				jsonrpc: '2.0',
				id: this.requestId++,
				method: 'resources/read',
				params: {
					uri
				}
			})
		});

		if (!response.ok) {
			throw new Error(`Failed to read resource ${uri}: ${response.statusText}`);
		}

		const text = await response.text();
		const lines = text.split('\n');

		for (const line of lines) {
			if (line.startsWith('data: ')) {
				const data = JSON.parse(line.substring(6));
				if (data.result?.contents) {
					return data.result.contents;
				}
				if (data.error) {
					throw new Error(`Error reading resource: ${data.error.message}`);
				}
			}
		}

		throw new Error('Failed to parse resource read response');
	}

	/**
	 * Call a tool
	 */
	async callTool(name: string, args: any): Promise<McpCallToolResult> {
		if (!this.sessionId) {
			throw new Error('Not initialized. Call initialize() first.');
		}

		const response = await fetchWithTimeout(this.baseUrl, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Accept': 'application/json, text/event-stream',
				'mcp-session-id': this.sessionId
			},
			body: JSON.stringify({
				jsonrpc: '2.0',
				id: this.requestId++,
				method: 'tools/call',
				params: {
					name,
					arguments: args
				}
			})
		});

		if (!response.ok) {
			throw new Error(`Failed to call tool ${name}: ${response.statusText}`);
		}

		const text = await response.text();
		const lines = text.split('\n');

		for (const line of lines) {
			if (line.startsWith('data: ')) {
				const data = JSON.parse(line.substring(6));
				if (data.result) {
					return data.result;
				}
				if (data.error) {
					return {
						content: [{
							type: 'text',
							text: `Error: ${data.error.message}`
						}],
						isError: true
					};
				}
			}
		}

		throw new Error('Failed to parse tool call response');
	}

	/**
	 * Get available tools as a Map
	 */
	getTools(): Map<string, McpTool> {
		return this.tools;
	}

	/**
	 * Get available resources as a Map
	 */
	getResources(): Map<string, McpResource> {
		return this.resources;
	}

	/**
	 * Get server info
	 */
	getServerInfo(): McpServerInfo | null {
		return this.serverInfo;
	}

	/**
	 * Close the session
	 */
	async close(): Promise<void> {
		if (!this.sessionId) {
			return;
		}

		try {
			await fetch(this.baseUrl, {
				method: 'DELETE',
				headers: {
					'mcp-session-id': this.sessionId
				}
			});
		} catch (error) {
			console.error('Failed to close MCP session:', error);
		} finally {
			this.sessionId = null;
			this.serverInfo = null;
			this.tools.clear();
			this.resources.clear();
		}
	}
}

/**
 * Create and initialize an MCP client
 */
export async function createMcpClient(baseUrl: string): Promise<McpClient> {
	const client = new McpClient(baseUrl);

	console.log(`[MCP Client] Initializing connection to ${baseUrl}`);
	try {
		await client.initialize();
		console.log('[MCP Client] Initialization successful');
	} catch (error) {
		console.error('[MCP Client] Initialization failed:', error);
		throw new Error(`Failed to initialize MCP client: ${error}`);
	}

	console.log('[MCP Client] Listing tools...');
	try {
		const tools = await client.listTools();
		console.log(`[MCP Client] Found ${tools.length} tools:`, tools.map((t) => t.name));
	} catch (error) {
		console.error('[MCP Client] Failed to list tools:', error);
		throw new Error(`Failed to list MCP tools: ${error}`);
	}

	console.log('[MCP Client] Listing resources...');
	try {
		const resources = await client.listResources();
		console.log(`[MCP Client] Found ${resources.length} resources:`, resources.map((r) => r.name));
	} catch (error) {
		console.error('[MCP Client] Failed to list resources:', error);
		// Don't throw - resources are optional
	}

	return client;
}
