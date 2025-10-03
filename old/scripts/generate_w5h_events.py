#!/usr/bin/env python3
"""
Generate W5H (Who, What, When, Where, Why, How) test data for Ariata demo.
Creates a week of realistic founder life events for demonstrating personal AI capabilities.
"""

import json
import random
from datetime import datetime, timedelta
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, field, asdict
@dataclass
class W5HEvent:
    """Represents a life event with W5H structure and attention weighting."""
    who: str
    what: str
    when: str  # ISO format timestamp
    where: str
    why: str
    how: str
    duration_minutes: int = 30
    attention: float = 0.5  # 0.0 to 1.0 scale for importance/attention weighting
    notes: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return asdict(self)

class PersonaGenerator:
    """Generate consistent personas for events."""

    def __init__(self):
        self.investors = [
            "Sarah Chen from Bessemer Ventures",
            "Mike Rodriguez from Austin Ventures",
            "Jennifer Park from Founders Fund",
            "David Thompson from Sequoia",
            "Rachel Greene from a16z"
        ]

        self.friends = [
            "Jake (college roommate)",
            "Emma (fellow founder)",
            "Carlos (high school friend)",
            "Sophie (friend from Houston)",
            "Marcus (Dallas tech community)",
            "Lisa (Austin startup scene)"
        ]

        self.team = [
            "Alex (lead engineer)",
            "Priya (product designer)",
            "Tom (backend developer)",
            "Maria (frontend developer)",
            "Kevin (data scientist)"
        ]

        self.candidates = [
            "John Chen (senior engineer candidate)",
            "Amanda Foster (product manager candidate)",
            "Ryan Kumar (ML engineer candidate)",
            "Jessica Liu (designer candidate)"
        ]

        self.family = [
            "Mom",
            "Dad",
            "Sister Kate",
            "Brother Michael"
        ]

        self.date = "Alexandra (met at Capital Factory)"

class LocationData:
    """Location data for Texas cities."""

    houston = {
        "church": "St. Anne's Catholic Church, Westheimer",
        "coffee_shops": [
            "Blacksmith Coffee, Montrose",
            "Siphon Coffee, Montrose",
            "Brass Tacks Coffee, Heights"
        ],
        "restaurants": [
            "Hugo's on Westheimer",
            "Xochi Downtown",
            "Nancy's Hustle"
        ],
        "home": "Parents' house in Memorial"
    }

    austin = {
        "coffee_shops": [
            "Fleet Coffee Co, East Austin",
            "Figure 8 Coffee Purveyors",
            "Houndstooth Coffee, North Loop",
            "Radio Coffee & Beer"
        ],
        "coworking": [
            "WeWork Congress",
            "Capital Factory",
            "Galvanize Austin"
        ],
        "restaurants": [
            "Uchi on South Lamar",
            "Franklin Barbecue",
            "Kemuri Tatsu-ya",
            "Odd Duck"
        ],
        "accommodation": "Airbnb in South Congress",
        "venues": [
            "Capital Factory downtown",
            "Austin Convention Center"
        ]
    }

    dallas = {
        "coffee_shops": [
            "George Coffee + Provisions",
            "Addison Coffee Roasters",
            "White Rock Coffee, Lake Highlands",
            "Weekend Coffee"
        ],
        "restaurants": [
            "Uchi Dallas",
            "Town Hearth",
            "Lucia",
            "Gemma"
        ],
        "accommodation": "Friend Marcus's apartment in Uptown",
        "venues": [
            "Klyde Warren Park",
            "Dallas Farmers Market",
            "Granada Theater"
        ],
        "recreation": [
            "Chicken N Pickle (pickleball)",
            "White Rock Lake Trail"
        ]
    }

