import { Api } from '$lib/api';
import { INTERNAL_API_URL } from '$lib/server/utils';
import { API_URL } from '$lib/utils';

if (!INTERNAL_API_URL || !API_URL) {
  throw Error()
}

export async function handleFetch({ event, request, fetch }) {
  if (request.url.startsWith(INTERNAL_API_URL)) {
    const cookie = event.request.headers.get('cookie');
    if (cookie) {
      request.headers.set('cookie', cookie);
    }
  }
  return await fetch(request);
}

export async function handle({ event, resolve }) {
  const api = new Api(INTERNAL_API_URL, event.fetch, event.cookies);
  const cookie = event.cookies.get('session_token');
  if (cookie) {
    const jwt_cookie = event.cookies.get('jwt');
    if (jwt_cookie) {
      api.tokenStore.set(jwt_cookie)
    } else {
      await api.refreshToken();
    }
    event.locals.user = await api.user.me();
  }
  event.locals.api = api;
  const response = await resolve(event, {
    transformPageChunk: ({ html }) => html.replace('%unocss-svelte-scoped.global%', 'unocss_svelte_scoped_global_styles'),
  })
  return response
}