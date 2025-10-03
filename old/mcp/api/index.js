const express = require('express');
const cors = require('cors');
const { Server } = require('@modelcontextprotocol/sdk/server/index.js');
const { StdioServerTransport } = require('@modelcontextprotocol/sdk/server/stdio.js');

const app = express();
app.use(cors());
app.use(express.json());

// The personal context data (in production, this would be from a database)
const ADAM_CONTEXT = {
  weekSummary: {
    dates: "2024-10-27 to 2024-11-02",
    totalEvents: 63,
    locations: ["Houston", "Austin", "Dallas"],
    keyPeople: ["Alexandra (date)", "Sarah Chen (investor)", "Jake (friend)", "Mom", "Dad"],
    majorEvents: [
      "Catholic Mass with family",
      "$250K angel investment commitment",
      "First date with Alexandra",
      "Capital Factory networking event",
      "Pickleball tournament"
    ]
  },

  patterns: {
    communication: {
      professional: {
        templates: [
          "Following up on our conversation. Looking forward to next steps.",
          "Let's schedule a follow-up. Are you free Tuesday 2pm?",
          "Great meeting tonight. Let's discuss next steps."
        ],
        style: "formal, structured, transaction-focused"
      },
      personal: {
        examples: [
          "Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?",
          "Thanks for breakfast. Driving now."
        ],
        insight: "Uses professional templates even in personal contexts"
      }
    },

    timeAllocation: {
      work: "71 hours (42%)",
      sleep: "45.5 hours (27% - avg 6.5hrs/night)",
      social: "24 hours (14%)",
      family: "7 hours (4%)",
      spiritual: "1.25 hours (0.7%)",
      insight: "Stated values (family, spirituality) inversely correlate with time spent"
    },

    attention: {
      highest: [
        { score: 0.9, event: "Refactored stream processing, 40% performance improvement" },
        { score: 0.9, event: "$250K angel commitment secured" },
        { score: 0.7, event: "Catholic Mass" },
        { score: 0.7, event: "Date with Alexandra" }
      ],
      lowest: [
        { score: 0.1, event: "Sleeping" },
        { score: 0.2, event: "Morning routines" }
      ],
      insight: "Attention inversely correlates with self-care"
    },

    behavioral: {
      scheduling: "Treats personal events like professional meetings (2-hour timeboxes)",
      documentation: "Documents transactions, not transformations",
      identity: "Always 'Adam with/at/doing' never just 'Adam'",
      language: {
        work: "active voice: building, shipping, crushing",
        life: "passive voice: maintaining, catching up, staying in touch"
      }
    }
  },

  insights: [
    "You text your date like an investor meeting",
    "You've turned your entire life into a sprint planning session",
    "You schedule spontaneity (even 'inspiration' is timeboxed)",
    "You document everything but experience little",
    "Your identity is always modified (Adam-with, Adam-at), never just Adam"
  ],

  keyQuestions: [
    "When did I stop being a person and become a founder?",
    "Why do I schedule spontaneity?",
    "What would happen if I texted Alexandra the way I text my Mom?",
    "Am I building Ariata to remind others to live, because I've forgotten how?",
    "What if the bug in my life isn't inefficiency, but the belief that life is something to be debugged?"
  ]
};

// MCP Tool definitions
const tools = [
  {
    name: "get_personal_context",
    description: "Retrieve Adam's personal context and behavioral patterns",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "What aspect of personal context to retrieve"
        },
        timeframe: {
          type: "string",
          description: "Specific timeframe to analyze"
        }
      }
    }
  },
  {
    name: "analyze_patterns",
    description: "Analyze behavioral patterns from Adam's week",
    inputSchema: {
      type: "object",
      properties: {
        pattern_type: {
          type: "string",
          enum: ["communication", "time", "attention", "behavioral"],
          description: "Type of pattern to analyze"
        }
      }
    }
  },
  {
    name: "compare_stated_vs_lived",
    description: "Compare Adam's stated values vs actual behavior",
    inputSchema: {
      type: "object",
      properties: {
        domain: {
          type: "string",
          description: "Domain to compare (family, work, spirituality, etc)"
        }
      }
    }
  }
];

