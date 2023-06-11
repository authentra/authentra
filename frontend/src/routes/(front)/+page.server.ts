import { createMeta } from "$lib/server/utils";
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({locals}) => {
    return {
        user: locals.user,
        meta: createMeta(locals)
    }
};