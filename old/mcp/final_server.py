#!/usr/bin/env python3
"""
Minimal working MCP server for Ariata dating insights demo
"""

import sys
import json

def send_message(msg):
    """Send a JSON-RPC message"""
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()

def log(msg):
    """Log to stderr for debugging"""
    sys.stderr.write(f"[Ariata] {msg}\n")
    sys.stderr.flush()

# Main loop
log("Server starting...")

while True:
    try:
        line = sys.stdin.readline()
        if not line:
            break

        request = json.loads(line.strip())
        method = request.get("method")
        request_id = request.get("id")

        if method == "initialize":
            send_message({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "ariata",
                        "version": "1.0.0"
                    }
                }
            })

        elif method == "initialized":
            # This is a notification, no response needed
            log("Client initialized")

        elif method == "tools/list":
            send_message({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "tools": [{
                        "name": "get_dating_insight",
                        "description": "Analyze how Adam texts dates vs investors - shows pattern of treating intimacy like transactions",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    }]
                }
            })

        elif method == "prompts/list":
            # Return empty prompts list
            send_message({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "prompts": []
                }
            })

        elif method == "resources/list":
            # Return empty resources list
            send_message({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "resources": []
                }
            })

        elif method == "tools/call":
            tool_name = request.get("params", {}).get("name")
            if tool_name == "get_dating_insight":
                send_message({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": """PATTERN DETECTED: You text dates like investor meetings

You texted Alexandra: "Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?"
You text investors: "Great meeting tonight. Let's schedule a follow-up. Are you free Tuesday 2pm?"

KEY INSIGHTS:
• Same template for both personal and professional
• 2-hour timeboxes for dates (7-9 PM)
• Treating intimacy as a transaction

The question: "Why do I need permission (a calendar slot) to experience joy?"

Try: Text her like you text your Mom - assuming relationship, not negotiating it."""
                        }]
                    }
                })
            else:
                send_message({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {
                        "code": -32601,
                        "message": f"Unknown tool: {tool_name}"
                    }
                })

        elif method:
            log(f"Unknown method: {method}")

    except Exception as e:
        log(f"Error: {e}")
        continue