// MCP Resources (prompts)
const prompts = [
  {
    name: "reflective_questions",
    description: "Generate questions Adam should be asking himself",
    arguments: [
      {
        name: "context",
        description: "Specific context or situation",
        required: false
      }
    ]
  },
  {
    name: "pattern_mirror",
    description: "Reflect Adam's patterns back with specific examples",
    arguments: [
      {
        name: "topic",
        description: "Topic to analyze (dating, work, family, etc)",
        required: true
      }
    ]
  }
];

// Initialize MCP server
class AriataContextServer {
  constructor() {
    this.server = new Server(
      {
        name: "ariata-context",
        version: "1.0.0"
      },
      {
        capabilities: {
          tools: {},
          prompts: {}
        }
      }
    );

    this.setupHandlers();
  }

  setupHandlers() {
    // List available tools
    this.server.setRequestHandler('tools/list', async () => ({
      tools
    }));

    // Handle tool calls
    this.server.setRequestHandler('tools/call', async (request) => {
      const { name, arguments: args } = request.params;

      switch (name) {
        case 'get_personal_context':
          return this.getPersonalContext(args);

        case 'analyze_patterns':
          return this.analyzePatterns(args);

        case 'compare_stated_vs_lived':
          return this.compareStatedVsLived(args);

        default:
          throw new Error(`Unknown tool: ${name}`);
      }
    });

    // List available prompts
    this.server.setRequestHandler('prompts/list', async () => ({
      prompts
    }));

    // Get a specific prompt
    this.server.setRequestHandler('prompts/get', async (request) => {
      const { name, arguments: args } = request.params;

      switch (name) {
        case 'reflective_questions':
          return this.getReflectiveQuestions(args);

        case 'pattern_mirror':
          return this.getPatternMirror(args);

        default:
          throw new Error(`Unknown prompt: ${name}`);
      }
    });
  }

