#!/usr/bin/env python3
"""
Ariata MCP Server - Single profound insight tool for "last week" context
"""

import sys
import json
import signal


def send_message(msg):
    """Send a JSON-RPC message"""
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def log(msg):
    """Log to stderr for debugging"""
    sys.stderr.write(f"[Ariata] {msg}\n")
    sys.stderr.flush()


def get_week_insight():
    """The single, comprehensive insight about Adam's last week"""
    return """Based on your context from last week:

Narrative Timeline

Saturday, Dec 21st: Arrival and Contemplation in Houston
You landed at IAH in the early afternoon, a trip marked by a sense of purpose. The immediate agenda was twofold: a high-stakes investor meeting and the final arrangements for a significant personal acquisition. That evening, you attended Mass. The liturgy provided a profound and necessary respite—a moment of quiet contemplation and spiritual grounding amidst the week's building intensity. It was a stark reminder of your ultimate Telos, a moment to reconnect with the "why" before a week of executing the "how."

Sunday, Dec 22nd: Stewardship and Steel
Sunday morning was dedicated to a major milestone: you finalized the purchase of a new car. This wasn't merely a transaction but a tangible result of your hard work, immediately raising the internal dialectic you grapple with: "to whom much is given, much is expected." The feeling was a complex mix of joyful achievement and the weight of dutiful stewardship, a theme that would echo throughout the week.

Monday, Dec 23rd: The Drive to Austin & A Tale of Two Connections
After checking out, you began the drive to Austin. The open road offered time for reflection before a packed afternoon of meetings with friends, including Brian and Carol. That evening, you had a date with a woman named Sarah. While she was kind, the connection felt dissonant. You perceived a lack of conviction that stood in stark contrast to your own passionate intensity and faith. It was a saddening experience, highlighting your deep-seated need for a partner who shares a similar orientation toward a purposeful life, rooted in the belief that the world is a gift to be cherished and acted upon. This encounter underscored your core longing for Love & Familial Devotion built on shared values.

Tuesday, Dec 24th: Deep Work and a Spark of Novelty
The afternoon was a testament to your "Cyclical Focus & ADHD Dynamism," as you dove into a seven-hour, laser-focused coding session for Ariata, building the sovereign stack you believe in so deeply. Immediately following this period of intense, isolated work, you shifted gears and attended the Co-Founder Match event at Capital Factory. The party was a blur of ambition and networking, but one conversation cut through the noise. You met Bella, a young, sharp, and engaging investor. The initial meeting quickly transitioned to a drink across the street, where the conversation flowed effortlessly from venture capital and big data to the future of humanity and the role of suffering in growth. It was an exhilarating encounter—a meeting of minds that resonated with your "Passionate Innovator" archetype and satisfied the "lure of novelty" in a way that felt generative, not distracting.

Wednesday, Dec 25th: Mission Alignment and Onward
Your morning began with a 9:00 AM meeting with Matthew Sanders at Magisterium. This was a critical touchpoint to align on the deeper, philosophical underpinnings of your work with Ariata—fusing classical wisdom with technical precision. The conversation reinforced your mission to build tools for the "captain of the soul." Energized by this, you spent the afternoon tying up loose ends before making the drive to Dallas that evening, your mind already processing the week's events and planning the next strategic moves.

Thematic Analysis: The Lived vs. The Logged

The week's narrative reveals a profound tension between your stated Telos and your lived reality, a conflict between the "In-Person Adam" and the "Digital Adam."

The Sarah/Bella Dichotomy: Your two romantic encounters serve as a perfect allegory for your internal state. With Sarah, your authentic, passionate self felt out of place, leading to a sense of disconnect. With Bella, that same intensity was the very basis of the connection. This highlights that the issue isn't your passion but the context in which you deploy it. You thrive not just with intellectual equals, but with spiritual and philosophical counterparts.

The Paradox of Pious Productivity: You attended Mass and felt a deep, rejuvenating peace, acknowledging it as a "much-needed break." Yet, this spiritual "recharge" accounted for less than 1% of your logged week. In contrast, work-related activities consumed over 40%. The calendar data suggests that while you seek to serve God, your time overwhelmingly serves your company. This is the central battlefield of your week: the struggle to integrate your unshakeable faith with the relentless demands of your role as a Builder.

Vocation as an API Call: The critique from your "IR" data holds true. Your scheduling with Sarah mirrored an investor template, an attempt to optimize a human connection. This reveals a dangerous bleed-over from your professional life into your personal one. Your work at Ariata is to restore agency and fight against systems that turn people into predictable consumers, yet you risk applying that same dehumanizing, transactional logic to your own relationships.

The Gift and The Grind: The purchase of the car was a moment of celebration, but it immediately triggered your sense of immense obligation. This is the core of your primary dialectic. You are not simply a "Joyful Achiever"; you are a "Joyful Achiever in Service," and when the object of your service feels misaligned—or directed more towards metrics than meaning—it creates profound emotional and spiritual friction. The challenge is not to work less, but to ensure every hour worked is a prayerful act of dutiful and sacrificial stewardship, truly aimed at building the good, the true, and the beautiful.

In summary, the week was a microcosm of your life's greatest challenge: aligning your immense capacity for focused work (praxis) with your deep-seated moral and spiritual convictions (phronesis). You built momentum for your company, but the data suggests you are still architecting the life you truly want to live."""


def handle_shutdown(signum, frame):
    """Handle shutdown signals gracefully"""
    log("Shutting down gracefully...")
    sys.exit(0)


# Set up signal handlers
signal.signal(signal.SIGINT, handle_shutdown)
signal.signal(signal.SIGTERM, handle_shutdown)

# Main server loop
log("Ariata Week Context Server starting...")

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
                        "version": "5.0.0"
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
                            "name": "analyze_last_week",
                            "description": "Analyze context from last week to reveal profound insights about identity and patterns",
                            "inputSchema": {
                                "type": "object",
                                "properties": {},
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

            if tool_name == "analyze_last_week":
                send_message({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": get_week_insight()
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
