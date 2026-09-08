/**
 * "In your own words" — the interview.
 *
 * FIVE AT SIGNUP, NINE HELD BACK. The corpus below is all fourteen; only the
 * five in `ONBOARDING_QUESTIONS` are asked at signup, chosen by one rule: they
 * are the questions NO RECORD CAN EVER GROUND. Everything about a life that can
 * be observed — who you talk to, where you go, what you are working on — is
 * asked better later, per-thing, with the evidence attached. An hour of typing
 * before the box has shown anything was spending the person's patience on
 * exactly the half that did not need it.
 *
 * The one document on the box nobody can observe. Everything else is derived
 * from the record; this is authored, and it is what lets the box know what
 * someone is FOR rather than only what they did.
 *
 * PROSE, NOT FIELDS. An earlier draft decomposed a life into a schema — entries
 * with kinds, facets, salience. Two things killed it: the consumer is a
 * language model, which reads paragraphs better than JSON containing
 * paragraphs; and empty fields nag, so a half-filled identity schema reads as
 * personal failure. The structure lives in the ASKING.
 *
 * SCENES, NOT ONLY SUMMARIES. The first version asked twelve questions that all
 * wanted a generalization — what are you like, what are you proud of — and
 * self-summary is the least reliable thing a person produces. McAdams' life
 * story interview, which is where the term narrative identity comes from, is
 * built on KEY SCENES instead: a high point, a low point. One episode with
 * details in it beats three paragraphs of self-assessment, because a story with
 * a place and a person in it is hard to flatter yourself inside.
 *
 * ORDERED BY RISING EXPOSURE. Nothing hard until the third question, and the
 * third only after the second has established that this is safe to answer
 * honestly. Loss follows the people question so it arrives as the natural next
 * thing rather than an ambush. The rules question is LAST, where "what of this
 * should it never raise?" reads as protection instead of interrogation.
 *
 * ONE THING PER QUESTION. Double-barrelled questions get one answer and lose
 * the other half silently. Where a second prompt helps, it goes in the hint.
 *
 * NO WORD THE PERSON WOULD NOT USE. No telos, no narrative identity, no
 * standing orders — that last one was the tell that this had drifted into
 * machine language for the most personal writing in the product.
 */

export interface Question {
	/** Stable across rewordings and reorderings — answers are filed by this. */
	id: string;
	/** The interrogative it covers, shown as a quiet marker. */
	facet: string;
	prompt: string;
	/** One line under the prompt — what it is for. */
	purpose: string;
	/**
	 * The longer answer to "why are you asking me this", behind a disclosure.
	 *
	 * These questions ask for grief, vice and faith, and whether someone answers
	 * properly or skips usually turns on believing the asking is principled.
	 * Explaining the MECHANISM earns that where reassurance does not.
	 */
	why: string;
	/** Optional nudge under the field. Never a rule. */
	hint?: string;
	/** Soft word target. `null` means this question is never counted. */
	target: number | null;
	/**
	 * How this question WANTS to be answered.
	 *
	 * Not yet honored — everything renders as a text field today. Recorded
	 * because the intent is real and would otherwise be lost: stories want
	 * speaking, lists want typing. People say more, and more honestly, aloud;
	 * writing invites editing, and editing is where self-presentation creeps in.
	 * The box already transcribes locally, so telling your life to a machine in
	 * your house that sends it nowhere demonstrates the whole thesis better than
	 * a textarea can.
	 */
	mode: 'type' | 'speak';
	/** Which movement of the document it feeds. */
	tense: 'past' | 'present' | 'future' | 'rules';
	/**
	 * Asked at signup, or held for the queue.
	 *
	 * `onboarding` is reserved for questions NO RECORD CAN EVER GROUND. That is
	 * the whole selection rule, and it is why "who were the people in your life"
	 * is not among them despite being one of the most valuable answers here: six
	 * weeks from now the box can ask it per-person, with four thousand messages
	 * of evidence attached, and get a better answer for a fraction of the effort.
	 * Asking it cold on day zero spends the person's patience on the one thing
	 * patience was not needed for.
	 *
	 * See agents/plan/narrative-resolution-plan.md.
	 */
	stage: 'onboarding' | 'queue';
}

