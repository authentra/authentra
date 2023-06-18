import { UserRoles, type UserRole } from "$lib/api/types";
import type { Actions, PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({params, locals}) => {
    return {
        user: locals.apis.users.get(params.id)
    }
};

function getRoles(formData: FormData): UserRole[] {
    const roles = [];
    for (const [index, value] of UserRoles.entries()) {
        const data = formData.has('role:'+value);
        if (data) {
            roles.push(value);
        }
    }
    return roles
}

export const actions: Actions = {
    edit: async ({request, locals}) => {
        const formData = await request.formData();
        const id = formData.get('id') as string;
        const name = formData.get('name') as string;
        const email = formData.get('email') as string | null;
        const active = formData.has('active');
        const roles = getRoles(formData);
        const customer = formData.has('customer');
        const require_password_reset = formData.has('require_password_reset');
        return await locals.apis.users.edit(id, name, email, active, roles, customer, require_password_reset);
    },
    delete: async ({request,locals}) => {
        const formData = await request.formData();
        const id = formData.get('id') as string;
        return await locals.apis.users.delete(id)
    }
};