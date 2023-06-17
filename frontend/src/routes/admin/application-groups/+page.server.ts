import { InternalScopes, type InternalScope } from "$lib/api/developer";
import { createMeta } from "$lib/server/utils";
import type { Actions, PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({url,locals}) => {
    return {
        groups: await locals.apis.application_groups.all(),
        meta: createMeta(locals)
    }
};

export const actions: Actions = {
    replace: async ({locals, request}) => {
        const formData = await request.formData();
        const id = formData.get('id') as string;
        const scopes: InternalScope[] = [];
        for (const [index, value] of InternalScopes.entries()) {
            const data = formData.get(value);
            if (data) {
                scopes.push(value);
            }
        }
        return await locals.apis.application_groups.replace(id, scopes)
    },
    used_by: async ({locals, request}) => {
        const formData = await request.formData();
        const id = formData.get('id') as string;
        return await locals.apis.application_groups.usedBy(id)
    },
    delete: async ({locals, request}) => {
        const formData = await request.formData();
        const id = formData.get('id') as string;
        return await locals.apis.application_groups.delete(id)
    },
    create: async ({locals, request}) => {
        const formData = await request.formData();
        const id = formData.get('id') as string;
        const scopes: InternalScope[] = [];
        for (const [index, value] of InternalScopes.entries()) {
            const data = formData.get(value);
            if (data) {
                scopes.push(value);
            }
        }
        return await locals.apis.application_groups.create(id, scopes)
    }
};