import { env } from "$env/dynamic/private";
import { redirectUrl, type Meta } from "$lib/utils";
import { error } from "@sveltejs/kit";
import { get } from "svelte/store";

export function createMeta(locals: App.Locals): Meta {
    return {
        api_token: get(locals.api.tokenStore)
    }
}

export function checkAuth(current: URL, locals: App.Locals) {
    if (!locals.user) {
        redirectUrl(current, '/login', 302)
    }
}

export function checkAdmin(current: URL, locals: App.Locals) {
    checkAuth(current, locals)
    if (!locals.user || !locals.user.roles.includes('admin')) {
        throw error(403, { message: 'Forbidden' })
    }
}

export const INTERNAL_API_URL = env.INTERNAL_API_URL as string;
if (!INTERNAL_API_URL) {
    throw Error("INTERNAL_API_URL not set")
}