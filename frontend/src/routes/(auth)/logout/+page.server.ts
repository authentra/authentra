import type { PageServerLoad } from "./$types";
import { redirect } from "@sveltejs/kit";

export const load: PageServerLoad = async ({ locals, cookies }) => {
    if (!locals.user) {
        throw redirect(303, '/login')
    }
    await locals.api.delete('/auth/browser/logout')
    cookies.delete('session-token')
    cookies.delete('jwt')
    throw redirect(303, '/login')
};