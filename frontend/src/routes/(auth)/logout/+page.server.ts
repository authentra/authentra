import type { PageServerLoad } from "./$types";
import * as set_cookie_parser from 'set-cookie-parser';
import { redirect } from "@sveltejs/kit";

export const load: PageServerLoad = async ({ locals, cookies }) => {
    if (!locals.user) {
        throw redirect(303, '/login')
    }
    await locals.api.delete('/auth/browser/logout').then(res => {
        //@ts-expect-error
        for (const str of set_cookie_parser.splitCookiesString(res.headers.get('set-cookie'))) {
            const { name, value, ...options } = set_cookie_parser.parseString(str);
            //@ts-expect-error
            cookies.set(name, value, { ...options });
        }
    })
    throw redirect(303, '/login')
};