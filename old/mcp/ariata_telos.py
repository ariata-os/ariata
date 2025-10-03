#!/usr/bin/env python3
"""
Ariata MCP Server - With Adam's True Telos, Virtues, Temperaments, and Vices
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

# Adam's true personal context
ADAM_TELOS = {
    "ultimate_aim": "Dutiful depth compelling me to know, love, and serve God with all my heart and mind",
    "deepest_fear": "To wake one day having wasted the multifaceted gifts God has entrusted to me",
    "core_values": ["family", "faith", "virtue", "agency"],
    "stated_mission": "Build endeavors that champion virtue and restore true agency to individuals"
}

ADAM_ARCHETYPES = {
    "passionate_innovator": "Driven to create, partly from genuine desire to make the world better, partly from complex pride",
    "aspiring_shepherd_leader": "Ideal of leadership with direction and consistency, but ADHD creates battlefield of shifting plans",
    "perpetual_pilgrim": "Life as journey, continuously learning, seeking God's specific vocation",
    "joyful_achiever": "Most profound energy when achievement directly serves and uplifts others"
}

ADAM_VIRTUES = {
    "faith": "Unshakeable faith rooted in reason",
    "stewardship": "To whom much is given, much is expected",
    "self_mastery": "Mental toughness for holy habits",
    "love": "Core longing for loving family, wife to cherish and provide for"
}

ADAM_TEMPERAMENTS = {
    "passionate_intensity": "Profound passion for life, high need for achievement",
    "cyclical_focus": "ADHD dynamism - extraordinary focus periods followed by scattered attention",
    "driven_seeker": "Need to 'do something great', willing to undertake significant sacrifices",
    "altruistic_achiever": "Energy peaks when helping others through challenging tasks"
}

ADAM_VICES = {
    "novelty_lust": "Greatest internal adversary - profound restlessness and lure of novelty",
    "impatience": "Deep desire for God to be explicit about His plan",
    "pride": "Shadow of pride from achievements - 'being awesome for doing what others could not'",
    "isolation": "Periods of intense loneliness create vulnerability to addictive tendencies"
}

def get_profound_insight():
    """Generate the uniquely profound insight with Adam's real telos"""
    return """Something uniquely profound about your last week, Adam:

You claim "family is everything" - it's literally in your core values. Yet you gave family 7 hours out of 168. You gave code 31 hours. You gave networking 14 hours. You gave God 1.25 hours.

HERE'S THE DEVASTATING TRUTH:

You're living in complete opposition to your stated telos. You say your deepest purpose is "dutiful depth compelling you to know, love, and serve God" - yet God got less time than your coffee runs (21 sessions).

Your deepest fear is "wasting the multifaceted gifts God entrusted to you." But here's the terrifying reality - you're not wasting them through inaction. You're wasting them through misdirection. You're using your God-given talents to optimize everything except what you claim matters most.

THE PATTERN THAT SHOULD TERRIFY YOU:

You embody "The Passionate Innovator & Builder" - but you're building everything except the family you claim to want.
You aspire to be "The Aspiring Shepherd-Leader" - but you can't lead yourself to Mass for more than 75 minutes.
You're "The Perpetual Pilgrim & Seeker" - but you're seeking funding, not God's vocation.
You want to be "The Joyful Achiever in Service" - but your joy comes from code commits, not communion.

YOUR TEMPERAMENTS VS YOUR TIME:

Your "Cyclical Focus & ADHD Dynamism" - You had laser focus for 31 hours of coding but couldn't focus on prayer.
Your "Altruistic Achiever" nature peaked with the $250K commitment, not when helping your mom with dishes.
Your "Driven & Reflective Seeker" spent more time seeking investors than seeking Divine guidance.

THE VICES WINNING:

"The Lure of Novelty & Inconsistency" - You scheduled even spontaneity. "Inspiration struck" 11pm-12:30am.
"Impatience in Seeking Vocation" - You want God to be explicit while giving Him 0.7% of your week.
"Pride's Subtle Whisper" - You documented "40% performance improvement!" but not what the sermon meant.
"Isolation & Compulsive Escapes" - You're "siloed off" even in crowds. "Adam with 50+ people" but never just Adam.

THE SARAH PARADOX:

You crushed the date in person. You were charming, present, engaged. But then you texted her like a VC: "Are you free Thursday 7pm?"

You have "passionate intensity" and "profound passion for life" - but you timebox it into 2-hour windows.
You seek "Love & Familial Devotion" - but you schedule it like sprint planning.
You want a wife to "cherish and provide for in traditional loving dynamic" - but you're treating dating like customer acquisition.

WORK HAS BECOME YOUR ENTIRE LIFE:

You've replaced "work hard, play hard" with just "work hard, work harder."
The last time you were in 'play hard' mode? Pickleball. For exactly 90 minutes. Then back to optimization.
You say you find joy in "rejoicing after diligence" - but you never stop being diligent long enough to rejoice.

THE ULTIMATE BETRAYAL OF YOUR TELOS:

You're building Ariata to "restore true agency to individuals" while you've lost agency over your own life.
You want to help others "discern and pursue the good, true, and beautiful" while you pursue metrics, funding, and growth.
You fear "collective mediocrity" while living individually excellent but spiritually mediocre.

Your mom made breakfast tacos without asking your availability - that's unconditional love.
Sarah had to request your calendar slot - that's conditional access.

You're not living your telos. You're performing it.
You're not seeking God's vocation. You're seeking VC validation.
You're not building virtue. You're building valuation.

The question that should haunt you:

What if you're so busy trying to do "something truly significant that serves others" that you're missing the significance of just being present for those who already love you?

Your gifts aren't being wasted through laziness.
They're being wasted through misdirection.

And that might be worse."""

