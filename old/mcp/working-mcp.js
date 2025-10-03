#!/usr/bin/env node
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

// Your dating context
const DATING_CONTEXT = {
  texts_to_alexandra: "Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?",
  texts_to_investor: "Great meeting tonight. Let's schedule a follow-up. Are you free Tuesday 2pm?",
  pattern: "You text your date exactly like an investor meeting - same template, same 2-hour window",
  insight: "You're treating intimacy like a transaction to optimize",
  advice: "Try texting her like you text your Mom - assuming relationship, not negotiating it"
};

// Create server
const server = new Server(
  {
    name: 'ariata-context',
    version: '1.0.0',
  },
  {
    capabilities: {
      tools: {},
    },
  }
);

// Handle list tools request
server.setRequestHandler('tools/list', async () => ({
  tools: [
    {
      name: 'get_dating_insight',
      description: 'Get insight about Adam dating patterns vs investor patterns',
      inputSchema: {
        type: 'object',
        properties: {},
      },
    },
  ],
}));

// Handle tool execution
server.setRequestHandler('tools/call', async (request) => {
  if (request.params.name === 'get_dating_insight') {
    return {
      content: [
        {
          type: 'text',
          text: `DATING PATTERN DETECTED:

You texted Alexandra: "${DATING_CONTEXT.texts_to_alexandra}"
You text investors: "${DATING_CONTEXT.texts_to_investor}"

${DATING_CONTEXT.pattern}

Key insight: ${DATING_CONTEXT.insight}

Advice: ${DATING_CONTEXT.advice}

The deeper question: Why do you need permission (a calendar slot) to experience joy?`,
        },
      ],
    };
  }

  throw new Error(`Unknown tool: ${request.params.name}`);
});

// Run the server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error('Ariata MCP server running...');
}

main().catch((error) => {
  console.error('Server error:', error);
  process.exit(1);
});