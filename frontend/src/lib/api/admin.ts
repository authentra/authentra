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

export const ApplicationKinds = ['web-server', 'spa'];
export type ApplicationKind = typeof ApplicationKinds[number];

export interface Application {
    id: string,
    name: string,
    application_group: string,
    kind: ApplicationKind,
    client_id: string,
    redirect_uri: string[]
}

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
    delete(id: string): Promise<string[]> {
        return checkResponse(this.api.delete(`/admin/application-groups/${id}`)).then(res => res.response)
    }
    create(id: string, scopes: InternalScope[]): Promise<string[]> {
        return checkResponse(this.api.post(`/admin/application-groups`, { ...jsonBody({ id, scopes }) })).then(res => res.response)
    }
}

class ApplicationApi {
    private api: Api;
    constructor(api: Api) {
        this.api = api;
    }

    all(): Promise<Application[]> {
        return checkResponse(this.api.get('/admin/applications')).then(res => res.response)
    }

    replace(id: string, name: string, redirect_uri: string[]) {
        return checkResponse(this.api.put('/admin/applications/' + id, {
            ...jsonBody({ name, redirect_uri })
        })).then(res => res.response)
    }
    
    delete(id: string): Promise<void> {
        return checkResponse(this.api.delete(`/admin/applications/${id}`)).then(res => res.response)
    }
    create(name: string, application_group: string, kind: ApplicationKind, redirect_uri: string[]): Promise<string[]> {
        return checkResponse(this.api.post(`/admin/applications`, { ...jsonBody({ name, application_group, kind, redirect_uri}) })).then(res => res.response)
    }
}