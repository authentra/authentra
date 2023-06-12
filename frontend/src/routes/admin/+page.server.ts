import { checkAdmin, createMeta } from "$lib/server/utils";
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({url,locals}) => {
    checkAdmin(url, locals)
    return {
        meta: createMeta(locals)
    }
};