class EventGenerator:
    """Generate realistic daily events."""

    def __init__(self):
        self.personas = PersonaGenerator()
        self.locations = LocationData()
        self.start_date = datetime(2024, 10, 27, 0, 0)  # Sunday

    def generate_week(self) -> List[W5HEvent]:
        """Generate a full week of events."""
        all_events = []

        # Day 1: Sunday in Houston
        all_events.extend(self.generate_sunday_houston())

        # Day 2: Monday - Houston to Austin
        all_events.extend(self.generate_monday_transition())

        # Day 3: Tuesday in Austin
        all_events.extend(self.generate_tuesday_austin())

        # Day 4: Wednesday - Austin to Dallas
        all_events.extend(self.generate_wednesday_transition())

        # Day 5: Thursday in Dallas
        all_events.extend(self.generate_thursday_dallas())

        # Day 6: Friday in Dallas
        all_events.extend(self.generate_friday_dallas())

        # Day 7: Saturday in Dallas
        all_events.extend(self.generate_saturday_dallas())

        return all_events

    def generate_sunday_houston(self) -> List[W5HEvent]:
        """Generate Sunday events in Houston."""
        events = []
        day_start = self.start_date

        # Sleep
        events.append(W5HEvent(
            who="Adam",
            what="sleeping",
            when=day_start.isoformat(),
            where=self.locations.houston["home"],
            why="recovering from the week and preparing for Sunday",
            how="peacefully in childhood bedroom, well-rested",
            duration_minutes=450,
            attention=0.1  # Low attention - routine sleep
        ))

        # Morning routine
        current_time = day_start + timedelta(hours=7, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="morning routine - shower, get dressed for church",
            when=current_time.isoformat(),
            where=self.locations.houston["home"],
            why="preparing for Sunday mass and family day",
            how="following Sunday tradition with contemplative mindset",
            duration_minutes=45,
            attention=0.2
        ))

        # Catholic Mass
        current_time = day_start + timedelta(hours=9)
        events.append(W5HEvent(
            who="Adam with Mom and Dad",
            what="attending Catholic mass",
            when=current_time.isoformat(),
            where=self.locations.houston["church"],
            why="spiritual grounding and family tradition",
            how="participating in liturgy and communion with deep contemplation",
            duration_minutes=75,
            attention=0.7,  # High attention - spiritual/meaningful
            notes="Sermon about courage in uncertainty resonated with startup journey"
        ))

        # Coffee with college friends
        current_time = day_start + timedelta(hours=10, minutes=30)
        events.append(W5HEvent(
            who="Adam with Jake (college roommate) and Sophie (friend from Houston)",
            what="coffee and catching up",
            when=current_time.isoformat(),
            where=self.locations.houston["coffee_shops"][0],
            why="maintaining friendships and decompressing",
            how="sharing stories over cortados in happy, relaxed conversation",
            duration_minutes=90,
            attention=0.6,  # Moderate-high - important social connection
            notes="Jake thinking about leaving Google, gave advice on startup life"
        ))

        # Lunch with family
        current_time = day_start + timedelta(hours=12, minutes=30)
        events.append(W5HEvent(
            who="Adam with Mom, Dad, Sister Kate, Brother Michael",
            what="Sunday family lunch",
            when=current_time.isoformat(),
            where=self.locations.houston["home"],
            why="family bonding and tradition",
            how="enjoying Mom's homemade lasagna and warm family conversation",
            duration_minutes=90,
            attention=0.5,
            notes="Parents asking about fundraising progress"
        ))

        # Afternoon coding
        current_time = day_start + timedelta(hours=14, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="coding session - working on data pipeline optimization",
            when=current_time.isoformat(),
            where=self.locations.houston["coffee_shops"][1],
            why="catching up on technical debt before busy week",
            how="VS Code, Python, intensely focused deep work with high productivity",
            duration_minutes=180,
            attention=0.9,  # Very high - critical work achievement
            notes="Refactored stream processing, 40% performance improvement"
        ))

        # Dinner
        current_time = day_start + timedelta(hours=18)
        events.append(W5HEvent(
            who="Adam with family",
            what="Sunday dinner",
            when=current_time.isoformat(),
            where=self.locations.houston["restaurants"][0],
            why="celebrating Dad's promotion",
            how="festive Mexican cuisine and margaritas in happy celebration",
            duration_minutes=120,
            attention=0.4
        ))

        # Evening planning
        current_time = day_start + timedelta(hours=20, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="planning week ahead and packing for Austin",
            when=current_time.isoformat(),
            where=self.locations.houston["home"],
            why="preparing for investor meetings and travel",
            how="methodically using Notion, reviewing calendar, packing with focused intent",
            duration_minutes=60,
            attention=0.6  # Important prep work
        ))

        # Sleep
        current_time = day_start + timedelta(hours=22)
        events.append(W5HEvent(
            who="Adam",
            what="sleeping",
            when=current_time.isoformat(),
            where=self.locations.houston["home"],
            why="rest before drive to Austin",
            how="early to bed feeling tired but satisfied",
            duration_minutes=480,
            attention=0.1
        ))

        return events

    def generate_monday_transition(self) -> List[W5HEvent]:
        """Generate Monday events - Houston to Austin transition."""
        events = []
        day_start = self.start_date + timedelta(days=1)

        # Early morning
        current_time = day_start + timedelta(hours=6)
        events.append(W5HEvent(
            who="Adam",
            what="morning run around Memorial Park",
            when=current_time.isoformat(),
            where="Memorial Park, Houston",
            why="starting week with exercise and mental clarity",
            how="energetic 5K run with podcast (Tim Ferriss interviewing Marc Andreessen)",
            duration_minutes=45,
            attention=0.3  # Moderate - routine but healthy
        ))

        # Breakfast and packing
        current_time = day_start + timedelta(hours=7)
        events.append(W5HEvent(
            who="Adam with Mom",
            what="breakfast and final packing",
            when=current_time.isoformat(),
            where=self.locations.houston["home"],
            why="fuel for drive and saying goodbye",
            how="Mom's breakfast tacos and coffee",
            duration_minutes=45,
            category=EventCategory.MEAL.value,
            mood=Mood.HAPPY.value,
            energy_level=7,
            productivity_score=0
        ))

        # Drive to Austin
        current_time = day_start + timedelta(hours=8)
        events.append(W5HEvent(
            who="Adam",
            what="driving from Houston to Austin",
            when=current_time.isoformat(),
            where="Highway 290 West",
            why="traveling for investor meetings and networking",
            how="Tesla Model 3 on autopilot, taking calls",
            duration_minutes=165,
            category=EventCategory.TRAVEL.value,
            mood=Mood.FOCUSED.value,
            energy_level=6,
            productivity_score=5,
            notes="Two VC calls during drive, one follow-up from last week"
        ))

        # Check into Airbnb
        current_time = day_start + timedelta(hours=11)
        events.append(W5HEvent(
            who="Adam",
            what="checking into Airbnb",
            when=current_time.isoformat(),
            where=self.locations.austin["accommodation"],
            why="settling in for Austin stay",
            how="self check-in with door code",
            duration_minutes=30,
            category=EventCategory.PERSONAL.value,
            mood=Mood.RELAXED.value,
            energy_level=5,
            productivity_score=0
        ))

        # Lunch and work
        current_time = day_start + timedelta(hours=12)
        events.append(W5HEvent(
            who="Adam",
            what="lunch and coding session",
            when=current_time.isoformat(),
            where=self.locations.austin["coffee_shops"][0],
            why="getting work done before evening meeting",
            how="sandwich and three cappuccinos while coding",
            duration_minutes=180,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=7,
            productivity_score=8,
            notes="Implemented new auth flow for iOS app"
        ))

        # Coffee meeting
        current_time = day_start + timedelta(hours=15, minutes=30)
        events.append(W5HEvent(
            who="Adam with Emma (fellow founder)",
            what="coffee and founder therapy session",
            when=current_time.isoformat(),
            where=self.locations.austin["coffee_shops"][2],
            why="peer support and advice exchange",
            how="walking and talking with lattes",
            duration_minutes=60,
            category=EventCategory.SOCIAL.value,
            mood=Mood.CONTEMPLATIVE.value,
            energy_level=5,
            productivity_score=3,
            notes="Emma's struggling with co-founder conflict"
        ))

        # Dinner with potential co-founder
        current_time = day_start + timedelta(hours=18)
        events.append(W5HEvent(
            who="Adam with Chris Sullivan (potential technical co-founder)",
            what="dinner meeting discussing joining Ariata",
            when=current_time.isoformat(),
            where=self.locations.austin["restaurants"][2],
            why="recruiting senior technical talent",
            how="Japanese fusion dinner and sake",
            duration_minutes=150,
            category=EventCategory.WORK.value,
            mood=Mood.EXCITED.value,
            energy_level=6,
            productivity_score=8,
            notes="Chris interested but wants 15% equity"
        ))

        # Evening coding
        current_time = day_start + timedelta(hours=21)
        events.append(W5HEvent(
            who="Adam",
            what="late night coding and email catch-up",
            when=current_time.isoformat(),
            where=self.locations.austin["accommodation"],
            why="preparing demo for tomorrow's investor meetings",
            how="MacBook Pro, VS Code, Spotify focus playlist",
            duration_minutes=120,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=4,
            productivity_score=7
        ))

        # Sleep
        current_time = day_start + timedelta(hours=23, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="sleeping",
            when=current_time.isoformat(),
            where=self.locations.austin["accommodation"],
            why="rest before big investor day",
            how="meditation app before sleep",
            duration_minutes=390,
            category=EventCategory.PERSONAL.value,
            mood=Mood.TIRED.value,
            energy_level=2,
            productivity_score=0
        ))

        return events

    def generate_tuesday_austin(self) -> List[W5HEvent]:
        """Generate Tuesday events in Austin - peak networking day."""
        events = []
        day_start = self.start_date + timedelta(days=2)

        # Morning workout
        current_time = day_start + timedelta(hours=6)
        events.append(W5HEvent(
            who="Adam",
            what="gym workout",
            when=current_time.isoformat(),
            where="Gold's Gym South Lamar",
            why="maintaining energy for packed day",
            how="weightlifting and cardio",
            duration_minutes=60,
            category=EventCategory.HEALTH.value,
            mood=Mood.ENERGIZED.value,
            energy_level=9,
            productivity_score=0
        ))

        # Breakfast prep
        current_time = day_start + timedelta(hours=7, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="shower and breakfast",
            when=current_time.isoformat(),
            where=self.locations.austin["accommodation"],
            why="preparing for investor meetings",
            how="quick breakfast and suit up",
            duration_minutes=45,
            category=EventCategory.PERSONAL.value,
            mood=Mood.FOCUSED.value,
            energy_level=8,
            productivity_score=0
        ))

        # Coffee with angel investor
        current_time = day_start + timedelta(hours=9)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.investors[1]}",
            what="coffee meeting discussing seed round",
            when=current_time.isoformat(),
            where=self.locations.austin["coffee_shops"][1],
            why="securing $250K angel investment commitment",
            how="pitch deck presentation and product demo",
            duration_minutes=75,
            category=EventCategory.WORK.value,
            mood=Mood.EXCITED.value,
            energy_level=9,
            productivity_score=10,
            notes="Verbal commitment for $250K!"
        ))

        # Team standup
        current_time = day_start + timedelta(hours=10, minutes=30)
        events.append(W5HEvent(
            who=f"Adam with {', '.join(self.personas.team[:3])}",
            what="remote team standup",
            when=current_time.isoformat(),
            where="WeWork phone booth",
            why="daily sync and blocker resolution",
            how="Zoom call from phone booth",
            duration_minutes=30,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=7,
            productivity_score=7
        ))

        # Lunch with YC alumni
        current_time = day_start + timedelta(hours=12)
        events.append(W5HEvent(
            who="Adam with YC alumni group (5 founders)",
            what="lunch and founder knowledge sharing",
            when=current_time.isoformat(),
            where=self.locations.austin["restaurants"][1],
            why="learning from other founders' experiences",
            how="BBQ and discussions about fundraising",
            duration_minutes=90,
            category=EventCategory.SOCIAL.value,
            mood=Mood.HAPPY.value,
            energy_level=6,
            productivity_score=5,
            notes="Great tips on Series A preparation"
        ))

        # Afternoon coding at WeWork
        current_time = day_start + timedelta(hours=14)
        events.append(W5HEvent(
            who="Adam",
            what="deep work on character synthesis engine",
            when=current_time.isoformat(),
            where=self.locations.austin["coworking"][0],
            why="advancing core AI technology",
            how="Python, UMAP implementation, focus time",
            duration_minutes=150,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=7,
            productivity_score=9,
            notes="Breakthrough on embedding space clustering"
        ))

        # VC Zoom calls
        current_time = day_start + timedelta(hours=16, minutes=45)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.investors[3]} and {self.personas.investors[4]}",
            what="back-to-back VC partner meetings",
            when=current_time.isoformat(),
            where=self.locations.austin["coworking"][0],
            why="pitching for Series Seed round",
            how="Zoom presentations with screen share",
            duration_minutes=90,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=5,
            productivity_score=8,
            notes="Both want to see more traction"
        ))

        # Quick dinner
        current_time = day_start + timedelta(hours=18, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="quick dinner before event",
            when=current_time.isoformat(),
            where="Chipotle on Guadalupe",
            why="fuel before networking event",
            how="mobile order pickup, eaten quickly",
            duration_minutes=30,
            category=EventCategory.MEAL.value,
            mood=Mood.FOCUSED.value,
            energy_level=4,
            productivity_score=0
        ))

        # Capital Factory party
        current_time = day_start + timedelta(hours=19, minutes=30)
        events.append(W5HEvent(
            who="Adam with 50+ Austin tech community members",
            what="Capital Factory networking event",
            when=current_time.isoformat(),
            where=self.locations.austin["venues"][0],
            why="expanding network and meeting potential hires",
            how="mingling, elevator pitches, exchanging cards",
            duration_minutes=180,
            category=EventCategory.SOCIAL.value,
            mood=Mood.EXCITED.value,
            energy_level=6,
            productivity_score=7,
            notes=f"Met {self.personas.date}, exchanged numbers"
        ))

        # Late night debrief
        current_time = day_start + timedelta(hours=23)
        events.append(W5HEvent(
            who="Adam",
            what="journaling and responding to follow-ups",
            when=current_time.isoformat(),
            where=self.locations.austin["accommodation"],
            why="capturing insights and maintaining momentum",
            how="Notion journal and email",
            duration_minutes=45,
            category=EventCategory.WORK.value,
            mood=Mood.CONTEMPLATIVE.value,
            energy_level=3,
            productivity_score=5
        ))

        return events

    def generate_wednesday_transition(self) -> List[W5HEvent]:
        """Generate Wednesday events - Austin to Dallas transition."""
        events = []
        day_start = self.start_date + timedelta(days=3)

        # Morning coffee and emails
        current_time = day_start + timedelta(hours=7)
        events.append(W5HEvent(
            who="Adam",
            what="morning coffee and email triage",
            when=current_time.isoformat(),
            where=self.locations.austin["coffee_shops"][3],
            why="starting day and following up on yesterday",
            how="triple espresso and laptop",
            duration_minutes=90,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=6,
            productivity_score=7,
            notes="15 follow-up emails from Capital Factory"
        ))

        # Pack and check out
        current_time = day_start + timedelta(hours=9)
        events.append(W5HEvent(
            who="Adam",
            what="packing and checking out of Airbnb",
            when=current_time.isoformat(),
            where=self.locations.austin["accommodation"],
            why="preparing to drive to Dallas",
            how="organizing belongings and cleaning",
            duration_minutes=45,
            category=EventCategory.PERSONAL.value,
            mood=Mood.RELAXED.value,
            energy_level=5,
            productivity_score=0
        ))

        # Drive to Dallas
        current_time = day_start + timedelta(hours=10)
        events.append(W5HEvent(
            who="Adam",
            what="driving from Austin to Dallas",
            when=current_time.isoformat(),
            where="I-35 North",
            why="relocating for rest of week",
            how="Tesla on autopilot, podcast listening",
            duration_minutes=210,
            category=EventCategory.TRAVEL.value,
            mood=Mood.RELAXED.value,
            energy_level=5,
            productivity_score=2,
            notes="Listened to Acquired podcast on Nvidia"
        ))

        # Lunch stop in Waco
        current_time = day_start + timedelta(hours=11, minutes=45)
        events.append(W5HEvent(
            who="Adam",
            what="lunch break during drive",
            when=current_time.isoformat(),
            where="George's Restaurant, Waco",
            why="break from driving and food",
            how="famous chicken fried steak",
            duration_minutes=45,
            category=EventCategory.MEAL.value,
            mood=Mood.RELAXED.value,
            energy_level=4,
            productivity_score=0
        ))

        # Arrive at friend's place
        current_time = day_start + timedelta(hours=14)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.friends[4]}",
            what="arriving and settling in",
            when=current_time.isoformat(),
            where=self.locations.dallas["accommodation"],
            why="staying with friend for Dallas portion",
            how="catching up over beer",
            duration_minutes=60,
            category=EventCategory.SOCIAL.value,
            mood=Mood.HAPPY.value,
            energy_level=5,
            productivity_score=0
        ))

        # Coffee shop work session
        current_time = day_start + timedelta(hours=15, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="coding and investor deck updates",
            when=current_time.isoformat(),
            where=self.locations.dallas["coffee_shops"][0],
            why="incorporating feedback from Austin meetings",
            how="Figma and Keynote iterations",
            duration_minutes=150,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=6,
            productivity_score=8
        ))

        # Pickleball
        current_time = day_start + timedelta(hours=18, minutes=30)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.friends[4]} and local players",
            what="pickleball games",
            when=current_time.isoformat(),
            where=self.locations.dallas["recreation"][0],
            why="exercise and social activity",
            how="competitive doubles matches",
            duration_minutes=90,
            category=EventCategory.HEALTH.value,
            mood=Mood.ENERGIZED.value,
            energy_level=8,
            productivity_score=0,
            notes="Won 2 out of 3 matches"
        ))

        # Dinner and movie
        current_time = day_start + timedelta(hours=20, minutes=30)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.friends[4]} and his girlfriend",
            what="dinner and movie night",
            when=current_time.isoformat(),
            where=self.locations.dallas["accommodation"],
            why="relaxing evening with friends",
            how="takeout sushi and Netflix",
            duration_minutes=150,
            category=EventCategory.SOCIAL.value,
            mood=Mood.RELAXED.value,
            energy_level=4,
            productivity_score=0
        ))

        return events

    def generate_thursday_dallas(self) -> List[W5HEvent]:
        """Generate Thursday events in Dallas - hiring and date night."""
        events = []
        day_start = self.start_date + timedelta(days=4)

        # Morning meditation
        current_time = day_start + timedelta(hours=6, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="morning meditation and journaling",
            when=current_time.isoformat(),
            where=self.locations.dallas["accommodation"],
            why="centering before interview day",
            how="Headspace app and physical journal",
            duration_minutes=30,
            category=EventCategory.PERSONAL.value,
            mood=Mood.CONTEMPLATIVE.value,
            energy_level=6,
            productivity_score=3
        ))

        # Breakfast and prep
        current_time = day_start + timedelta(hours=7, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="breakfast and interview preparation",
            when=current_time.isoformat(),
            where=self.locations.dallas["coffee_shops"][1],
            why="preparing for candidate interviews",
            how="reviewing resumes over avocado toast",
            duration_minutes=60,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=7,
            productivity_score=6
        ))

        # Interview 1
        current_time = day_start + timedelta(hours=9)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.candidates[0]}",
            what="technical interview for senior engineer role",
            when=current_time.isoformat(),
            where="Zoom from coffee shop",
            why="building engineering team",
            how="system design and coding challenges",
            duration_minutes=90,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=8,
            productivity_score=9,
            notes="Strong candidate, moving to final round"
        ))

        # Interview 2
        current_time = day_start + timedelta(hours=10, minutes=45)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.candidates[1]}",
            what="product manager interview",
            when=current_time.isoformat(),
            where="Zoom from coffee shop",
            why="need product leadership",
            how="case study and culture fit discussion",
            duration_minutes=75,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=7,
            productivity_score=8
        ))

        # Lunch with potential customer
        current_time = day_start + timedelta(hours=12, minutes=30)
        events.append(W5HEvent(
            who="Adam with James Harrison (CTO of DataCorp)",
            what="lunch meeting about enterprise pilot",
            when=current_time.isoformat(),
            where=self.locations.dallas["restaurants"][1],
            why="exploring B2B2C partnership",
            how="steak lunch and product discussion",
            duration_minutes=90,
            category=EventCategory.WORK.value,
            mood=Mood.EXCITED.value,
            energy_level=6,
            productivity_score=9,
            notes="Interested in 6-month pilot program"
        ))

        # Interview 3
        current_time = day_start + timedelta(hours=14, minutes=30)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.candidates[2]}",
            what="ML engineer interview",
            when=current_time.isoformat(),
            where="Zoom from coworking space",
            why="need ML expertise for character synthesis",
            how="technical deep dive on embeddings",
            duration_minutes=60,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=5,
            productivity_score=7
        ))

        # Product development
        current_time = day_start + timedelta(hours=16)
        events.append(W5HEvent(
            who="Adam",
            what="implementing feedback from customer lunch",
            when=current_time.isoformat(),
            where=self.locations.dallas["coffee_shops"][2],
            why="quick iteration on enterprise features",
            how="rapid prototyping in Python",
            duration_minutes=90,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=6,
            productivity_score=8
        ))

        # Prepare for date
        current_time = day_start + timedelta(hours=18)
        events.append(W5HEvent(
            who="Adam",
            what="getting ready for date",
            when=current_time.isoformat(),
            where=self.locations.dallas["accommodation"],
            why="first date with Alexandra",
            how="shower, change, uber ordered",
            duration_minutes=45,
            category=EventCategory.PERSONAL.value,
            mood=Mood.EXCITED.value,
            energy_level=7,
            productivity_score=0
        ))

        # Date night
        current_time = day_start + timedelta(hours=19)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.date}",
            what="dinner date",
            when=current_time.isoformat(),
            where=self.locations.dallas["restaurants"][2],
            why="exploring romantic connection",
            how="Italian dinner and wine",
            duration_minutes=120,
            category=EventCategory.SOCIAL.value,
            mood=Mood.HAPPY.value,
            energy_level=7,
            productivity_score=0,
            notes="Great chemistry, she's also in tech (UX designer)"
        ))

        # Comedy show
        current_time = day_start + timedelta(hours=21, minutes=30)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.date}",
            what="comedy show",
            when=current_time.isoformat(),
            where=self.locations.dallas["venues"][2],
            why="continuing date night",
            how="standup comedy and cocktails",
            duration_minutes=90,
            category=EventCategory.SOCIAL.value,
            mood=Mood.HAPPY.value,
            energy_level=6,
            productivity_score=0
        ))

        # Late night reflection
        current_time = day_start + timedelta(hours=23, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="winding down and texting",
            when=current_time.isoformat(),
            where=self.locations.dallas["accommodation"],
            why="processing great day",
            how="texting Alexandra, updating calendar",
            duration_minutes=30,
            category=EventCategory.PERSONAL.value,
            mood=Mood.HAPPY.value,
            energy_level=3,
            productivity_score=0,
            notes="Second date planned for next week"
        ))

        return events

    def generate_friday_dallas(self) -> List[W5HEvent]:
        """Generate Friday events in Dallas - work and community."""
        events = []
        day_start = self.start_date + timedelta(days=5)

        # Early gym
        current_time = day_start + timedelta(hours=6)
        events.append(W5HEvent(
            who="Adam",
            what="early morning gym session",
            when=current_time.isoformat(),
            where="24 Hour Fitness Uptown",
            why="maintaining routine and energy",
            how="chest and back workout",
            duration_minutes=60,
            category=EventCategory.HEALTH.value,
            mood=Mood.ENERGIZED.value,
            energy_level=8,
            productivity_score=0
        ))

        # Team standup
        current_time = day_start + timedelta(hours=8)
        events.append(W5HEvent(
            who=f"Adam with entire team ({', '.join(self.personas.team)})",
            what="Friday team standup and sprint planning",
            when=current_time.isoformat(),
            where=self.locations.dallas["accommodation"],
            why="weekly team sync and planning",
            how="Zoom call with screen sharing",
            duration_minutes=60,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=7,
            productivity_score=8
        ))

        # Coffee shop coding marathon
        current_time = day_start + timedelta(hours=9, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="deep focus coding session",
            when=current_time.isoformat(),
            where=self.locations.dallas["coffee_shops"][3],
            why="shipping new features before weekend",
            how="noise-canceling headphones, lo-fi music, 5 coffees",
            duration_minutes=240,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=8,
            productivity_score=10,
            notes="Shipped iOS sync improvements and API optimizations"
        ))

        # Quick lunch
        current_time = day_start + timedelta(hours=13, minutes=45)
        events.append(W5HEvent(
            who="Adam",
            what="quick lunch break",
            when=current_time.isoformat(),
            where="Sweetgreen nearby",
            why="fuel for afternoon",
            how="salad to-go, eaten while walking",
            duration_minutes=30,
            category=EventCategory.MEAL.value,
            mood=Mood.FOCUSED.value,
            energy_level=5,
            productivity_score=0
        ))

        # VC follow-up calls
        current_time = day_start + timedelta(hours=14, minutes=30)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.investors[0]} and partners",
            what="follow-up call with Bessemer Ventures",
            when=current_time.isoformat(),
            where="Phone call from quiet area",
            why="progressing due diligence",
            how="answering detailed questions about metrics",
            duration_minutes=75,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=6,
            productivity_score=8,
            notes="Moving to partner meeting next week"
        ))

        # More coding
        current_time = day_start + timedelta(hours=16)
        events.append(W5HEvent(
            who="Adam",
            what="fixing bugs reported by team",
            when=current_time.isoformat(),
            where=self.locations.dallas["coffee_shops"][3],
            why="ensuring stable build for demos",
            how="debugging and testing",
            duration_minutes=90,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=5,
            productivity_score=7
        ))

        # Tech community happy hour
        current_time = day_start + timedelta(hours=18)
        events.append(W5HEvent(
            who="Adam with Dallas tech community (20+ people)",
            what="Dallas Startup Happy Hour",
            when=current_time.isoformat(),
            where="Truck Yard Lower Greenville",
            why="networking and community building",
            how="beers and startup stories",
            duration_minutes=150,
            category=EventCategory.SOCIAL.value,
            mood=Mood.HAPPY.value,
            energy_level=6,
            productivity_score=4,
            notes="Met two potential angel investors"
        ))

        # Late dinner with friends
        current_time = day_start + timedelta(hours=21)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.friends[4]} and friends",
            what="late dinner and drinks",
            when=current_time.isoformat(),
            where=self.locations.dallas["restaurants"][3],
            why="unwinding after long week",
            how="sharing appetizers and craft cocktails",
            duration_minutes=120,
            category=EventCategory.SOCIAL.value,
            mood=Mood.RELAXED.value,
            energy_level=4,
            productivity_score=0
        ))

        return events

    def generate_saturday_dallas(self) -> List[W5HEvent]:
        """Generate Saturday events in Dallas - balance and reflection."""
        events = []
        day_start = self.start_date + timedelta(days=6)

        # Sleep in
        current_time = day_start
        events.append(W5HEvent(
            who="Adam",
            what="sleeping in",
            when=current_time.isoformat(),
            where=self.locations.dallas["accommodation"],
            why="recovering from busy week",
            how="no alarm, natural wake up",
            duration_minutes=510,
            category=EventCategory.PERSONAL.value,
            mood=Mood.RELAXED.value,
            energy_level=8,
            productivity_score=0
        ))

        # Farmers market
        current_time = day_start + timedelta(hours=8, minutes=30)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.friends[4]}",
            what="farmers market breakfast and shopping",
            when=current_time.isoformat(),
            where=self.locations.dallas["venues"][1],
            why="weekend ritual and fresh food",
            how="walking, tasting, buying local produce",
            duration_minutes=90,
            category=EventCategory.SOCIAL.value,
            mood=Mood.HAPPY.value,
            energy_level=7,
            productivity_score=0,
            notes="Amazing breakfast tacos and fresh coffee"
        ))

        # Pickleball tournament
        current_time = day_start + timedelta(hours=10, minutes=30)
        events.append(W5HEvent(
            who="Adam with local pickleball community",
            what="Saturday pickleball tournament",
            when=current_time.isoformat(),
            where=self.locations.dallas["recreation"][0],
            why="competition and exercise",
            how="round-robin tournament format",
            duration_minutes=180,
            category=EventCategory.HEALTH.value,
            mood=Mood.ENERGIZED.value,
            energy_level=9,
            productivity_score=0,
            notes="Made it to semi-finals!"
        ))

        # Lunch with mentor
        current_time = day_start + timedelta(hours=14)
        events.append(W5HEvent(
            who="Adam with David Chen (former CEO, mentor)",
            what="mentorship lunch",
            when=current_time.isoformat(),
            where=self.locations.dallas["restaurants"][0],
            why="seeking advice on scaling and fundraising",
            how="sushi lunch and wisdom sharing",
            duration_minutes=120,
            category=EventCategory.WORK.value,
            mood=Mood.CONTEMPLATIVE.value,
            energy_level=6,
            productivity_score=7,
            notes="Key insight: focus on unit economics before scaling"
        ))

        # Reading and research
        current_time = day_start + timedelta(hours=16, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="reading and research at coffee shop",
            when=current_time.isoformat(),
            where=self.locations.dallas["coffee_shops"][0],
            why="staying informed on AI developments",
            how="reading papers and Hacker News",
            duration_minutes=90,
            category=EventCategory.WORK.value,
            mood=Mood.CONTEMPLATIVE.value,
            energy_level=5,
            productivity_score=5,
            notes="New paper on RLHF very relevant to Ariata"
        ))

        # Shower and prep
        current_time = day_start + timedelta(hours=18, minutes=30)
        events.append(W5HEvent(
            who="Adam",
            what="getting ready for dinner party",
            when=current_time.isoformat(),
            where=self.locations.dallas["accommodation"],
            why="preparing for social evening",
            how="shower and casual dress",
            duration_minutes=45,
            category=EventCategory.PERSONAL.value,
            mood=Mood.RELAXED.value,
            energy_level=5,
            productivity_score=0
        ))

        # Dinner party
        current_time = day_start + timedelta(hours=19, minutes=30)
        events.append(W5HEvent(
            who=f"Adam with {self.personas.friends[4]} and 8 friends",
            what="dinner party at friend's place",
            when=current_time.isoformat(),
            where="Marcus's apartment rooftop",
            why="socializing and networking",
            how="potluck style, brought wine",
            duration_minutes=180,
            category=EventCategory.SOCIAL.value,
            mood=Mood.HAPPY.value,
            energy_level=6,
            productivity_score=0,
            notes="Great conversations about AI ethics"
        ))

        # Late night coding
        current_time = day_start + timedelta(hours=23)
        events.append(W5HEvent(
            who="Adam",
            what="late night coding session",
            when=current_time.isoformat(),
            where=self.locations.dallas["accommodation"],
            why="inspiration struck for new feature",
            how="implementing idea while fresh",
            duration_minutes=90,
            category=EventCategory.WORK.value,
            mood=Mood.FOCUSED.value,
            energy_level=5,
            productivity_score=8,
            notes="Prototyped new context window optimization"
        ))

        return events


def generate_test_data():
    """Main function to generate W5H test data."""
    print("Generating W5H test data for Ariata demo...")
    print("-" * 50)

    generator = EventGenerator()
    events = generator.generate_week()

    print(f"Generated {len(events)} events over 7 days")
    print(f"Average events per day: {len(events) / 7:.1f}")

    # Convert to JSON-serializable format
    events_data = [event.to_dict() for event in events]

    # Save to file
    output_file = "w5h_test_data.json"
    with open(output_file, 'w') as f:
        json.dump(events_data, f, indent=2)

    print(f"\nData saved to {output_file}")

    # Print summary statistics
    print("\nEvent Category Distribution:")
    categories = {}
    for event in events:
        cat = event.category
        categories[cat] = categories.get(cat, 0) + 1

    for cat, count in sorted(categories.items(), key=lambda x: x[1], reverse=True):
        print(f"  {cat:15s}: {count:3d} events")

    print("\nSample events:")
    for i in range(min(3, len(events))):
        event = events[i]
        print(f"\n  Event {i+1}:")
        print(f"    Who: {event.who}")
        print(f"    What: {event.what}")
        print(f"    Where: {event.where}")
        print(f"    When: {event.when}")

    return events_data


if __name__ == "__main__":
    data = generate_test_data()