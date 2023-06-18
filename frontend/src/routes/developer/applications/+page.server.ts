import { checkAdmin, createMeta } from "$lib/server/utils";
import type { Actions, PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({locals}) => {
    return {
        is_admin: locals.user?.roles?.includes('admin') ?? false,
        applications: await locals.apis.applications.all(),
        groups: (await locals.apis.application_groups.all()).map(v => v.id),
        meta: createMeta(locals)
    }
};

function read_uris(entries: IterableIterator<[string, FormDataEntryValue]>): string[] {
    const uris: string[] = [];
    for (const entry of entries) {
        if (entry[0].startsWith("uri=")) {
            uris.push(entry[1] as string)
        }
    }
    return uris
}

export const actions: Actions = {
    edit: async ({locals, request}) => {
        const formData = await request.formData();
        const id = formData.get("id") as string;
        const name = formData.get(("name")) as string;
        const uris = read_uris(formData.entries());
        return await locals.apis.applications.replace(id, name, uris)
    },
    create: async ({locals, request}) => {
        const formData = await request.formData();
        const id = formData.get("id") as string;
        const name = formData.get(("name")) as string;
        const application_group = formData.get(("application_group")) as string;
        const kind = formData.get(("kind")) as string;
        const uris = read_uris(formData.entries());
        return await locals.apis.applications.create(name, application_group, kind, uris)
    },
    delete: async ({locals, request}) => {
        const formData = await request.formData();
        const id = formData.get("id") as string;
        return await locals.apis.applications.delete(id)
    },
    
};