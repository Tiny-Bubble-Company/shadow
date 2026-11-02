// Shadow Hub launch state and links: one place to flip when signup opens.
// Every CTA ("Start free Hub trial" vs "Join the Hub waitlist", "Book a demo") reads this.

export const hub = {
	/** Hub signup is open. While false, buyer CTAs open the waitlist form instead. */
	hubLive: false,
	/** Hub signup page, used once `hubLive` is true. PLACEHOLDER: final Hub domain. */
	signupUrl: 'https://hub.getshadow.dev/signup',
	/** A booking link (for example Cal.com). PLACEHOLDER: empty until one exists; the demo form is used instead. */
	demoBookingUrl: '',
	/** The trial offered once Hub is live. PLACEHOLDER. */
	trial: '14-day free trial, no card required, then free for up to 3 users',
};

export const BUYER_CTA = hub.hubLive ? 'Start free Hub trial' : 'Join the Hub waitlist';
export const DEMO_CTA = 'Book a demo';
export const CORE_CTA = 'Install Core free';

/** The one-line Core install command for macOS and Linux. */
export const INSTALL_CMD =
	'curl -fsSL https://hub.getshadow.dev/core/install.sh | sh';
