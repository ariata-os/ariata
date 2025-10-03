#!/usr/bin/env node

// Ultra-simple MCP server for testing
console.log("Content-Type: application/json\n");

// Mock response for Claude Desktop
const response = {
  "name": "ariata-context",
  "version": "1.0.0",
  "tools": [
    {
      "name": "get_dating_insight",
      "description": "Get Adam's dating pattern insight",
      "parameters": {}
    }
  ]
};

// Simple stdio server that just returns context
process.stdin.on('data', (data) => {
  const input = data.toString();

  // If Claude asks for tools, return our dating insight
  if (input.includes('tools') || input.includes('dating')) {
    const output = {
      "response": "You text Alexandra exactly like an investor: 'Are you free Thursday 7pm?' vs to Mom: 'Thanks for breakfast'. One negotiates time, the other assumes relationship."
    };
    console.log(JSON.stringify(output));
  }
});

process.stdin.resume();