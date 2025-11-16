export interface ModelOption {
	id: string;
	displayName: string;
	provider: string;
}

export const models: ModelOption[] = [
	{
		id: 'openai/gpt-oss-120b',
		displayName: 'GPT OSS 120B',
		provider: 'OpenAI'
	},
	{
		id: 'openai/gpt-oss-20b',
		displayName: 'GPT OSS 20B',
		provider: 'OpenAI'
	},
	{
		id: 'anthropic/claude-sonnet-4.5',
		displayName: 'Claude Sonnet 4.5',
		provider: 'Anthropic'
	},
	{
		id: 'anthropic/claude-opus-4.1',
		displayName: 'Claude Opus 4.1',
		provider: 'Anthropic'
	},
	{
		id: 'anthropic/claude-haiku-4.5',
		displayName: 'Claude Haiku 4.5',
		provider: 'Anthropic'
	},
	{
		id: 'openai/gpt-5',
		displayName: 'GPT-5',
		provider: 'OpenAI'
	},
	{
		id: 'google/gemini-2.5-pro',
		displayName: 'Gemini 2.5 Pro',
		provider: 'Google'
	},
	{
		id: 'google/gemini-2.5-flash',
		displayName: 'Gemini 2.5 Flash',
		provider: 'Google'
	},
	{
		id: 'xai/grok-4',
		displayName: 'Grok 4',
		provider: 'xAI'
	},
	{
		id: 'moonshotai/kimi-k2-thinking',
		displayName: 'Kimi K2 Thinking',
		provider: 'Moonshot AI'
	}
];

export const DEFAULT_MODEL = models[0]; // GPT OSS 120B