  async getPersonalContext({ query }) {
    // In reality, this would search through embeddings
    const relevantContext = {
      ...ADAM_CONTEXT.weekSummary,
      patterns: ADAM_CONTEXT.patterns,
      insights: ADAM_CONTEXT.insights
    };

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(relevantContext, null, 2)
        }
      ]
    };
  }

  async analyzePatterns({ pattern_type }) {
    const pattern = ADAM_CONTEXT.patterns[pattern_type];

    if (!pattern) {
      throw new Error(`Unknown pattern type: ${pattern_type}`);
    }

    let analysis = `Pattern Analysis: ${pattern_type.toUpperCase()}\n\n`;

    if (pattern_type === 'communication') {
      analysis += `You text Alexandra (your date) with the same template as investors:\n`;
      analysis += `- To date: "${ADAM_CONTEXT.patterns.communication.personal.examples[0]}"\n`;
      analysis += `- To investor: "${ADAM_CONTEXT.patterns.communication.professional.templates[0]}"\n\n`;
      analysis += `Insight: ${pattern.personal.insight}`;
    } else {
      analysis += JSON.stringify(pattern, null, 2);
    }

    return {
      content: [
        {
          type: "text",
          text: analysis
        }
      ]
    };
  }

  async compareStatedVsLived({ domain }) {
    let comparison = `Stated vs Lived Values: ${domain.toUpperCase()}\n\n`;

    switch(domain) {
      case 'family':
        comparison += `STATED: "Family is everything"\n`;
        comparison += `LIVED: 7 hours out of 168 (4%)\n`;
        comparison += `PATTERN: You schedule family dinners like investor meetings\n`;
        break;

      case 'spirituality':
        comparison += `STATED: "Relationship with God grounds me"\n`;
        comparison += `LIVED: 1.25 hours out of 168 (0.7%)\n`;
        comparison += `PATTERN: Mass gets 0.7 attention vs 0.9 for code improvements\n`;
        break;

      case 'balance':
        comparison += `STATED: "Want to find balance and meaning"\n`;
        comparison += `LIVED: 42% work, 27% sleep, 14% social\n`;
        comparison += `PATTERN: Your "balanced" week is 71 hours of work\n`;
        break;

      default:
        comparison += JSON.stringify(ADAM_CONTEXT.patterns.timeAllocation, null, 2);
    }

    return {
      content: [
        {
          type: "text",
          text: comparison
        }
      ]
    };
  }

  async getReflectiveQuestions({ context }) {
    const questions = [...ADAM_CONTEXT.keyQuestions];

    if (context === 'dating') {
      questions.unshift(
        "Why do I text Alexandra like she's a calendar conflict to resolve?",
        "What would intimacy look like if I didn't schedule it?"
      );
    }

    return {
      messages: [
        {
          role: "user",
          content: {
            type: "text",
            text: `Questions you should be asking yourself:\n\n${questions.map((q, i) => `${i + 1}. ${q}`).join('\n')}`
          }
        }
      ]
    };
  }

  async getPatternMirror({ topic }) {
    let mirror = `Looking at your week, here's what I see about ${topic}:\n\n`;

    switch(topic) {
      case 'dating':
        mirror += `You texted Alexandra: "Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?"\n\n`;
        mirror += `This is identical to how you text professional contacts. You're treating intimacy like a transaction.\n\n`;
        mirror += `But notice: You texted her at 11:30 PM when exhausted (you never email investors then).\n`;
        mirror += `This breaks your pattern. Part of you knows this is different.\n\n`;
        mirror += `Try texting her like you text your mom - assuming relationship, not negotiating it.`;
        break;

      case 'work':
        mirror += `Your highest attention (0.9) goes to code improvements.\n`;
        mirror += `You describe work in active voice: "shipping", "building", "crushing it"\n`;
        mirror += `You describe life in passive voice: "maintaining", "catching up"\n\n`;
        mirror += `You're not living life - you're preventing it from degrading.`;
        break;

      default:
        mirror += JSON.stringify(ADAM_CONTEXT.patterns, null, 2);
    }

    return {
      messages: [
        {
          role: "assistant",
          content: {
            type: "text",
            text: mirror
          }
        }
      ]
    };
  }

  async run() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error("Ariata Context MCP server running");
  }
}

// Express API endpoints for web access
app.get('/api/health', (req, res) => {
  res.json({ status: 'healthy', service: 'ariata-mcp' });
});

app.post('/api/context', (req, res) => {
  const { query } = req.body;

  // Return relevant context based on query
  const response = {
    context: ADAM_CONTEXT,
    query,
    relevance_score: 0.94,
    insights: ADAM_CONTEXT.insights.slice(0, 3)
  };

  res.json(response);
});

app.post('/api/analyze', (req, res) => {
  const { topic, question } = req.body;

  let analysis = {
    topic,
    question,
    patterns: [],
    insights: [],
    questions_to_ask: []
  };

  if (topic === 'dating') {
    analysis.patterns = [
      "You text dates like investor meetings",
      "2-hour timeboxes for personal events",
      "Professional language in personal contexts"
    ];
    analysis.insights = [
      "You're treating intimacy like a transaction",
      "You schedule spontaneity",
      "Your identity is always modified (Adam-with), never just Adam"
    ];
    analysis.questions_to_ask = [
      "Why do I need permission (a schedule slot) to experience joy?",
      "What would happen if I texted her the way I text my Mom?",
      "Who am I when I'm not optimizing?"
    ];
  }

  res.json(analysis);
});

// For Vercel
module.exports = app;

// For local MCP server mode
if (require.main === module) {
  if (process.env.MCP_MODE) {
    const server = new AriataContextServer();
    server.run().catch(console.error);
  } else {
    const PORT = process.env.PORT || 3001;
    app.listen(PORT, () => {
      console.log(`Ariata MCP API running on port ${PORT}`);
    });
  }
}