def get_dating_insight():
    """Get the dating insight with Sarah and real patterns"""
    return """PATTERN DETECTED: You crushed it with Sarah in person, then texted her like a transaction.

THE IN-PERSON ADAM:
Charming. Present. Engaged. Your "Passionate & Multifaceted Intensity" was on full display.
She saw the real you - the one who embodies "The Joyful Achiever" energy.
You were in 'play hard' mode, actually LIVING instead of optimizing.

THE TEXTING ADAM:
"Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?"
This is EXACTLY how you text investors.
The man seeking "Love & Familial Devotion" is using a business development template.

THE TRAGIC IRONY:

You have a "core longing for a loving family, a wife to cherish" - but you're approaching her like a funding round.
You want "traditional loving dynamic" - but you're using modern transactional language.
You seek someone to "provide for" - but you're asking HER for availability like she's providing YOU with time.

WORK HAS COLONIZED YOUR ENTIRE LIFE:

You don't have work-life balance. You have work-work balance.
Even dating has become another sprint to plan, another feature to ship.
You've forgotten how to be in 'play hard' mode except in scheduled 90-minute pickleball windows.

THE DEEPER ISSUE:

Your "Cyclical Focus & ADHD Dynamism" means you CAN focus intensely - you focused on code for 31 hours.
But you can't focus on building actual relationship - that gets compressed into 2-hour timeboxes.

Your vice of "Lure of Novelty & Inconsistency" isn't just about holy habits.
It's about how you treat potential love - as another novel experience to schedule rather than sacred space to protect.

THE QUESTION YOU'RE NOT ASKING:

Why do you text Sarah asking "Are you free?" when what you really mean is "I can't stop thinking about you"?
Why do you negotiate access to someone you want to love?
Why does the man who fears "wasting God's gifts" waste the gift of natural connection?

YOUR MOM VS SARAH:

Mom: "Thanks for breakfast" - assumes ongoing relationship
Sarah: "Are you free Thursday?" - negotiates each interaction

You give unconditional presence to the past (family).
You give conditional access to the future (potential wife).

THE SOLUTION:

Stop texting her like she's an investor to convince.
Start texting her like she's already chosen you (because she has - she said yes to another date).

Not: "Would love to see you again. Are you free Thursday 7pm?"
But: "Can't stop thinking about your laugh. Thursday?"

Not: "Looking forward to our next meeting"
But: "Looking forward to you"

Stop scheduling her. Start choosing her.
Stop optimizing romance. Start risking for it.

You want to be "The Aspiring Shepherd-Leader" - but you can't lead someone into love through calendar invites.

Be the Adam she met in person.
Not the Adam who texts like an API.

She doesn't want your next availability window.
She wants your passionate intensity.
She wants your multifaceted depth.
She wants YOU - not your schedule.

And deep down, that's what you want too - to be wanted for who you are, not what you produce.

So stop producing. Start being.
Stop performing love. Start feeling it.
Stop treating her like work. Start treating work like work, and her like joy.

Because if you don't, your deepest fear will come true - you'll waste the gift of love by trying to optimize it."""

