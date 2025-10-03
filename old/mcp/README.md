# Ariata MCP Server Demo

This is a simple MCP (Model Context Protocol) server for demonstrating Ariata's personal context capabilities.

## What This Demo Shows

The MCP server provides Claude with access to your personal context, enabling it to:
- Reference specific events and patterns from your week
- Compare your stated values vs actual behavior
- Identify linguistic patterns (like texting dates like investor meetings)
- Ask you questions you wouldn't ask yourself

## Quick Start

### Local Development

```bash
cd mcp
npm install
npm run dev
```

The server will run on `http://localhost:3000`

### Deploy to Vercel

```bash
npm run deploy
```

Or connect your GitHub repo to Vercel for automatic deployments.

## API Endpoints

### GET /api/health
Health check endpoint

### POST /api/context
Retrieve personal context based on a query

```json
{
  "query": "dating advice"
}
```

### POST /api/analyze
Analyze behavioral patterns

```json
{
  "topic": "dating",
  "question": "What patterns do you see?"
}
```

## MCP Tools Available

When running as an MCP server, the following tools are available:

1. **get_personal_context** - Retrieve Adam's weekly context and patterns
2. **analyze_patterns** - Analyze specific behavioral patterns
3. **compare_stated_vs_lived** - Compare stated values against actual behavior

## Demo Usage

### For the VC Demo

1. **Left Screen**: Open Claude without any MCP connection
2. **Right Screen**: Open Claude with MCP connection to this server

Ask both the same question:
> "I had a great first date this week and we're planning a second one. But I'm worried I'm approaching dating wrong while building my startup. What questions should I be asking myself that I'm not?"

### The Difference

- **Without Ariata**: Generic dating/startup advice
- **With Ariata**: Specific insights like "You text Alexandra with the same template as investors"

## Key Insights This Demo Reveals

1. **Communication Patterns**: Professional language bleeding into personal contexts
2. **Time Allocation**: 42% work vs 4% family (despite "family is everything")
3. **Attention Distribution**: 0.9 for code improvements vs 0.1 for sleep
4. **Identity Modulation**: Always "Adam-with/at/doing", never just "Adam"
5. **Scheduled Spontaneity**: Even "inspiration" happens in timeboxed windows

## The Core Message

This demo shows that Ariata doesn't just have your data - it understands your story. And more importantly, it asks you the questions you won't ask yourself:

- "When did I stop being a person and become a founder?"
- "Why do I schedule spontaneity?"
- "What if the bug in my life isn't inefficiency, but the belief that life needs debugging?"

## Technical Notes

This is a simplified demo. The real Ariata would:
- Use vector embeddings for semantic search
- Connect to your actual event timeline
- Provide real-time context updates
- Respect privacy with local-first architecture

## Contact

Built by Adam Jace for Ariata - the sovereign stack for personal AI.

Demo question: "What questions should I be asking myself that I'm not?"