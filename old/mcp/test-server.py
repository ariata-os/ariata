#!/usr/bin/env python3
"""
Minimal MCP server for Ariata dating insights demo
"""
import json
import sys

# Your dating context that shows the pattern
DATING_INSIGHTS = """
You texted Alexandra: "Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?"
You text investors: "Great meeting tonight. Let's schedule a follow-up. Are you free Tuesday 2pm?"

PATTERN: You're using the exact same template for dates as investor meetings.
- Same 2-hour timeboxing (7-9 PM)
- Same "Are you free [day] [time]?" structure
- Same transactional language

But notice: You texted her at 11:30 PM when exhausted (you never email investors then).

The question you should ask: "Why do I need permission (a calendar slot) to experience joy?"

Try texting her like you text your Mom: Not "Are you free Thursday?" but "Thinking about you. Thursday?"
One assumes relationship. The other negotiates it.
"""

def handle_request(request):
    """Handle JSON-RPC requests"""
    if "method" in request:
        method = request["method"]

        # Handle initialization
        if method == "initialize":
            return {
                "jsonrpc": "2.0",
                "id": request.get("id"),
                "result": {
                    "protocolVersion": "0.1.0",
                    "capabilities": {
                        "tools": {
                            "list": True
                        }
                    }
                }
            }

        # List available tools
        elif method == "tools/list":
            return {
                "jsonrpc": "2.0",
                "id": request.get("id"),
                "result": {
                    "tools": [
                        {
                            "name": "get_dating_insight",
                            "description": "Get Adam's dating pattern insight",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        }
                    ]
                }
            }

        # Execute tool
        elif method == "tools/call":
            if request.get("params", {}).get("name") == "get_dating_insight":
                return {
                    "jsonrpc": "2.0",
                    "id": request.get("id"),
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": DATING_INSIGHTS
                            }
                        ]
                    }
                }

    # Default error response
    return {
        "jsonrpc": "2.0",
        "id": request.get("id", 0),
        "error": {
            "code": -32601,
            "message": "Method not found"
        }
    }

def main():
    """Main loop - read JSON-RPC from stdin, write to stdout"""
    sys.stderr.write("Ariata MCP server starting...\n")

    while True:
        try:
            # Read line from stdin
            line = sys.stdin.readline()
            if not line:
                break

            # Parse JSON request
            request = json.loads(line)

            # Handle request
            response = handle_request(request)

            # Send response
            sys.stdout.write(json.dumps(response) + "\n")
            sys.stdout.flush()

        except Exception as e:
            sys.stderr.write(f"Error: {e}\n")
            # Send error response
            error_response = {
                "jsonrpc": "2.0",
                "id": 0,
                "error": {
                    "code": -32603,
                    "message": str(e)
                }
            }
            sys.stdout.write(json.dumps(error_response) + "\n")
            sys.stdout.flush()

if __name__ == "__main__":
    main()