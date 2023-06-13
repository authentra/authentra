import { handleMeta, type ClientMeta, type Meta, jsonBody } from "$lib/utils";
import { checkResponse, type Api } from ".";

export interface ClientAdminMeta extends ClientMeta {
    admin: AdminApi,
}

export function handleAdminMeta(meta: Meta): ClientAdminMeta {
    let newMeta = handleMeta(meta) as ClientAdminMeta;
    newMeta.admin = new AdminApi(newMeta.api);
    return newMeta
}
export const InternalScopes = ['profile:read', 'profile:write'] as const;
export type InternalScope = typeof InternalScopes[number];

export interface ApplicationGroup {
    id: string,
    scopes: InternalScope[]
}

export class AdminApi {
    private api: Api;

    readonly applicationGroups: ApplicationGroupApi;
    readonly application: ApplicationApi;
    constructor(api: Api) {
        this.api = api;
        this.applicationGroups = new ApplicationGroupApi(api);
        this.application = new ApplicationApi(api);
    }
}

class ApplicationGroupApi {
    private api: Api;
    constructor(api: Api) {
        this.api = api;
    }

    allGroups(): Promise<ApplicationGroup[]> {
        return checkResponse(this.api.get('/admin/application-groups')).then(res => res.response)
    }

    replace(id: string, scopes: InternalScope[]) {
        return checkResponse(this.api.put('/admin/application-groups/' + id, {
            ...jsonBody({ scopes })
        })).then(res => res.response)
    }
    
    usedBy(id: string): Promise<string[]> {
        return checkResponse(this.api.get(`/admin/application-groups/${id}/usages`)).then(res => res.response)
    }
}

class ApplicationApi {
    private api: Api;
    constructor(api: Api) {
        this.api = api;
    }
}