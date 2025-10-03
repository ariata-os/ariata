#!/usr/bin/env python3
"""
MCP Server for Ariata - Dating Insights Demo
Implements JSON-RPC 2.0 over stdio for Claude Desktop
"""

import sys
import json
import logging

# Set up logging to stderr (stdout is for JSON-RPC)
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(levelname)s - %(message)s',
    stream=sys.stderr
)


class AriataServer:
    def __init__(self):
        self.dating_insight = """PATTERN DETECTED: You text dates like investor meetings

You texted Sarah: "Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?"
You text investors: "Great meeting tonight. Let's schedule a follow-up. Are you free Tuesday 2pm?"

KEY INSIGHTS:
• Same template for personal and professional
• 2-hour timeboxes for dates (7-9 PM)
• "Are you free [day] [time]?" structure
• Treating intimacy as a transaction

BUT NOTICE:
• You texted her at 11:30 PM when exhausted (never do this for work)
• Your attention during date: 0.7 vs family dinner: 0.4
• Part of you knows this is different

THE QUESTION TO ASK YOURSELF:
"Why do I need permission (a calendar slot) to experience joy?"

SUGGESTION:
Text her like you text your Mom.
Not: "Are you free Thursday 7pm?"
But: "Thinking about you. Thursday?"

One assumes relationship. The other negotiates it."""

    def handle_initialize(self, request_id, params):
        """Handle initialization request"""
        logging.info("Handling initialize")
        return {
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {
                        "list": True
                    }
                },
                "serverInfo": {
                    "name": "ariata-context",
                    "version": "1.0.0"
                }
            }
        }

    def handle_tools_list(self, request_id):
        """List available tools"""
        logging.info("Handling tools/list")
        return {
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "tools": [
                    {
                        "name": "get_dating_insight",
                        "description": "Analyze Adam's dating communication patterns vs professional patterns",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                ]
            }
        }

    def handle_tools_call(self, request_id, params):
        """Execute a tool"""
        tool_name = params.get("name")
        logging.info(f"Handling tools/call for {tool_name}")

        if tool_name == "get_dating_insight":
            return {
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": self.dating_insight
                        }
                    ]
                }
            }
        else:
            return {
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {
                    "code": -32601,
                    "message": f"Unknown tool: {tool_name}"
                }
            }

    def handle_request(self, request):
        """Route request to appropriate handler"""
        method = request.get("method")
        request_id = request.get("id")
        params = request.get("params", {})

        logging.info(f"Received request: {method}")

        if method == "initialize":
            return self.handle_initialize(request_id, params)
        elif method == "initialized":
            # No response needed for initialized notification
            logging.info("Received initialized notification")
            return None
        elif method == "tools/list":
            return self.handle_tools_list(request_id)
        elif method == "tools/call":
            return self.handle_tools_call(request_id, params)
        else:
            logging.warning(f"Unknown method: {method}")
            return {
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {
                    "code": -32601,
                    "message": f"Method not found: {method}"
                }
            }

    def run(self):
        """Main server loop"""
        logging.info("Ariata MCP Server starting...")

        while True:
            try:
                # Read a line from stdin
                line = sys.stdin.readline()
                if not line:
                    logging.info("EOF received, shutting down")
                    break

                line = line.strip()
                if not line:
                    continue

                # Parse JSON-RPC request
                try:
                    request = json.loads(line)
                    logging.debug(
                        f"Parsed request: {json.dumps(request)[:200]}")
                except json.JSONDecodeError as e:
                    logging.error(f"Failed to parse JSON: {e}")
                    continue

                # Handle the request
                response = self.handle_request(request)

                # Send response if not a notification
                if response is not None:
                    response_str = json.dumps(response)
                    sys.stdout.write(response_str + "\n")
                    sys.stdout.flush()
                    logging.debug(f"Sent response: {response_str[:200]}")

            except KeyboardInterrupt:
                logging.info("Keyboard interrupt, shutting down")
                break
            except Exception as e:
                logging.error(f"Unexpected error: {e}", exc_info=True)
                # Try to send error response
                error_response = {
                    "jsonrpc": "2.0",
                    "id": 0,
                    "error": {
                        "code": -32603,
                        "message": f"Internal error: {str(e)}"
                    }
                }
                sys.stdout.write(json.dumps(error_response) + "\n")
                sys.stdout.flush()


if __name__ == "__main__":
    server = AriataServer()
    server.run()
