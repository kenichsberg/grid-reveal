// Single-page app: there is no server to render on, and auth lives in a cookie
// pointed at another origin.
export const ssr = false;
export const prerender = false;
