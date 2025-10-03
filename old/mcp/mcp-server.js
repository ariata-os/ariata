#!/usr/bin/env node

// Simple MCP server for Ariata demo
const { Server } = require('@modelcontextprotocol/sdk/server/index.js');
const { StdioServerTransport } = require('@modelcontextprotocol/sdk/server/stdio.js');

// Your personal context data
const ADAM_CONTEXT = {
  communication_patterns: {
    to_date: "Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?",
    to_investor: "Great meeting tonight. Let's schedule a follow-up. Are you free Tuesday 2pm?",
    to_cofounder: "Great meeting tonight. Let's discuss equity structure. Are you free Tuesday 2pm?",
    insight: "You text Alexandra (your date) with the exact same template as professional contacts"
  },

  time_allocation: {
    work: "71 hours (42%)",
    sleep: "45.5 hours (27%)",
    social: "24 hours (14%)",
    family: "7 hours (4%)",
    dating: "3.5 hours (2%)",
    spiritual: "1.25 hours (0.7%)"
  },

  attention_scores: {
    highest: [
      "0.9 - Code performance improvement",
      "0.9 - $250K investment secured",
      "0.9 - Character synthesis breakthrough"
    ],
    alexandra_date: "0.7 - Higher than family dinner (0.4)",
    sleep: "0.1 - Lowest attention to self-care"
  },

  behavioral_patterns: [
    "You schedule dates in 2-hour windows like investor meetings",
    "You document transactions, not transformations",
    "You're never just 'Adam', always 'Adam-with/at/doing'",
    "Your morning routine gets 0.2 attention vs 0.9 for work",
    "You text at 11:30pm only for Alexandra (never investors)"
  ],

  key_insights: [
    "You're treating intimacy like a transaction to optimize",
    "You've turned your entire life into a sprint planning session",
    "You schedule spontaneity - even 'inspiration struck' is timeboxed",
    "You're building Ariata for human flourishing while living mechanically",
    "The question isn't efficiency - it's why you see life as something to debug"
  ],

  questions_not_asking: [
    "When did I stop being a person and become a founder?",
    "Why do I need permission (a calendar slot) to experience joy?",
    "What would happen if I texted Alexandra the way I text my Mom?",
    "Who am I when I'm not optimizing?",
    "What if the bug isn't inefficiency, but believing life needs debugging?"
  ]
};

class AriataContextServer {
  constructor() {
    this.server = new Server(
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

    this.setupHandlers();
  }

  setupHandlers() {
    // List available tools
    this.server.setRequestHandler('tools/list', async () => ({
      tools: [
        {
          name: 'get_personal_context',
          description: 'Get Adam\'s personal behavioral patterns and insights',
          inputSchema: {
            type: 'object',
            properties: {
              topic: {
                type: 'string',
                description: 'Topic to analyze (dating, work, time, patterns)',
                enum: ['dating', 'work', 'time', 'patterns', 'all']
              }
            },
            required: ['topic']
          }
        },
        {
          name: 'get_dating_analysis',
          description: 'Analyze Adam\'s dating patterns specifically',
          inputSchema: {
            type: 'object',
            properties: {}
          }
        }
      ]
    }));

    // Handle tool calls
    this.server.setRequestHandler('tools/call', async (request) => {
      const { name, arguments: args } = request.params;

      if (name === 'get_personal_context') {
        return await this.getPersonalContext(args.topic || 'all');
      } else if (name === 'get_dating_analysis') {
        return await this.getDatingAnalysis();
      }

      throw new Error(`Unknown tool: ${name}`);
    });
  }

  async getPersonalContext(topic) {
    let content = '';

    switch(topic) {
      case 'dating':
        content = `DATING PATTERN ANALYSIS:

You texted Alexandra: "${ADAM_CONTEXT.communication_patterns.to_date}"
You texted investor: "${ADAM_CONTEXT.communication_patterns.to_investor}"

${ADAM_CONTEXT.communication_patterns.insight}

Key patterns:
- 2-hour timebox for date (7-9 PM) like meetings
- Professional language template in personal context
- But: Texted her at 11:30 PM when exhausted (never do this for work)
- Attention during date: ${ADAM_CONTEXT.attention_scores.alexandra_date}

The deeper question: Why do you need a calendar slot to experience joy?`;
        break;

      case 'time':
        content = `TIME ALLOCATION ANALYSIS:

Your week breakdown:
${Object.entries(ADAM_CONTEXT.time_allocation).map(([k,v]) => `- ${k}: ${v}`).join('\n')}

Stated values: "Family is everything", "Faith grounds me"
Lived values: 42% work, 4% family, 0.7% spiritual

You're living an inverted priority pyramid.`;
        break;

      case 'patterns':
        content = `BEHAVIORAL PATTERNS:

${ADAM_CONTEXT.behavioral_patterns.join('\n- ')}

Key Insights:
${ADAM_CONTEXT.key_insights.join('\n- ')}`;
        break;

      default:
        content = JSON.stringify(ADAM_CONTEXT, null, 2);
    }

    return {
      content: [
        {
          type: 'text',
          text: content
        }
      ]
    };
  }

  async getDatingAnalysis() {
    const analysis = `DATING ANALYSIS - The Deeper Pattern:

You approached Alexandra exactly like a business transaction:

1. Initial text: "${ADAM_CONTEXT.communication_patterns.to_date}"
   Compare to: "${ADAM_CONTEXT.communication_patterns.to_investor}"

2. Scheduled in 2-hour window (7-9 PM) like your meetings

3. Documented "great chemistry" like "40% performance improvement"

BUT here's what's beautiful (and what you're missing):
- You texted her at 11:30 PM exhausted (NEVER email investors then)
- Your attention was 0.7 - higher than family dinner (0.4)
- You broke pattern to connect when tired

The question you should ask:
"${ADAM_CONTEXT.questions_not_asking[1]}"

Try this: Text her like you text your Mom.
Not "Are you free Thursday 7pm?"
But "Thinking about you. Thursday?"

One assumes relationship. The other negotiates it.

${ADAM_CONTEXT.key_insights[0]}`;

    return {
      content: [
        {
          type: 'text',
          text: analysis
        }
      ]
    };
  }

  async run() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error('Ariata Context MCP server running...');
  }
}

// Run the server
if (require.main === module) {
  const server = new AriataContextServer();
  server.run().catch(console.error);
}