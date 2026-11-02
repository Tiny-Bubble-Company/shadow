// Shadow pricing: the only place prices live. Components, JSON-LD and llms.txt read it.
// PLACEHOLDER: every Hub price, seat band and inclusion below is a hypothesis until
// confirmed; Core is free and open source (Apache-2.0) for good.

export type Billing = 'annual' | 'monthly';
export type CtaAction = 'install' | 'hub' | 'demo';

export interface Tier {
	id: 'core' | 'hub-free' | 'hub-starter' | 'hub-business' | 'hub-enterprise';
	name: string;
	/** Company size it is meant for. */
	size: string;
	/** Seat range for the cost estimator (`max` null = no upper bound). */
	seats: { min: number; max: number | null };
	/** USD per user per month; null = free or custom. */
	price: { annual: number | null; monthly: number | null };
	/** Shown instead of a number: "Free", "Custom". */
	priceLabel?: string;
	/** Small print under the price. */
	priceNote: string;
	includes: string[];
	cta: { label: string; action: CtaAction };
	featured?: boolean;
	/** Anchor IDs this tier also answers to (kept for old links). */
	aliases?: string[];
}

export const TIERS: Tier[] = [
	{
		id: 'core',
		name: 'Core',
		size: 'Anyone',
		seats: { min: 1, max: null },
		price: { annual: 0, monthly: 0 },
		priceLabel: '$0',
		priceNote: 'Apache-2.0, forever',
		includes: ['Local proxy on every laptop or server', 'Masking and blocking', 'Activity dashboard, menu-bar app and CLI stats', 'No account, no telemetry'],
		cta: { label: 'Install Core free', action: 'install' },
	},
	{
		id: 'hub-free',
		name: 'Hub Free',
		size: '1–5 users',
		seats: { min: 1, max: 5 },
		price: { annual: 0, monthly: 0 },
		priceLabel: '$0',
		priceNote: 'up to 5 users',
		includes: ['Hosted dashboard', '7-day history', 'Email invites'],
		cta: { label: 'Start free', action: 'hub' },
	},
	{
		id: 'hub-starter',
		name: 'Hub Starter',
		size: '6–50 users',
		seats: { min: 6, max: 50 },
		price: { annual: 8, monthly: 10 },
		priceNote: 'per user / month',
		includes: ['Everything in Hub Free', '30-day history', 'Policy alerts', 'CSV export'],
		cta: { label: 'Start free Hub trial', action: 'hub' },
		featured: true,
		aliases: ['team'],
	},
	{
		id: 'hub-business',
		name: 'Hub Business',
		size: '51–500 users',
		seats: { min: 51, max: 500 },
		price: { annual: 15, monthly: null },
		priceNote: 'per user / month',
		includes: ['Everything in Hub Starter', '1-year history', 'SSO', 'MDM rollout', 'Policy packs', 'Slack and email alerts'],
		cta: { label: 'Start free Hub trial', action: 'hub' },
		aliases: ['business'],
	},
	{
		id: 'hub-enterprise',
		name: 'Hub Enterprise',
		size: '500+ users',
		seats: { min: 501, max: null },
		price: { annual: 20, monthly: null },
		priceLabel: 'Custom',
		priceNote: 'from $20 per user / month',
		includes: ['Everything in Hub Business', 'SIEM export', 'SCIM', 'Audit evidence', 'EU data residency', 'DPA and SLA'],
		cta: { label: 'Book a demo', action: 'demo' },
		aliases: ['enterprise'],
	},
];

export const PRICING_NOTES = [
	'You pay only for users connected to Hub; admins are free.',
	'14-day Business trial, no card required.',
];

/** Hub tiers only (for the estimator and schema). */
export const HUB_TIERS = TIERS.filter((t) => t.id.startsWith('hub-'));

/** Lowest and highest published Hub price per user per month (for AggregateOffer). */
export const HUB_PRICE_RANGE = {
	low: 0,
	high: Math.max(...HUB_TIERS.flatMap((t) => [t.price.annual ?? 0, t.price.monthly ?? 0])),
};

/** The tier for `seats` users. */
export function tierForSeats(seats: number): Tier {
	return HUB_TIERS.find((t) => seats >= t.seats.min && (t.seats.max === null || seats <= t.seats.max)) ?? HUB_TIERS[HUB_TIERS.length - 1]!;
}

/** "From $8/user/mo" for the cheapest paid Hub tier. */
export const HUB_FROM = `From $${Math.min(...HUB_TIERS.map((t) => t.price.annual ?? Infinity).filter((p) => p > 0))}/user/mo`;
