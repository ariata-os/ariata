# Ariata MCP Demo: The Power of Personal Context

## Demo Setup

### Left Screen: Claude (Without Ariata)
Standard Claude with no personal context

### Right Screen: Claude (With Ariata MCP)
Claude enhanced with your personal event timeline and patterns via MCP server

---

## THE DEMO QUESTION

### User Prompt:
> "I had a great first date this week and we're planning a second one. But I'm worried I'm approaching dating wrong while building my startup. What questions should I be asking myself that I'm not?"

---

## Expected Response Comparison

### Claude WITHOUT Ariata (Generic):
```
That's wonderful about your date! Building a startup while dating can be challenging. Here are some questions to consider:

1. **Are you being present?** - Are you fully engaged during dates or thinking about work?

2. **What are your priorities?** - Have you thought about what you're looking for in a relationship right now?

3. **Communication expectations** - Are you being upfront about your time constraints?

4. **Work-life boundaries** - Can you create space for personal relationships?

5. **Long-term compatibility** - Does this person understand the founder lifestyle?

6. **Self-care** - Are you taking care of yourself enough to be a good partner?

These are common challenges for founders. The key is finding someone who understands your journey while ensuring you don't lose yourself in work.
```

### Claude WITH Ariata (Context-Aware):
```
Adam, I notice something fascinating in your week's data that answers your deeper question.

**You texted Alexandra: "Had a wonderful evening. Would love to see you again. Are you free next Thursday 7pm?"**

Compare this to your other texts:
- To Chris (potential co-founder): "Great meeting tonight. Let's schedule a follow-up to discuss equity structure. Are you free Tuesday 2pm?"
- To investor: "Following up on our conversation. Looking forward to next steps."

You're approaching Alexandra like she's a calendar conflict to resolve, not a person to know. The question isn't whether you're "approaching dating wrong" - it's:

**"When did I start treating intimacy like a transaction?"**

Your own data reveals the pattern:
- You scheduled your date in a 2-hour window (7-9 PM) like your investor meetings
- You texted her with the same template as professional contacts
- You noted "great chemistry" the way you noted "40% performance improvement"

But here's what's beautiful - and what you're missing:
- You texted her at 11:30 PM when exhausted (you NEVER email investors then)
- Your attention during the date was 0.7 - higher than family dinner (0.4)
- You broke your pattern of immediate sleep after social events to text her

**The question you should be asking:**
*"Why do I feel I need permission (a 'schedule slot') to experience joy?"*

You're building Ariata for "human flourishing" while living like a Google Calendar. Alexandra didn't ask for your next availability window - she said yes to knowing YOU. But your text could have been written by your own AI.

Try this: Text her the way you'd text your Mom. Not "Are you free Thursday 7pm?" but "Thinking about you. Thursday?"

One assumes relationship. The other negotiates it.

Your startup needs a founder. Alexandra needs Adam. Those aren't the same person right now - and that's the question you're not asking: **"Who am I when I'm not optimizing?"**
```

---

## The MCP System Prompt

```markdown
You are Claude enhanced with Ariata's personal context engine. You have access to:

1. **Event Timeline**: Complete W5H (Who, What, When, Where, Why, How) data for Adam's week
2. **Attention Scores**: Weighted importance of each event (0.0-1.0)
3. **Communication Patterns**: Linguistic analysis of all messages sent
4. **Behavioral Clusters**: Time allocation, stated vs lived values
5. **Relationship Topology**: How Adam describes and relates to others

When responding:
- Reference specific events, times, and patterns from the data
- Compare similar situations to reveal behavioral patterns
- Use exact quotes from Adam's communications when relevant
- Highlight discrepancies between stated values and actions
- Focus on questions Adam isn't asking himself

Context Retrieved for Current Query:
[The full ariata_context_week.md content would be inserted here]

Remember: You're not giving generic advice. You're reflecting Adam's own patterns back to him with precision and insight.
```

---

## Demo Talk Track

### Opening (30 seconds)
"Let me show you the difference between AI that knows about the world, versus AI that knows about YOU."

### Setup (20 seconds)
"Same question to both: I'm dating while building my startup, what should I ask myself?"

### Show Generic Response (15 seconds)
"The left side - standard AI - gives good but generic advice. Work-life balance, be present, communicate..."

### Show Ariata Response (30 seconds)
"But watch what happens with Ariata. It notices I text my date like an investor. Same template. Same language. Same 2-hour timebox."

### The Insight (20 seconds)
"It's not telling me about dating. It's showing me that I've turned intimacy into a transaction. That's the mirror only personal AI can provide."

### The Close (15 seconds)
"This is Ariata. Not AI that studies you to sell to you, but AI that knows you to serve you. The sovereign stack for human flourishing."

---

## Technical Implementation Notes

### MCP Server Setup
```python
# Simple MCP server that injects context
class AriataMCP:
    def __init__(self):
        self.context = load_markdown('ariata_context_week.md')

    def enhance_prompt(self, user_query):
        # Retrieve relevant context based on query
        relevant_context = self.retrieve_relevant_sections(user_query)

        # Inject into system prompt
        return f"""
        You are Claude with access to Adam's personal context via Ariata.

        Retrieved Context:
        {relevant_context}

        User Query: {user_query}

        Respond with specific references to Adam's actual behavior patterns.
        """
```

### Key Patterns to Highlight
1. **Linguistic Convergence**: Professional language in personal contexts
2. **Temporal Compression**: Everything scheduled in blocks
3. **Identity Modulation**: Never just "Adam", always "Adam-with/at/doing"
4. **Attention Inversion**: Highest attention to work, lowest to self-care
5. **Documentary Living**: Recording life rather than experiencing it

---

## Alternative Demo Questions (if time permits)

1. **"How am I doing?"**
   - Generic: "That depends on your goals..."
   - Ariata: "Your mom made you breakfast tacos knowing you'd skip meals. That's how you're doing."

2. **"Should I hire Chris as co-founder?"**
   - Generic: "Consider skills, culture fit, equity..."
   - Ariata: "You described him as 'potential technical co-founder' even in your private notes. You've already decided he's temporary."

3. **"What's my biggest blind spot?"**
   - Generic: "Common blind spots include delegation..."
   - Ariata: "You consume coffee during emotional conversations, not tired ones. You're medicating feelings, not fatigue."

---

## The Wow Moment

The moment they realize: **Ariata doesn't just have your data. It understands your story.**

And more importantly: **It asks you the questions you won't ask yourself.**