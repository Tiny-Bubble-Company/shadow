// Serves the built site (dist/, via the ASSETS binding) and sends www.getshadow.dev to the
// apex domain with a 301, keeping the path and query string.
const APEX = 'getshadow.dev';

export default {
	async fetch(request, env) {
		const url = new URL(request.url);
		if (url.hostname === `www.${APEX}`) {
			url.hostname = APEX;
			url.protocol = 'https:';
			return Response.redirect(url.toString(), 301);
		}
		return env.ASSETS.fetch(request);
	},
};
