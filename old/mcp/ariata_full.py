#!/usr/bin/env python3
"""
Full Ariata MCP Server with complete week context
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

# Your full week's context and patterns
WEEK_CONTEXT = {
    "temporal_window": "2024-10-27 to 2024-11-02",
    "total_events": 63,
    "high_attention_events": 18,

    "communication_patterns": {
        "to_alexandra": "Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?",
        "to_investor": "Following up on our conversation. Looking forward to next steps.",
        "to_mom": "Driving to Austin now. Will call later. Thanks for breakfast.",
        "pattern": "84% overlap between professional and personal language"
    },

    "time_allocation": {
        "work": "71 hours (42%)",
        "sleep": "45.5 hours (27% - avg 6.5hrs/night)",
        "social": "24 hours (14%)",
        "family": "7 hours (4%)",
        "spiritual": "1.25 hours (0.7%)"
    },

    "attention_scores": {
        "highest": [
            "0.9 - Refactored stream processing, 40% performance improvement",
            "0.9 - $250K angel commitment secured",
            "0.9 - Breakthrough on embedding space clustering"
        ],
        "mass": "0.7 - sermon about courage resonated",
        "alexandra_date": "0.7",
        "family_dinner": "0.5",
        "sleep": "0.1 - lowest attention to self-care"
    },

    "profound_insights": [
        "You've turned your entire life into a sprint planning session",
        "You document transactions, not transformations",
        "You're alone with yourself only at night",
        "Your lived priorities inverse your stated values perfectly",
        "You schedule spontaneity - even 'inspiration struck' is timeboxed 11pm-12:30am"
    ]
}

def get_profound_insight():
    """Generate the uniquely profound insight about Adam's week"""
    return """Something uniquely profound about your last week:

You attended Mass seeking "spiritual grounding" (0.7 attention) where a sermon about "courage in uncertainty" resonated with you. Yet here's the profound irony - you're not uncertain about work at all. You shipped with 0.9 attention, secured $250K, had technical breakthroughs.

The courage you need isn't for uncertainty - it's for CERTAINTY. The certainty of unscheduled time. The certainty of relationships that don't need calendar slots. The certainty of being "just Adam" instead of "Adam-with-purpose."

Your most profound moment wasn't the $250K commitment or the 40% performance improvement. It was at 11:30 PM when you texted Alexandra despite exhaustion. For once, you broke your optimization protocol for something that had no ROI - just human connection.

The deepest pattern: You're building Ariata for "human flourishing" while living a life optimized for everything except flourishing. You have 71 hours for work but only 1.25 hours for the spiritual grounding you claim centers you.

Here's what's uniquely profound - you're not just busy, you're PERFORMED busy. Even alone, you're "Adam-coding" or "Adam-planning." You've turned existence into a feature to ship.

The question hiding in your data: "What if the bug in my life isn't inefficiency, but the belief that life is something to debug?"

Your Mom made you breakfast tacos knowing you'd skip meals. She sees what your 0.9 attention can't - you're succeeding at everything except being."""

def get_dating_insight():
    """Get the dating-specific insight"""
    return """PATTERN DETECTED: You text dates like investor meetings

You texted Alexandra: "Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?"
You text investors: "Great meeting tonight. Let's schedule a follow-up. Are you free Tuesday 2pm?"

KEY INSIGHTS:
• Same template for both personal and professional
• 2-hour timeboxes for dates (7-9 PM)
• Treating intimacy as a transaction
• But: You texted her at 11:30 PM when exhausted (never do this for work)

The question: "Why do I need permission (a calendar slot) to experience joy?"

Try: Text her like you text your Mom - "Thinking about you" not "Are you free?"
One assumes relationship. The other negotiates it."""

def get_questions_not_asking():
    """Get the questions Adam should be asking himself"""
    return """Questions you're not asking yourself:

1. "When did I stop being a person and become a founder?"
   - Your identity is always modified: Adam-with, Adam-at, Adam-doing
   - Never just "Adam"

2. "Why do I schedule spontaneity?"
   - Even "inspiration struck" is timeboxed 11pm-12:30am

3. "What would happen if I texted Alexandra the way I text my Mom?"
   - To Mom: assumes continuity
   - To Alexandra: negotiates each interaction

4. "Am I building Ariata to remind others to live, because I've forgotten how?"
   - Your mission: "technology for human flourishing"
   - Your life: optimized for everything except flourishing

5. "Do I track everything because I'm afraid I'm not actually experiencing any of it?"
   - 63 events meticulously logged
   - But can you remember how the coffee tasted?"""

