import { env } from "$env/dynamic/public";
import { getContext, setContext } from "svelte";
import { Api } from "./api";
import { redirect } from "@sveltejs/kit";
import { building } from "$app/environment";

export interface Meta {
    api_token: string | null
}

export interface ClientMeta {
    api: Api
}

export function extractRedirect(params: URLSearchParams): string {
    const redirect = params.get('redirect')
    if (!redirect) {
        return '/'
    }
    return `/${redirect.slice(1)}`
}

export function jsonBody(body: any): RequestInit {
    return {
        body: JSON.stringify(body),
        headers: {
            "Content-Type": "application/json"
        }
    }
}

export function redirectUrl(current: URL, target: string, code: 300 | 301 | 302 | 303 | 304 | 305 | 306 | 307 | 308, paramName: string = 'redirect'): URL {
    const params = new URLSearchParams();
    params.set(paramName, `${current.pathname}${current.search}`)
    throw redirect(code, `${target}?${params.toString()}`);
}

export function handleMeta(meta: Meta): ClientMeta {
    const existingMeta: ClientMeta | null = getContext('meta');
    if (existingMeta) {
        existingMeta.api.tokenStore.set(meta.api_token)
        return existingMeta
    }
    const api = new Api(API_URL, fetch, null);
    const newMeta = {
        api
    }
    setContext('meta', newMeta)
    return newMeta
}

export const API_URL: string = env.PUBLIC_API_URL as string
if (!env.PUBLIC_API_URL && !building) {
    throw Error("PUBLIC_API_URL not set")
}

export function formatName(first_name: string | null, last_name: string | null): string | null {
    if (!first_name && !last_name) {
        return null
    }
    if (first_name && last_name) {
        return first_name + " " + last_name;
    }

    if (first_name) {
        return first_name
    }
    return null
}