// Research topic taxonomy for scalable long-tail content architecture.
// Each topic has a slug, label, description, and related keywords for SEO.
// This enables hundreds/thousands of specific AI-tool privacy questions to be organized.

export interface ResearchTopic {
	slug: string;
	label: string;
	description: string;
	keywords: string[];
	parent?: string;
}

// Primary topic categories (broad)
export const PRIMARY_TOPICS: ResearchTopic[] = [
	{
		slug: 'ai-privacy',
		label: 'AI Privacy',
		description: 'Research on data privacy risks when using AI tools, what happens to your prompts, and how to protect sensitive information.',
		keywords: ['ai privacy', 'llm privacy', 'ai data protection', 'prompt privacy'],
	},
	{
		slug: 'ai-coding-tools',
		label: 'AI Coding Tools',
		description: 'Privacy and security analysis of AI coding assistants like Cursor, GitHub Copilot, Claude Code, and other developer AI tools.',
		keywords: ['ai coding assistant privacy', 'cursor privacy', 'copilot security', 'claude code data'],
	},
	{
		slug: 'enterprise-ai',
		label: 'Enterprise AI',
		description: 'How businesses can use AI safely: enterprise plans, data agreements, compliance considerations, and governance frameworks.',
		keywords: ['enterprise ai security', 'chatgpt enterprise', 'ai governance', 'ai compliance'],
	},
	{
		slug: 'shadow-ai',
		label: 'Shadow AI',
		description: 'Understanding and detecting unauthorized AI use in organizations, and how to govern AI without blocking productivity.',
		keywords: ['shadow ai', 'unauthorized ai use', 'ai governance', 'detect ai usage'],
	},
	{
		slug: 'ai-security',
		label: 'AI Security',
		description: 'Security risks of AI tools including data leaks, credential exposure, and how to protect against them.',
		keywords: ['ai security risks', 'llm security', 'ai data leak', 'prompt injection'],
	},
	{
		slug: 'data-privacy',
		label: 'Data Privacy',
		description: 'How personal data, customer information, and sensitive business data can be exposed through AI tools.',
		keywords: ['pii in ai', 'data leak ai', 'gdpr ai', 'personal data llm'],
	},
	{
		slug: 'ai-governance',
		label: 'AI Governance',
		description: 'Policies, frameworks, and technical controls for governing AI use in organizations.',
		keywords: ['ai policy', 'ai acceptable use', 'ai governance framework', 'ai risk management'],
	},
	{
		slug: 'developer-tools',
		label: 'Developer Tools',
		description: 'AI tools for software development and how to use them without exposing code, secrets, or customer data.',
		keywords: ['developer ai privacy', 'coding assistant security', 'api key leak', 'secrets in prompts'],
	},
	{
		slug: 'privacy-controls',
		label: 'Privacy Controls',
		description: 'Technical and administrative controls for protecting data when using AI: proxies, masking, enterprise settings.',
		keywords: ['ai privacy controls', 'llm proxy', 'data masking', 'privacy mode'],
	},
	{
		slug: 'research',
		label: 'Research',
		description: 'Original research and data on AI privacy, including leak reports and aggregate statistics.',
		keywords: ['ai privacy research', 'llm data research', 'ai leak report'],
	},
];

// Specific AI tools for long-tail targeting
export const AI_TOOLS = [
	'ChatGPT',
	'Claude',
	'Claude Code',
	'Cursor',
	'GitHub Copilot',
	'Gemini',
	'Codex',
	'GPT-4',
	'Claude Opus',
	'Claude Sonnet',
	'Perplexity',
	'Notion AI',
	'Grammarly',
	'Jasper',
	'Copy.ai',
	'Midjourney',
	'DALL-E',
	'Stable Diffusion',
	'Ollama',
	'vLLM',
	'LiteLLM',
	'OpenRouter',
];

// AI providers
export const AI_PROVIDERS = [
	'OpenAI',
	'Anthropic',
	'Google',
	'Microsoft',
	'Amazon Bedrock',
	'Azure OpenAI',
	'Mistral',
	'Groq',
	'Together AI',
	'Replicate',
	'Hugging Face',
];

// Target audiences
export const AUDIENCES = [
	'Developers',
	'CISOs',
	'IT Admins',
	'Security Teams',
	'Compliance Officers',
	'Engineering Managers',
	'CTOs',
	'Privacy Officers',
	'Legal Teams',
	'Enterprise Buyers',
];

// Get all topics as a flat list
export function getAllTopics(): ResearchTopic[] {
	return PRIMARY_TOPICS;
}

// Get topic by slug
export function getTopicBySlug(slug: string): ResearchTopic | undefined {
	return PRIMARY_TOPICS.find((t) => t.slug === slug);
}

// Map article topics to topic objects
export function mapTopicsToObjects(topicLabels: string[]): ResearchTopic[] {
	return topicLabels
		.map((label) => PRIMARY_TOPICS.find((t) => t.label === label))
		.filter((t): t is ResearchTopic => t !== undefined);
}