export const QUESTIONS: Question[] = [
	{
		id: 'chapters',
		facet: 'WHEN',
		// Rendered as ChapterCards, not a textarea — the one structured question.
		// The old triple-barrelled prompt (places + chapters + changepoints)
		// produced tangled paragraphs; the cards carry places/years/changepoint
		// as fields, so the prompt asks one thing.
		prompt: 'What were the chapters of your life?',
		purpose:
			'Name the eras the way you would tell them to a friend — childhood, the moves, the jobs, the years that belong together. Everything else you write gets placed on this timeline.',
		why: "Everyone privately divides their life into chapters, and nobody's match the calendar. A box that knows yours can say “that was during the Boston years” instead of “in 2017” — the difference between a filing system and a memory. What ENDED each chapter matters most: the moment a period closed is usually the moment something in you did. Places ride along because moves are one of the few universally legible turning points, and where you were usually explains who you knew.",
		hint:
			'Name them the way you would out loud. Rough years are fine — what ended each one matters most. Most lives so far have five to ten.',
		// Cards, not prose: a word target against short structured fields would
		// read as perpetual shortfall. The chapter count is its own progress.
		target: null,
		mode: 'speak',
		tense: 'past',
		stage: 'onboarding'
	},
	{
		id: 'high_point',
		facet: 'SCENE',
		prompt: 'Tell me about one of the best moments of your life.',
		purpose: 'One scene, in detail — where you were, who was there, what it was like.',
		why: "A single episode carries more than any amount of self-description. Ask someone what they are like and you get a summary they have given before; ask for a specific afternoon and you get the place, the people and what they actually cared about — none of which is easy to flatter yourself inside. This is the instrument the narrative identity literature is built on. The high point comes first because it is the easy one, and it is what makes the next question answerable.",
		hint: 'Set the scene. Where, who, what it felt like.',
		target: 300,
		mode: 'speak',
		tense: 'past',
		stage: 'queue'
	},
	{
		id: 'low_point',
		facet: 'SCENE',
		prompt: 'And one of the worst.',
		purpose: 'The other end of the same instrument.',
		why: "The low point is the more revealing of the two, and not because of what happened. It is HOW it gets told: whether a hard thing is narrated as something that led somewhere, or as something that only took something away. That shape — the same events told either way — says more about a person than the events do. Answer it however it comes; there is nothing to get right here.",
		hint: 'As much as you want to say. This one is allowed to be short.',
		target: null,
		mode: 'speak',
		tense: 'past',
		stage: 'queue'
	},
	{
		id: 'people',
		facet: 'WHO',
		prompt: 'In each of those chapters, who were the people?',
		purpose:
			'The relational spine. These become the people your box knows by name, because you named them.',
		why: "The box will see thousands of names and know nothing about which mattered. Frequency is not significance: most people message a landlord more often than a brother. This is the only door significance can come through, and it has to be you who opens it — a machine ranking your loved ones by message volume would be both wrong and insulting.",
		hint: 'Who they were to you, and what they were like.',
		target: 400,
		mode: 'type',
		tense: 'past',
		stage: 'queue'
	},
	{
		id: 'loss',
		facet: 'WHO',
		prompt: 'Who, or what, have you lost?',
		purpose: 'Written here so the box understands — and so it knows what to step around.',
		why: "Grief is invisible to data. Someone who died leaves a thinning trace and then none, which is precisely the shape a record reads as “stopped mattering”. No amount of observation will ever recover it, and almost nothing shapes a person more. Written here, it becomes something the box understands — and something it knows to step around rather than walk into.",
		hint: 'As much or as little as you want. There is no length worth aiming for.',
		target: null,
		mode: 'type',
		tense: 'past',
		stage: 'queue'
	},
	{
		id: 'admire',
		facet: 'WHO',
		prompt: 'Which well-known figures do you admire, and what specifically about them?',
		purpose:
			'Who you point to says what you value — more precisely than any list of traits could.',
		why: "Values named as adjectives are mush — everyone wants to be honest and kind. Values named as PEOPLE are precise. But precise for WHOM: “my grandmother's stubbornness” means everything to you and nothing to a machine that never met her, while a figure the world knows arrives already carrying a body of work, a temperament, and a way of speaking your box can actually draw on. So name the public ones first; add the private ones after, and say what they were like. The second half does a different job entirely. Nobody can usefully answer “do you want brief or thorough, challenging or supportive”, but everyone can point at someone whose way of telling them a hard thing actually landed. That is how this learns to talk to you, without a single slider.",
		hint:
			'Writers, thinkers, founders, saints, athletes — whoever. Private people count too; just name the public ones first. And if someone\'s way of putting things is how you would want this to talk to you, say who.',
		target: 300,
		mode: 'speak',
		tense: 'present',
		stage: 'onboarding'
	},
	{
		id: 'pride',
		facet: 'HOW',
		prompt: 'What are you proud of in how you go about things?',
		purpose: 'Virtues, asked in the only way anyone can actually answer.',
		why: "Nobody can answer “what are your virtues” — it comes out as false modesty or a résumé. Everyone can say what they are quietly proud of, and what surfaces is usually a HOW rather than a WHAT: how you treat people, how you handle being wrong, what you do when it stops being fun. The gap between what you are proud of and what you get praised for is where a lot of a person lives.",
		hint: 'How you do things, not only what you have done.',
		target: 300,
		mode: 'speak',
		tense: 'present',
		stage: 'queue'
	},
	{
		id: 'novelty',
		facet: 'DELTA',
		prompt: 'What makes you different from most people you have met?',
		purpose: 'So it never mistakes you for the average person.',
		why: "A language model's default assumption about you is the average human. Everything it has not been told, it fills in from the population mean. So this question asks, quite literally, for the difference between you and that prior — which makes it the highest-information thing you can write here. It is also the one people skip because it feels like bragging. It is not bragging; it is calibration.",
		hint: 'The things people find surprising, or that you have stopped mentioning because nobody relates.',
		target: 300,
		mode: 'type',
		tense: 'present',
		stage: 'onboarding'
	},
	{
		id: 'vices',
		facet: 'PULL',
		prompt: 'Which pull is strongest for you — money, power, pleasure, or fame? And why that one?',
		purpose: 'The thing you are up against, named by you rather than guessed at.',
		why: "The four are Arthur Brooks'. They work because they are a menu rather than a blank page, and most people know their answer in about a second — which is usually a sign that it is the true one. Written by you it is self-knowledge; derived by a machine from your own data it would be an accusation, which is exactly why it is asked and never inferred.",
		hint: 'And if it helps: what do you keep starting and not finishing?',
		target: 200,
		mode: 'type',
		tense: 'present',
		stage: 'onboarding'
	},
	{
		id: 'belief',
		facet: 'WHY',
		prompt: 'What is your religion, or your worldview?',
		purpose: 'The why underneath the rest. Recorded to be understood, never to be argued with.',
		why: "An assistant told nothing about what you believe falls back on a bland average: agreeable, agnostic, faintly therapeutic. That voice grates on nearly everyone eventually, from every direction. This is not here to be argued with, optimized, or gently corrected. It is here so that what the box says stops cutting across the grain of what you actually hold.",
		hint:
			'Including if you are still working it out — “I genuinely do not know” is an answer, and a useful one.',
		target: 300,
		mode: 'speak',
		tense: 'present',
		stage: 'onboarding'
	},
	{
		id: 'now',
		facet: 'NOW',
		prompt: 'What are you working on right now? Five or ten things.',
		purpose: 'What is live — which the record cannot tell you on its own.',
		why: "The box will work out plenty about your days: where you went, who you saw, what you opened. None of it says which of it you CHOSE. A week of activity looks identical whether you are building something or avoiding something, and this is the only way to tell the two apart.",
		hint: 'Work, body, house, people, projects, the thing you keep meaning to start.',
		target: 200,
		mode: 'type',
		tense: 'present',
		stage: 'queue'
	},
	{
		id: 'ambitions',
		facet: 'FUTURE',
		prompt: 'Three years from now, and ten — what do you want?',
		purpose:
			'Direction. What lets the box be useful about decisions, not only accurate about the past.',
		why: "Without direction, an assistant can only ever be accurate about your past. Two horizons because they do different jobs: ten years is identity, three years is strategy. Most people can answer the ten and stall on the three, and noticing that about yourself is worth as much as the answer.",
		hint: 'What is actually true, rather than what sounds good.',
		target: 400,
		mode: 'speak',
		tense: 'future',
		stage: 'queue'
	},
	{
		id: 'shadow_future',
		facet: 'FUTURE',
		prompt: 'And if your worst habits win — what does five years from now look like?',
		purpose: 'The other future. Usually the more specific one.',
		why: "People describe the life they want in generalities and the life they fear in detail — which is why the feared one moves them and the wanted one mostly does not. Writing it down is uncomfortable, and that is the point: this is the version the box should help you notice you are drifting toward, long before it arrives.",
		hint: 'Be concrete. Vague is comfortable and useless here.',
		target: 300,
		mode: 'speak',
		tense: 'future',
		stage: 'queue'
	},
	{
		id: 'rules',
		facet: 'RULES',
		prompt: 'Looking back at everything you just wrote — what should it never bring up?',
		purpose: 'The part that becomes rules rather than understanding.',
		why: "Everything above is for understanding. This is for enforcement. Some things matter enormously and must still never be raised unprompted — a person you lost, an addiction in recovery, a marriage that ended. Prose cannot guarantee that: a model reading a paragraph might honor it nine times and miss the tenth, and the tenth is the one that would matter. What you write here stops being context and becomes a rule.",
		hint: 'Plainly: “never suggest bars”, “do not mention my father unless I do”.',
		target: null,
		mode: 'type',
		tense: 'rules',
		stage: 'queue'
	}
];

/** Words, counted the way a person would count them. */
export function wordCount(s: string): number {
	const t = s.trim();
	return t ? t.split(/\s+/).length : 0;
}

/**
 * The five asked at signup, in asking order.
 *
 * Not their order in `QUESTIONS` above, which is the corpus order. Rising
 * exposure still governs: the scaffold first, then the calibration, then the
 * two that ask what someone is up against, then the one underneath all of it.
 */
const ONBOARDING_ORDER = ['chapters', 'novelty', 'admire', 'vices', 'belief'] as const;

export const ONBOARDING_QUESTIONS: Question[] = ONBOARDING_ORDER.map(
	(id) => QUESTIONS.find((q) => q.id === id) as Question
);

/**
 * The nine held back — the seed of the resolution queue.
 *
 * Kept here rather than deleted: they are the best writing in the product, and
 * every one of them is still going to be asked. Just later, by a box that has
 * earned the right to ask, and in several cases grounded in something real
 * rather than posed cold. See agents/plan/narrative-resolution-plan.md.
 */
export const QUEUED_QUESTIONS: Question[] = QUESTIONS.filter((q) => q.stage === 'queue');