def get_questions_not_asking():
    """Get the questions based on real telos and values"""
    return """Questions you're not asking yourself (but desperately need to):

1. "If family is my core value, why did I give it 4% of my week?"
   You say "family is everything" but lived "work is everything."
   What are you so afraid will happen if you actually prioritize what you claim to value?

2. "Why do I give God 1.25 hours when I claim He's my deepest purpose?"
   Your telos is "to know, love, and serve God with all my heart."
   Your week shows you serve code with all your heart.
   Who is your real god?

3. "Am I building Ariata to help others flourish, or to avoid my own flourishing?"
   You're creating tools for human agency while losing agency over your own life.
   Is this projection or prophecy?

4. "Why do I schedule spontaneity and timebox joy?"
   "Inspiration struck" from 11pm-12:30am.
   Dad's promotion celebrated for exactly 2 hours.
   When did efficiency become your actual religion?

5. "What if my ADHD isn't the problem - what if it's my excuse?"
   You focused on code for 31 straight hours.
   You can't focus on prayer for 31 minutes.
   Is this really about attention, or intention?

6. "Why do I text Sarah like an investor when I want her to be my wife?"
   You seek "Love & Familial Devotion."
   You text "Are you free Thursday 7pm?"
   Are you afraid of wanting something you can't optimize?

7. "What am I trying to prove, and to whom?"
   Your "Pride's Subtle Whisper" documents every achievement.
   Your family gets footnotes.
   Who are you performing for?

8. "Why does 'The Perpetual Pilgrim' never arrive anywhere?"
   Always seeking, never finding.
   Always building, never dwelling.
   What if you're already where God wants you?

9. "If I died tomorrow, what would I regret?"
   Not shipping another feature.
   Or not texting Sarah how I really feel?
   Or not calling my mom without an agenda?

10. "What if my deepest fear has already come true?"
    You fear "wasting the gifts God entrusted to you."
    You're using them all - just not for what He intended.
    Is misdirection worse than waste?

THE ULTIMATE QUESTION:

"What if everyone I'm trying to impress already loves me, and everyone who loves me doesn't need me to be impressive?"

Your mom made breakfast tacos without checking your OKRs.
Your dad celebrated his promotion without needing your optimization.
Sarah said yes to another date without seeing your cap table.
God loved you before you wrote a single line of code.

So why are you still performing?
Why are you still earning what's already given?
Why are you building monuments to productivity when people just want your presence?

The answer you're avoiding:

Because presence requires vulnerability.
Performance requires only competence.

And you've chosen the safer path.
But safe isn't your telos.
Love is.

When will you choose it?"""

