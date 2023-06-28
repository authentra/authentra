import { building } from "$app/environment";
import { env } from "$env/dynamic/private";
import { UserRoles, type UserRole } from "$lib/api/types";
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
export function checkDeveloper(current: URL, locals: App.Locals) {
    checkAuth(current, locals)
    if (!locals.user || !hasRole(locals.user.roles, 'developer')) {
        throw error(403, { message: 'Forbidden' })
    }
}

function hasRole(roles: UserRole[], role: UserRole): boolean {
    if (roles.includes(role)) {
        return true
    } else if (roles.includes('admin')) {
        return true
    } else {
        return false
    }
}

export function getRolesFromForm(formData: FormData): UserRole[] {
    const roles = [];
    for (const [index, value] of UserRoles.entries()) {
        const data = formData.has('role:' + value);
        if (data) {
            roles.push(value);
        }
    }
    return roles
}

export const INTERNAL_API_URL = env.INTERNAL_API_URL as string;
if (!INTERNAL_API_URL && !building) {
    throw Error("INTERNAL_API_URL not set")
}