# Main server loop
log("Ariata Full Context Server starting...")

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
                        "version": "2.0.0"
                    }
                }
            })

        elif method == "initialized":
            log("Client initialized")

        elif method == "tools/list":
            send_message({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "tools": [
                        {
                            "name": "get_profound_insight",
                            "description": "Tell me something uniquely profound about my last week - analyzes deep patterns in Adam's life",
                            "inputSchema": {
                                "type": "object",
                                "properties": {},
                                "required": []
                            }
                        },
                        {
                            "name": "get_dating_insight",
                            "description": "Analyze how Adam texts dates vs investors - shows pattern of treating intimacy like transactions",
                            "inputSchema": {
                                "type": "object",
                                "properties": {},
                                "required": []
                            }
                        },
                        {
                            "name": "get_questions_not_asking",
                            "description": "What questions should Adam be asking himself that he's not?",
                            "inputSchema": {
                                "type": "object",
                                "properties": {},
                                "required": []
                            }
                        },
                        {
                            "name": "analyze_week_patterns",
                            "description": "Analyze patterns in Adam's week - time allocation, attention, priorities",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "focus": {
                                        "type": "string",
                                        "description": "Area to focus on: time, attention, communication, relationships",
                                        "enum": ["time", "attention", "communication", "relationships", "all"]
                                    }
                                },
                                "required": []
                            }
                        }
                    ]
                }
            })

        elif method == "prompts/list":
            send_message({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "prompts": []
                }
            })

        elif method == "resources/list":
            send_message({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "resources": []
                }
            })

        elif method == "tools/call":
            tool_name = request.get("params", {}).get("name")
            tool_args = request.get("params", {}).get("arguments", {})

            if tool_name == "get_profound_insight":
                send_message({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": get_profound_insight()
                        }]
                    }
                })

            elif tool_name == "get_dating_insight":
                send_message({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": get_dating_insight()
                        }]
                    }
                })

            elif tool_name == "get_questions_not_asking":
                send_message({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": get_questions_not_asking()
                        }]
                    }
                })

            elif tool_name == "analyze_week_patterns":
                focus = tool_args.get("focus", "all")

                if focus == "time":
                    analysis = f"""TIME ALLOCATION ANALYSIS:

Your stated values: "Family is everything", "Faith grounds me"
Your lived week: {WEEK_CONTEXT['time_allocation']['work']} work vs {WEEK_CONTEXT['time_allocation']['family']} family

You're living an inverted priority pyramid. The things you say matter most receive the least time."""

                elif focus == "attention":
                    analysis = f"""ATTENTION ANALYSIS:

Highest attention (0.9): Code improvements, money, technical breakthroughs
Moderate attention (0.7): Mass, date with Alexandra
Lowest attention (0.1): Sleep - the foundation of everything

Pattern: Attention inversely correlates with self-care. You're most present for transactions, least present for transformation."""

                elif focus == "communication":
                    analysis = f"""COMMUNICATION PATTERN:

To Alexandra: "{WEEK_CONTEXT['communication_patterns']['to_alexandra']}"
To investor: "{WEEK_CONTEXT['communication_patterns']['to_investor']}"
To Mom: "{WEEK_CONTEXT['communication_patterns']['to_mom']}"

You use the same templates for love and logistics. {WEEK_CONTEXT['communication_patterns']['pattern']}."""

                else:
                    analysis = f"""WEEK OVERVIEW:

{WEEK_CONTEXT['temporal_window']}
{WEEK_CONTEXT['total_events']} events tracked

Time: {WEEK_CONTEXT['time_allocation']['work']} work, {WEEK_CONTEXT['time_allocation']['spiritual']} spiritual
Attention: Highest for code (0.9), lowest for sleep (0.1)
Pattern: You've turned life into a sprint planning session

Most profound insight: {WEEK_CONTEXT['profound_insights'][0]}"""

                send_message({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": analysis
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