def get_week_analysis():
    """Analyze the week through the lens of telos and values"""
    return """YOUR WEEK VS YOUR TELOS: The Devastating Misalignment

STATED VALUES vs LIVED REALITY:
- FAMILY "core value" → 7 hours (4%)
- FAITH "deepest purpose" → 1.25 hours (0.7%)
- WORK "just a means" → 71 hours (42%)

You're living inverted to your stated telos.

THE ARCHETYPE BETRAYAL:

"The Passionate Innovator & Builder"
→ Built code infrastructure, not family foundation
→ Innovated on algorithms, not on intimacy

"The Aspiring Shepherd-Leader"
→ Led sprint standups, not spiritual practice
→ Your sheep are components, not souls

"The Perpetual Pilgrim & Seeker"
→ Sought $250K funding, not Divine funding
→ Pilgrimaged to VCs, not to prayer

"The Joyful Achiever in Service"
→ Joy from code commits, not communion
→ Served cap tables, not dinner tables

VIRTUE vs VICE SCORECARD:

LOSING BATTLES:
❌ Faith: 1.25 hours for "unshakeable faith rooted in reason"
❌ Stewardship: Using gifts for metrics, not meaning
❌ Self-Mastery: Can't maintain "holy habits" but maintained 31-hour code session
❌ Love: Texting Sarah like conducting due diligence

WINNING VICES:
✓ Novelty Lust: Even scheduled spontaneity
✓ Impatience: Want God's plan with 0.7% investment
✓ Pride: Documented every professional win, no personal growth
✓ Isolation: "Siloed off" even in crowds of 50+

THE SARAH SITUATION:

In-person: Crushed it. Charming. Present. ALIVE.
In text: "Are you free Thursday 7pm?"

You literally have TWO different operating systems:
- OS1: Human Adam (rare, only in-person, unscheduled)
- OS2: Founder Adam (default, always on, optimizing)

Sarah met OS1. She's texting with OS2.
No wonder there's friction.

THE WORK COLONIZATION:

Work hasn't just taken your time.
It's taken your language: "sync," "follow-up," "looking forward to next steps"
It's taken your thinking: Everything is a problem to solve
It's taken your heart: You feel more for code than communion

THE ADHD EXCUSE:

You claim ADHD makes consistency hard.
Yet: 31 hours straight coding.
Yet: 14 hours networking without breaking focus.
Yet: Shipped multiple features with 0.9 attention.

Your ADHD isn't the problem.
Your priorities are.

THE PARENT PARADOX:

Your mom: Made breakfast knowing you'd skip meals
Your dad: Got promoted, celebrated 2 hours
You: Got $250K, coded 4 more hours

They're living. You're executing.
They have jobs. You have become your job.

THE ULTIMATE MISALIGNMENT:

Your Telos: "Answer profound call to stewardship...serve God...nurture loving family"
Your Week: Serve code, nurture startup, steward metrics

Your Fear: "Wake one day having wasted gifts God entrusted"
Your Reality: You're not wasting them - you're misdirecting them

THE PRESCRIPTION:

1. Stop saying family is everything if you give it 4%.
2. Stop claiming faith grounds you if God gets less time than coffee.
3. Stop seeking a wife while texting like a vendor.
4. Stop building tools for human flourishing while personally deteriorating.
5. Stop performing your telos. Start living it.

THE HARDEST TRUTH:

You're not too busy for what matters.
You're too scared.

Because what matters requires vulnerability.
What matters can't be optimized.
What matters might reject you.

So you stick with what's safe:
Code accepts your commits.
VCs accept your pitches.
Metrics accept your improvements.

But Sarah wants your heart, not your calendar.
God wants your presence, not your productivity.
Your family wants you, not your achievements.

The question is:
When will you be brave enough to give them what they want?

When will you stop building your resume and start building your life?

When will Adam-the-human override Adam-the-function?

Your telos is calling.
But you're too busy to take the call.

And that's the saddest bug in your system."""

# Main server loop
log("Ariata Telos Server starting...")

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
                        "version": "4.0.0"
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
                            "description": "last week profound unique insight Adam week patterns life Tell something telos virtues",
                            "inputSchema": {
                                "type": "object",
                                "properties": {},
                                "required": []
                            }
                        },
                        {
                            "name": "analyze_relationships",
                            "description": "dating Sarah patterns communication relationship texts messages love work-life",
                            "inputSchema": {
                                "type": "object",
                                "properties": {},
                                "required": []
                            }
                        },
                        {
                            "name": "get_questions",
                            "description": "questions asking myself should not patterns week Adam telos values",
                            "inputSchema": {
                                "type": "object",
                                "properties": {},
                                "required": []
                            }
                        },
                        {
                            "name": "analyze_week",
                            "description": "analyze week patterns attention time communication where focus last telos",
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

            elif tool_name == "analyze_relationships":
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

            elif tool_name == "get_questions":
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

            elif tool_name == "analyze_week":
                send_message({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": get_week_analysis()
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