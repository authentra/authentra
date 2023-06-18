import { UserRoles, type UserRole } from "$lib/api/types";
import { getRolesFromForm } from "$lib/server/utils";
import { redirect } from "@sveltejs/kit";
import type { Actions, PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({params, locals}) => {
    return {
        user: locals.apis.users.get(params.id)
    }
};

export const actions: Actions = {
    edit: async ({request, locals}) => {
        const formData = await request.formData();
        const id = formData.get('id') as string;
        const name = formData.get('name') as string;
        let email = formData.get('email') as string | null;
        if (email == "") {
            email = null;
        }
        const active = formData.has('active');
        const roles = getRolesFromForm(formData);
        const customer = formData.has('customer');
        const require_password_reset = formData.has('require_password_reset');
        return await locals.apis.users.edit(id, name, email, active, roles, customer, require_password_reset);
    },
    delete: async ({params, locals}) => {
        console.log("DEleting" +params.id)
        await locals.apis.users.delete(params.id)
        throw redirect(307, '/admin/users')
    },
};